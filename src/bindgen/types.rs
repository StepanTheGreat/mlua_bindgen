//! Lua types defined as structures

use shared::{
    enums::ParsedEnum,
    funcs::ParsedFunc,
    impls::{FieldKind, ParsedImpl},
    mods::{ModuleItem, ModulePath, ParsedModule},
    utils::{remove_lua_prefix, LastPathIdent},
    ToTokens,
};
use std::{
    collections::HashMap,
    fmt::{Debug, Write},
    sync::LazyLock,
};
use syn::{GenericArgument, Pat, PathArguments, Type};

use crate::error::Error;

use super::expand::LuaExpand;
use super::USERDATA_CHAR;

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
        LuaType::Void => (()),
        LuaType::Any => (Value)
    }
}

/// Lua type enum. Doesn't neccessarily represent lua types, though mostly it does. It also contains mlua
/// specific values for edge cases.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum LuaType {
    Integer,
    Number,
    Boolean,
    String,
    Function,
    /// Can only be used to declare optional arguments (i.e. "type?")
    ///
    /// In mlua, this type is made using `Option<T>`, where `T` implements [IntoLua].
    ///
    /// Recursive optionals aren't supported, though I'm sure there are tricks to make it valid
    Optional(Box<LuaType>),
    Array(Box<LuaType>),
    /// A union type that can represent 2 times at the same time. In luau/teal it's `A | B`
    Either((Box<LuaType>, Box<LuaType>)),
    /// Tuples can only be used as return types
    Tuple(Vec<LuaType>),
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
    /// Any type in lua. Only works if you use [`Value`] in your arguments
    Any,
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
            None => LuaType::Custom(remove_lua_prefix(ident.to_string())),
        }
    }

    /// Try to convert a syn type to a [`LuaType`]. If the type isn't recognized,
    /// it's likely it's a custom type, so we'll just make a new one.
    pub fn from_syn_ty(ty: &Type) -> Result<Self, Error> {
        let lua_ty = match ty {
            Type::Array(ty_arr) => Self::Array(Box::new(Self::from_syn_ty(&ty_arr.elem)?)),
            Type::Path(ty_path) => {
                let ident = ty_path.path.last_ident();
                // For Options we have a slightly different procedure.
                if ident == "Option" {
                    let inner_ty = Self::from_syn_ty(parse_inner_ty(ty_path)?)?;

                    if let Self::Optional(_) = inner_ty {
                        return Err(Error::ParseErr { message: "Lua optional types can't be recursive (i.e. contain an Option inside an Option)".to_owned() });
                    }

                    Self::Optional(Box::new(inner_ty))
                } else if ident == "Vec" {
                    let inner_ty = Self::from_syn_ty(parse_inner_ty(ty_path)?)?;
                    Self::Array(Box::new(inner_ty))
                } else if ident == "Either" {
                    let inner_tys = parse_inner_tys(ty_path)?;

                    if inner_tys.len() != 2 {
                        return Err(Error::ParseErr { message: "Lua union types have to contain exactly 2 generic arguments".to_owned() });
                    }

                    let left = Self::from_syn_ty(inner_tys[0])?;
                    let right = Self::from_syn_ty(inner_tys[1])?;
                    Self::Either((
                        Box::new(left), Box::new(right)
                    ))
                } else {
                    Self::from_syn_ident(ident)
                }
            }
            Type::Reference(ty_ref) => Self::from_syn_ty(&ty_ref.elem)?,
            Type::Tuple(tup) => {
                if !tup.elems.is_empty() {
                    let mut tys: Vec<LuaType> = Vec::new();
                    for ty in tup.elems.iter() {
                        tys.push(LuaType::from_syn_ty(ty)?);
                    }
                    Self::Tuple(tys)
                    // return Err(Error::Unimplemented { message: "Multi-value tuples aren't supported currently".to_owned() });
                } else {
                    Self::Void
                }
            }
            _ => {
                return Err(Error::Unimplemented {
                    message: "For now only arrays and type paths are supported".to_owned(),
                })
            }
        };
        Ok(lua_ty)
    }

    /// Check whether this type is optional
    pub fn is_optional(&self) -> bool {
        match self {
            Self::Optional(_) => true,
            _ => false,
        }
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
                // "function" can't be used in declaration files, the same thing as with the table
                LuaType::Function => "(any): any".to_owned(),
                LuaType::Array(ty) => format!("{{{ty}}}"),
                // Depending on the context, optionals are declared differently (in fields/args with `T?`)
                // while in return types with `T | nil`. We leave this to the individual types
                LuaType::Optional(ty) => ty.to_string(),
                LuaType::Error => "error".to_owned(),
                // "table" isn't acceptable in declaration files, so we use this any syntax
                LuaType::Table => "{any: any}".to_owned(),
                LuaType::Thread => "thread".to_owned(),
                LuaType::Userdata => "userdata".to_owned(),
                LuaType::Nil => "nil".to_owned(),
                LuaType::Void => "".to_owned(),
                LuaType::Either((left, right)) => format!("{left} | {right}"),
                LuaType::Custom(ty) => format!("{USERDATA_CHAR}{}", ty.clone()),
                LuaType::Tuple(tys) => {
                    if tys.len() == 1 {
                        format!("({})", tys[0])
                    } else {
                        let mut result = String::new();
                        for (ind, ty) in tys.iter().enumerate() {
                            if ind == 0 {
                                result.push_str(&ty.to_string());
                            } else {
                                result.push_str(&(", ".to_owned() + &ty.to_string()));
                            }
                        }

                        format!("({result})")
                    }
                }
                LuaType::Any => "any".to_owned(),
            }
        )
    }
}

