//! Bindgen functionality. This is supposed to be accessed via the "bindgen" feature.

//! # Lua Bindgen
//! ## Generate Lua bindings for your Rust types!

use core::fmt;
use shared::enums::{parse_enum, ParsedEnum};
use shared::funcs::{parse_func, FuncKind, ParsedFunc};
use shared::impls::{parse_impl, ParsedImpl};
use shared::mods::{parse_mod, ModuleItem, ParsedModule};
use shared::utils::{
    contains_attr, parse_attrs, syn_error, ItemAttrs, LastPathIdent, MLUA_BINDGEN_ATTR,
};
use std::fs;
use std::sync::LazyLock;
use std::{collections::HashMap, path::PathBuf};
use syn::{Attribute, Type};
use syn::{Item, ItemFn, ItemImpl};
use types::{LuaEnum, LuaFile, LuaFunc};
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
        LuaType::Function => (Function)
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
            }
        )
    }
}

impl LuaType {
    /// Stringify the ident token, then try to match it against the TYPE_MAP.
    /// If successful - returns [`Some<Self>`]
    pub fn from_syn_ident(ident: &syn::Ident) -> Option<Self> {
        TYPE_MAP.get(ident.to_string().as_str()).cloned()
    }

    pub fn from_syn_ty(ty: &Type) -> Option<Self> {
        match ty {
            Type::Array(ty_arr) => {
                Self::from_syn_ty(&ty_arr.elem).map(|inner| Self::Array(Box::new(inner)))
            }
            Type::Path(ty_path) => {
                let ident = ty_path.path.last_ident();
                Self::from_syn_ident(ident)
            }
            _ => unimplemented!("For now only arrays and type paths are supported"),
        }
    }
}

/// A collection of all mlua_bindgen items in a single structure
pub struct ParsedFile {
    pub mods: Vec<ParsedModule>,
    pub funcs: Vec<ParsedFunc>,
    pub impls: Vec<ParsedImpl>,
    pub enums: Vec<ParsedEnum>,
}

impl ParsedFile {
    pub fn transform_to_lua(self) -> Option<LuaFile> {
        let mods = Vec::new();
        let impls = Vec::new();
        let mut enums = Vec::new();
        let mut funcs = Vec::new();

        for enm in self.enums {
            enums.push(LuaEnum::from_parsed(enm)?);
        }

        for func in self.funcs {
            funcs.push(LuaFunc::from_parsed(func)?);
        }

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

    let attr_tokens = get_attribute_args(attr)?;

    match parse_attrs(attr_tokens) {
        Ok(attrs) => Some(attrs),
        Err(err) => None,
    }
}

/// Parsed a file.
fn parse_file(file: syn::File) -> syn::Result<ParsedFile> {
    let mut mods: Vec<ParsedModule> = Vec::new();
    let mut funcs: Vec<ParsedFunc> = Vec::new();
    let mut impls: Vec<ParsedImpl> = Vec::new();
    let mut enums: Vec<ParsedEnum> = Vec::new();

    for item in file.items {
        match item {
            Item::Mod(mod_item) => {
                if let Some(attrs) = get_bindgen_attrs(&mod_item.attrs) {
                    mods.push(parse_mod(attrs, mod_item)?);
                }
            }
            Item::Fn(fn_item) => {
                if contains_attr(&fn_item.attrs, MLUA_BINDGEN_ATTR) {
                    funcs.push(parse_func(fn_item, &FuncKind::Func)?);
                }
            }
            Item::Impl(impl_item) => {
                if contains_attr(&impl_item.attrs, MLUA_BINDGEN_ATTR) {
                    impls.push(parse_impl(impl_item)?);
                }
            }
            Item::Enum(enum_item) => {
                if contains_attr(&enum_item.attrs, MLUA_BINDGEN_ATTR) {
                    enums.push(parse_enum(enum_item)?);
                }
            }
            _ => continue,
        };
    }

    Ok(ParsedFile {
        mods,
        impls,
        funcs,
        enums,
    })
}

pub fn load_file(path: impl Into<PathBuf>) -> syn::Result<ParsedFile> {
    let src = fs::read_to_string(path.into()).unwrap();
    let file = syn::parse_file(&src)?;

    // let (funcs, structs) = extract_items(file);
    parse_file(file)
}
