//! Bindgen functionality. This is supposed to be accessed via the "bindgen" feature.

//! # Lua Bindgen
//! ## Generate Lua bindings for your Rust types!

use core::fmt;
use shared::enums::{parse_enum, ParsedEnum};
use shared::funcs::{parse_func, FuncKind, ParsedFunc};
use shared::impls::{parse_impl, ParsedImpl};
use shared::mods::{parse_mod, ParsedModule};
use shared::utils::{contains_attr, parse_attrs, ItemAttrs, LastPathIdent, MLUA_BINDGEN_ATTR};
use std::fs;
use std::sync::LazyLock;
use std::{collections::HashMap, path::PathBuf};
use syn::Item;
use syn::{Attribute, Type};
use types::{LuaEnum, LuaFile, LuaFunc, LuaModule, LuaStruct};
use utils::{find_attr, get_attribute_args};

mod types;
mod utils;

type TypeMap<'a> = HashMap<&'a str, LuaType>;

// This wasn't supposed to be threaded, but that's the simplest solution for now
static TYPE_MAP: LazyLock<TypeMap> = LazyLock::new(type_map);

fn type_map<'a>() -> TypeMap<'a> {
    // A lot of types here are ASSUMED to implement the mlua IntoLua trait.
    // The bindgen doesn't guarantee that it will, though it can try to check by first
    // running `cargo check` on the project.

    macro_rules! type_map {
        ($($val:expr => ($($key:ty),*)), * $(,)?) => {
            HashMap::from([
                $(
                    $((stringify!($key), $val)),*
                ,)*
            ])
        };
    }

    type_map! {
        LuaType::Number => (i8, i16, i32, i64, i128, isize),
        LuaType::Number => (u8, u16, u32, u64, u128, usize),
        LuaType::Number  => (f32, f64),
        LuaType::Boolean => (bool),
        LuaType::String  => (Box<str>, CString, String, OsString, PathBuf, BString),
        LuaType::Table => (HashMap, Vec, BTreeMap, Box, Table),
        LuaType::Error => (Error),
        LuaType::Thread => (Thread),
        LuaType::Userdata => (AnyUserData, LightUserData, UserDataRef, UserDataRefMut),
        LuaType::Function => (Function),
        LuaType::Void => (())
    }
}

/// Lua type enum. Doesn't neccessarily represent lua types, though mostly it does. It also contains mlua
/// specific values for edge cases.
#[derive(Debug, Clone)]
pub enum LuaType {
    Integer,
    Number,
    Boolean,
    String,
    Function,
    Array(Box<LuaType>),
    Error,
    Table,
    Thread,
    Userdata,
    Nil,
    /// Void represents an absence of return type `()` (in both Luau and Rust)
    Void,
    /// Custom types are new types defined by the user of mlua itself.
    /// These are passed directly as string, as they're simply a reference to a
    /// defined type (i.e. through [`mlua_bindgen`] macro)
    Custom(String),
}

impl fmt::Display for LuaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                LuaType::Integer => "integer".to_owned(),
                LuaType::Number => "number".to_owned(),
                LuaType::Boolean => "boolean".to_owned(),
                LuaType::String => "string".to_owned(),
                LuaType::Function => "function".to_owned(),
                LuaType::Array(ty) => format!("{{{}}}", ty),
                LuaType::Error => "error".to_owned(),
                LuaType::Table => "table".to_owned(),
                LuaType::Thread => "thread".to_owned(),
                LuaType::Userdata => "userdata".to_owned(),
                LuaType::Nil => "nil".to_owned(),
                LuaType::Void => "()".to_owned(),
                LuaType::Custom(ty) => ty.clone(),
            }
        )
    }
}

impl LuaType {
    /// Stringify the ident token, then try to match it against the TYPE_MAP.
    /// If successful - returns [`Some<Self>`]
    pub fn from_syn_ident(ident: &syn::Ident) -> Self {
        match TYPE_MAP.get(ident.to_string().as_str()) {
            Some(ty) => ty.clone(),
            None => LuaType::Custom(ident.to_string()),
        }
    }

    /// Try to convert a syn type to a [`LuaType`]. If the type isn't recognized,
    /// it's likely it's a custom type, so we'll just make a new one.
    pub fn from_syn_ty(ty: &Type) -> Self {
        match ty {
            Type::Array(ty_arr) => Self::Array(Box::new(Self::from_syn_ty(&ty_arr.elem))),
            Type::Path(ty_path) => {
                let ident = ty_path.path.last_ident();
                Self::from_syn_ident(ident)
            }
            Type::Reference(ty_ref) => Self::from_syn_ty(&ty_ref.elem),
            Type::Tuple(tup) => {
                if !tup.elems.is_empty() {
                    unimplemented!("Multi-value tuples aren't supported currently");
                } else {
                    Self::Void
                }
            }
            _ => unimplemented!("For now only arrays and type paths are supported"),
        }
    }
}