/// The same as [parse_inner_tys], but only for single items
fn parse_inner_ty(input: &syn::TypePath) -> Result<&Type, Error> {
    parse_inner_tys(input).map(|res| res[0])
}

/// Try parse a generic type with generic arguments. 
/// 
/// Will return a vector of referenced types 
/// 
/// This function can fail if the generic arguments are incorrect
fn parse_inner_tys(input: &syn::TypePath) -> Result<Vec<&Type>, Error> {
    let segment = input.path.segments.last().unwrap();
    if let PathArguments::AngleBracketed(args) = &segment.arguments {
        let tys: Vec<&Type> = args.args.iter().map(|arg| {
            match arg {
                GenericArgument::Type(ref ty) => Ok(ty),
                _ => {
                    Err(Error::Unimplemented {
                        message: "mlua_bindgen only supports types with generic type arguments".to_owned(),
                    })
                }
            }
        }).collect::<Result<Vec<&Type>, Error>>()?;

        Ok(tys)
    } else {
        Err(Error::ParseErr {
            message: "Failed to parse lua type's brackets. Expected angular brackets"
                .to_owned(),
        })
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

impl std::fmt::Display for LuaArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let optional = if self.ty.is_optional() { "?" } else { "" };
        let name = &self.name;
        let ty = &self.ty;
        write!(f, "{name}{optional}: {ty}")
    }
}

/// A return type for a luau function
///
/// This only exists to simplify working with functions that return Option<T>
pub struct LuaReturn {
    pub ty: LuaType,
    pub optional: bool,
}

impl std::fmt::Display for LuaReturn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let LuaType::Void = self.ty {
            write!(f, "")
        } else {
            let optional = if self.ty.is_optional() { " | nil" } else { "" };
            let ty = &self.ty;
            write!(f, ": {ty}{optional}")
        }
    }
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
    pub return_ty: LuaReturn,
    pub args: Vec<LuaArg>,
}

