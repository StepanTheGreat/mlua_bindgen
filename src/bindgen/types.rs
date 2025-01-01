//! Lua types defined as structures

use shared::{
    enums::ParsedEnum, funcs::ParsedFunc, impls::{FieldKind, ParsedImpl}, mods::{ModuleItem, ModulePath, ParsedModule}, utils::{remove_lua_prefix, LastPathIdent}, ToTokens
};
use std::{collections::HashMap, fmt::Write, sync::LazyLock};
use syn::{Pat, Type};

use crate::error::Error;

use super::expand::LuaExpand;

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

impl LuaType {
    /// Stringify the ident token, then try to match it against the TYPE_MAP.
    /// 
    /// If successful - returns [`Some<Self>`]
    pub fn from_syn_ident(ident: &syn::Ident) -> Self {

        // The thinking here is that, in mlua API, only types that implment mlua::IntoLua can be used in the Lua runtime
        // There's also a default type list which already implements this trait, so taking that into the account,
        // we will guess that any types that can't be mapped is a custom Lua type.
        match TYPE_MAP.get(ident.to_string().as_str()) {
            Some(ty) => ty.clone(),
            None => LuaType::Custom(
                remove_lua_prefix(ident.to_string())
            ),
        }
    }

    /// Try to convert a syn type to a [`LuaType`]. If the type isn't recognized,
    /// it's likely it's a custom type, so we'll just make a new one.
    pub fn from_syn_ty(ty: &Type) -> Result<Self, Error> {
        let lua_ty = match ty {
            Type::Array(ty_arr) => Self::Array(Box::new(Self::from_syn_ty(&ty_arr.elem)?)),
            Type::Path(ty_path) => {
                let ident = ty_path.path.last_ident();
                Self::from_syn_ident(ident)
            }
            Type::Reference(ty_ref) => Self::from_syn_ty(&ty_ref.elem)?,
            Type::Tuple(tup) => {
                if !tup.elems.is_empty() {
                    return Err(Error::Unimplemented { message: "Multi-value tuples aren't supported currently".to_owned() });
                } else {
                    Self::Void
                }
            }
            _ => return Err(Error::Unimplemented { message: "For now only arrays and type paths are supported".to_owned() })
        };
        Ok(lua_ty)
    }
}

impl std::fmt::Display for LuaType {
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

pub type ItemDoc = Option<String>;

/// An argument for the luau function
pub struct LuaArg {
    pub name: String,
    pub ty: LuaType,
    /// Optional args can be ignored when calling a function. In rust it's declared as [`Option<T>`],
    /// while in Lua it's just `T?`
    pub optional: bool,
}

/// A field for luau structs
pub struct LuaField {
    pub name: String,
    pub ty: LuaType,
}

/// A field for luau enums
pub struct LuaVariant {
    pub name: String,
}

/// A luau function that contains its name, doc, return type, named [`LuaArg`] and its parent module name
pub struct LuaFunc {
    pub name: String,
    pub doc: ItemDoc,
    pub return_ty: LuaType,
    pub args: Vec<LuaArg>,
}

impl LuaFunc {
    pub fn from_parsed(parsed: ParsedFunc) -> Result<Self, Error> {
        let name = parsed.name.to_string();
        let name = remove_lua_prefix(name);

        let return_ty = LuaType::from_syn_ty(&parsed.return_ty)?;

        // Get the amount of required arguments by mlua. These have no use in luau declaration
        let skip_args = parsed.req_arg_count();

        let mut args = Vec::new();
        for (ind, arg) in parsed.args.iter().enumerate() {
            // If the index is smaller than the amount of required arguments - skip
            if ind < skip_args {
                continue;
            }

            let arg_name = match arg.name {
                Pat::Ident(ref pat_ident) => pat_ident.ident.to_string(),
                _ => continue,
            };
            args.push(LuaArg {
                name: arg_name,
                ty: LuaType::from_syn_ty(&arg.ty)?,
                optional: false, // TODO: In the future, the argument should be optional if it's of type Option<T>
            });
        }

        Ok(Self {
            name,
            doc: None,
            return_ty,
            args,
        })
    }

    /// Get luau formatted arguments. Instead of getting an entire function declaration, it only returns
    /// a formatted argument list like so: `number, string?, number`. (Yes, without  parantheses)
    pub fn get_fmt_args(&self) -> String {
        let mut string = String::new();

        for (ind, arg) in self.args.iter().enumerate() {
            let name = &arg.name;
            let ty = arg.ty.to_string();
            let opt = if arg.optional { "?" } else { "" };
            let comma = if ind > 0 { ", " } else { "" };
            string += &format!("{comma}{name}: {ty}{opt}");
        }
        string
    }

    /// Format self as a function type. 
    /// 
    /// It for example can be used in luau return types.
    /// Since `function` can't be used for returns, a more precise declaration is required:
    /// `(arg1, arg2, ...) -> return_type`
    pub fn as_ty(&self) -> String {
        let args = self.get_fmt_args();
        let return_ty = self.return_ty.to_string();
        format!("({args}) -> {return_ty}")
    }