/// A collection of all mlua_bindgen items in a single structure
pub struct ParsedFile {
    pub mods: Vec<ParsedModule>,
}

impl ParsedFile {
    pub fn transform_to_lua(self) -> Option<LuaFile> {
        let mut mods = Vec::new();
        let mut impls = Vec::new();
        let mut enums = Vec::new();
        let mut funcs = Vec::new();
    
        // Here we parse and collect all lua modules. 
        // We also check if any of them is main, and if so - we add it to a separate main_mod variable,
        // which will be important for us later
        let mut main_mod: Option<LuaModule> = None;
        let mut mod_map = HashMap::new();
        for parsed_mod in self.mods {
            let lua_mod = LuaModule::from_parsed(parsed_mod)?;
            if lua_mod.is_main() {
                if main_mod.is_some() {
                    // There are 2 main modules? This is a guaranteed panic!
                    panic!("Found more than 2 main modules. Only 1 can be present at the same time");
                } else {
                    main_mod = Some(lua_mod);
                }
            } else {
                mod_map.insert(lua_mod.name(), lua_mod);
            }
        }
        println!("Got modules: {:}", mod_map.len());

        let mut main_mod = main_mod.expect("Bindgen requires a main module to be present");

        // Now we need to insert modules appropriately, starting from the main module
        loop {
            // A tuple of parent module, and a list of modules to be inserted into them
            let mut insertions: HashMap<String, Vec<String>> = HashMap::new();
            for (mod_name, module) in mod_map.iter() {
                let needs = module.get_included();
                if needs.is_empty() { continue };

                // Iterate over all module paths this module requires
                for needed_path in needs {
                    // Check whether it exists in the map
                    if let Some(needed_mod) = mod_map.get(&needed_path.name()) {
                        // Also make sure that the found module doesn't include anything. If it does - 
                        // we need to resolve that module first.
                        if needed_mod.get_included().is_empty() {
                            if insertions.contains_key(mod_name) {
                                insertions.get_mut(mod_name).unwrap().push(needed_mod.name());
                            } else {
                                insertions.insert(mod_name.clone(),  vec![needed_mod.name()]);
                            }
                        }
                    }
                }
            } 

            if insertions.is_empty() {
                // No more insertions to make, the modules are prepared
                break
            } else {
                // Iterate over all insertions, and insert them into their appropriate modules
                for (mod_name, to_insert) in insertions.into_iter() {
                    for item in to_insert {
                        let removed = mod_map.remove(&item).unwrap();
                        mod_map.get_mut(&mod_name).unwrap().insert_module(removed);
                    }
                }
            }
        }
        // Now we just need to insert all collected modules into the main module
        let main_mod_included = main_mod.get_included();
        for module in mod_map.into_values() {
            let contains = main_mod_included.iter()
                .find(|path| module.is(path))
                .is_some();

            if contains {
                mods.push(module)    
            }
        }

        // Now we remove all the main module items into the main file instead
        for lua_enum in main_mod.enums.drain(..) {
            enums.push(lua_enum);
        }

        for lua_impl in main_mod.impls.drain(..) {
            impls.push(lua_impl);
        }

        for lua_func in main_mod.funcs.drain(..) {
            funcs.push(lua_func);
        }

        println!("Mods: {}", mods.len());
        println!("Funcs: {}", funcs.len());
        println!("Impls: {}", impls.len());
        println!("Enums: {}", enums.len());

        Some(LuaFile {
            mods,
            funcs,
            impls,
            enums,
        })
    }
}

/// Find an [`MLUA_BINDGEN_ATTR`] argument, then convert it into tokens,
/// then parse it into [`ItemAttrs`]. It can fail at any stage and return [`None`]
fn get_bindgen_attrs(item_attrs: &[Attribute]) -> Option<ItemAttrs> {
    let attr = find_attr(item_attrs, MLUA_BINDGEN_ATTR)?;

    match get_attribute_args(attr) {
        Some(attr_tokens) => {
            match parse_attrs(attr_tokens) {
                Ok(attrs) => Some(attrs),
                Err(_err) => None
            }
        },
        None => Some(ItemAttrs::new_empty())
    }
}

/// Parsed a file.
fn parse_file(file: syn::File) -> syn::Result<ParsedFile> {
    let mut mods: Vec<ParsedModule> = Vec::new();

    for item in file.items {
        if let Item::Mod(mod_item) = item {
            if let Some(attrs) = get_bindgen_attrs(&mod_item.attrs) {
                mods.push(parse_mod(attrs, mod_item, true)?);
            }
        }
    }

    Ok(ParsedFile { mods })
}

pub fn load_file(path: impl Into<PathBuf>) -> syn::Result<ParsedFile> {
    let src = fs::read_to_string(path.into()).unwrap();
    let file = syn::parse_file(&src)?;
    parse_file(file)
}