impl LuaFunc {
    pub fn from_parsed(parsed: ParsedFunc) -> Result<Self, Error> {
        let name = parsed.name.to_string();
        let name = remove_lua_prefix(name);

        let return_ty = {
            let ty = LuaType::from_syn_ty(&parsed.return_ty)?;
            let optional = ty.is_optional();

            LuaReturn { ty, optional }
        };

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

            let arg_ty = LuaType::from_syn_ty(&arg.ty)?;
            let optional = arg_ty.is_optional();
            args.push(LuaArg {
                name: arg_name,
                ty: arg_ty,
                optional,
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
            let comma = if ind > 0 { ", " } else { "" };

            string += &format!("{comma}{arg}");
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
        let return_ty = &self.return_ty;
        format!("function({args}){return_ty}")
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
        let return_ty = self.return_ty.to_string().replace("Self", ty);

        let self_arg = if is_method {
            // format!("self: {ty}")
            "self".to_owned()
        } else {
            "".to_owned()
        };

        let self_comma = if is_method && !args.is_empty() {
            ", "
        } else {
            ""
        }
        .to_owned();

        // TODO: Sometimes arguments can be empty, so a trailing comma can cause issues in the future.
        format!("function({self_arg}{self_comma}{args}){return_ty}")
    }
}

/// In luau described as both type and table
pub struct LuaStruct {
    pub name: String,
    pub doc: ItemDoc,
    pub fields: Vec<LuaField>,
    pub funcs: Vec<LuaFunc>,
    pub methods: Vec<LuaFunc>,
    pub meta_funcs: Vec<LuaFunc>,
}

impl LuaStruct {
    pub fn from_parsed(parsed: ParsedImpl) -> Result<Self, Error> {
        let name = parsed.name.to_token_stream().to_string();
        // Remove the lua prefix of course, if it's present
        let name = remove_lua_prefix(name);

        let mut funcs = Vec::new();
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        let mut meta_funcs = Vec::new();

        for func in parsed.funcs {
            let lfunc = LuaFunc::from_parsed(func.func)?;
            funcs.push(lfunc);
        }

        for method in parsed.methods {
            let lmethod = LuaFunc::from_parsed(method.func)?;
            methods.push(lmethod);
        }

        // We're pushing meta functions to the same vector, since they're by type the same as methods,
        // the sole difference being their arguments.
        for meta_func in parsed.meta_funcs {
            let lmeta_func = LuaFunc::from_parsed(meta_func.func)?;
            meta_funcs.push(lmeta_func);
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
            meta_funcs,
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
    pub enums: Vec<LuaEnum>,
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
                }
                ModuleItem::Enum(enm) => {
                    enums.push(LuaEnum::from_parsed(enm)?);
                }
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
            enums,
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
        self.includes
            .retain(|mod_path| &mod_path.name() != mod_name);
        self.mods.push(module);
    }
}

/// Describes a lua file, which basically is similar to [`ParsedFile`], but contains
/// useful information for Lua instead.
pub struct LuaFile<'a> {
    items: Vec<Box<dyn LuaExpand + 'a>>,
}

impl<'a> LuaFile<'a> {
    pub(crate) fn new() -> Self {
        Self { items: Vec::new() }
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

#[cfg(test)]
mod test {
    use super::{Error, LuaType};

    #[test]
    fn lua_types() {
        let array = LuaType::Array(Box::new(LuaType::Boolean));
        assert_eq!(array.to_string(), "{boolean}".to_owned());

        let tuple = LuaType::Tuple(vec![
            LuaType::Boolean,
            LuaType::Custom("MyType".to_owned()),
            LuaType::Number,
            LuaType::Nil,
        ]);

        assert_eq!(
            tuple.to_string(),
            "(boolean, uMyType, number, nil)".to_owned()
        );

        let single_tuple = LuaType::Tuple(vec![LuaType::String]);

        assert_eq!(single_tuple.to_string(), "(string)".to_owned());
    }

    #[test]
    fn into_lua_types() -> Result<(), Error> {
        let optional = LuaType::from_syn_ty(&syn::parse_str("Option<u32>")?)?;

        // Here we match if the Lua type is an Option itself, and if it is, we also check if its inner
        // type is a number
        assert!(matches!(optional, LuaType::Optional(inner) if matches!(*inner, LuaType::Number)));

        // The same check for the array as well
        let array = LuaType::from_syn_ty(&syn::parse_str("[String; 12]")?)?;
        assert!(matches!(array, LuaType::Array(inner) if matches!(*inner, LuaType::String)));

        Ok(())
    }

    #[test]
    fn fail_into_lua_types() -> Result<(), Error> {
        let recursive = LuaType::from_syn_ty(&syn::parse_str("Option<Option<u32>>")?);

        assert!(
            recursive.is_err(),
            "Recursive option types should fail to parse"
        );
        Ok(())
    }
}