    /// The same as [`LuaFunc::as_ty`], but for impl functions (class functions or methods)
    /// 
    /// It takes a type name as an argument, and will replace all `Self` keywords with the name
    /// of the type.
    pub fn as_ty_impl(&self, ty: &String, is_method: bool) -> String {
        let args = self.get_fmt_args();

        // We use this ugly and slow method for now, I'll change it in the future.
        // We have to replace all Self arguments with the name type, since Self is not recognized
        // in luau as a reference to a self type.
        let args = args.replace("Self", ty);

        let mut return_ty = self.return_ty.to_string();
        if return_ty == "Self" {
            return_ty = ty.to_string();
        }

        let self_arg = if is_method {
            format!("self: {ty}")
        } else {
            "".to_owned()
        };

        let self_comma = if is_method && !args.is_empty() { 
            ", "
        } else { 
            ""
        }.to_owned();

        // TODO: Sometimes arguments can be empty, so a trailing comma can cause issues in the future.
        format!("({self_arg}{self_comma}{args}) -> {return_ty}")
    }
}

/// In luau described as both type and table
pub struct LuaStruct {
    pub name: String,
    pub doc: ItemDoc,
    pub fields: Vec<LuaField>,
    pub funcs: Vec<LuaFunc>,
    pub methods: Vec<LuaFunc>,
}

impl LuaStruct {
    pub fn from_parsed(parsed: ParsedImpl) -> Result<Self, Error> {
        let name = parsed.name.to_token_stream().to_string();
        // Remove the lua prefix of course, if it's present
        let name = remove_lua_prefix(name);

        let mut funcs = Vec::new();
        let mut fields = Vec::new();
        let mut methods = Vec::new();

        for func in parsed.funcs {
            let lfunc = LuaFunc::from_parsed(func.func)?;
            funcs.push(lfunc);
        }

        for method in parsed.methods {
            let lmethod = LuaFunc::from_parsed(method.func)?;
            methods.push(lmethod);
        }

        for field in parsed.fields {
            if let FieldKind::Getter = field.kind {
                let fname = field.func.name.to_string();
                let fty = LuaType::from_syn_ty(&field.func.return_ty)?;
                fields.push(LuaField {
                    name: fname,
                    ty: fty,
                });
            }
        }

        Ok(Self {
            name,
            doc: None,
            funcs,
            fields,
            methods,
        })
    }
}

pub struct LuaEnum {
    pub name: String,
    pub doc: ItemDoc,
    pub variants: Vec<LuaVariant>,
}

impl LuaEnum {
    pub fn from_parsed(parsed: ParsedEnum) -> Result<Self, Error> {
        let name = parsed.ident.to_string();
        let name = remove_lua_prefix(name);

        let variants = parsed
            .variants
            .into_iter()
            .map(|(vident, _)| LuaVariant {
                name: vident.to_string(),
            })
            .collect();

        Ok(Self {
            name,
            doc: None,
            variants,
        })
    }
}

/// Just an item that contains module name. It doesn't own anything, lua items describe its relationship
/// with it using the [`ItemParent`] attribute.
pub struct LuaModule {
    /// If this module is the main (entrypoint) module
    pub ismain: bool,
    pub doc: ItemDoc,
    pub name: String,
    pub includes: Vec<ModulePath>,
    pub mods: Vec<LuaModule>,
    pub funcs: Vec<LuaFunc>,
    pub impls: Vec<LuaStruct>,
    pub enums: Vec<LuaEnum>
}

impl LuaModule {
    pub fn from_parsed(parsed: ParsedModule) -> Result<Self, Error> {
        let name = parsed.ident.to_string();

        let ismain = parsed.ismain;
        let mut funcs = Vec::new();
        let mut impls = Vec::new();
        let mut enums = Vec::new();

        for item in parsed.items {
            match item {
                ModuleItem::Fn(func) => {
                    funcs.push(LuaFunc::from_parsed(func)?);
                },
                ModuleItem::Enum(enm) => {
                    enums.push(LuaEnum::from_parsed(enm)?);
                },
                ModuleItem::Impl(imp) => {
                    impls.push(LuaStruct::from_parsed(imp)?);
                }
            }
        }        

        Ok(Self {
            ismain,
            name,
            includes: parsed.includes,
            doc: None,
            mods: Vec::new(),
            funcs,
            impls,
            enums
        })
    }

    /// Check whether this module is of provided path.
    pub fn is(&self, path: &ModulePath) -> bool {
        self.name == path.name()
    }

    /// Insert a module and remove its name from the `includes` list (i.e. requested modules) 
    pub fn insert_module(&mut self, module: LuaModule) {
        let mod_name = &module.name;
        // Remove the name of this module from required modules
        self.includes.retain(|mod_path| {
            &mod_path.name() != mod_name
        });
        self.mods.push(module);
    }
}

/// Describes a lua file, which basically is similar to [`ParsedFile`], but contains
/// useful information for Lua instead.
pub struct LuaFile<'a> {
    items: Vec<Box<dyn LuaExpand + 'a>>
}

impl<'a> LuaFile<'a> {
    pub(crate) fn new() -> Self {
        Self {
            items: Vec::new()
        }
    }

    /// Add an item that implements [LuaExpand] to the list
    pub(crate) fn add_item(&mut self, item: impl LuaExpand + 'a) {
        self.items.push(Box::new(item));
    }

    /// Add a vector of items that implement [LuaExpand]
    pub(crate) fn add_items(&mut self, items: Vec<impl LuaExpand + 'a>) {
        for item in items {
            self.add_item(item);
        }
    }

    /// Write all its contents to a provided type that implements [std::io::Write]
    /// 
    /// # Warning
    /// This will expand the source code each time from scratch
    pub fn write(&self, to: &mut impl std::io::Write) {
        to.write(self.to_string().as_bytes()).unwrap();
    }

    /// Convert the LuaFile to a Lua source string
    pub fn to_string(&self) -> String {
        let mut src = String::new();

        for item in self.items.iter() {
            let (global_expanded, inner_expanded) = item.lua_expand(false);
            
            if !global_expanded.is_empty() {
                let _ = writeln!(&mut src, "{global_expanded}");
            }

            if !inner_expanded.is_empty() {
                let _ = writeln!(&mut src, "{inner_expanded}").unwrap();
            }
        }

        src
    }
}
