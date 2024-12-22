use shared::{
    enums::{LuaVariantType, ParsedEnum},
    funcs::ParsedFunc,
};
use std::fmt::Write;
use syn::Pat;

use super::{utils::add_tabs, LuaType};

/// Describes an item's parent. [`None`] means it's a global
pub type ItemParent = Option<String>;
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
    pub value: LuaVariantType,
}

/// A luau function that contains its name, doc, return type, named [`LuaArg`] and its parent module name
pub struct LuaFunc {
    parent: ItemParent,
    name: String,
    doc: ItemDoc,
    return_ty: LuaType,
    args: Vec<LuaArg>,
}

impl LuaFunc {
    pub fn from_parsed(parsed: ParsedFunc) -> Option<Self> {
        let name = parsed.name.to_string();
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

        Some(Self {
            parent: None,
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

    /// Format self as a function type. It's for example can be used in luau in return types.
    /// Since `function` can't be used for returns, a more precise declaration is required:
    /// `(arg1, arg2, ...) -> return_type`
    pub fn as_ty(&self) -> String {
        let args = self.get_fmt_args();
        let return_ty = self.return_ty.to_string();
        format!("({args}) -> {return_ty}")
    }
}

/// In luau described as both type and table
pub struct LuaStruct {
    parent: ItemParent,
    name: String,
    doc: ItemDoc,
    fields: Vec<LuaField>,
    funcs: Vec<LuaFunc>,
    methods: Vec<LuaFunc>,
}

pub struct LuaEnum {
    parent: ItemParent,
    name: String,
    doc: ItemDoc,
    variants: Vec<LuaVariant>,
}

impl LuaEnum {
    pub fn from_parsed(parsed: ParsedEnum) -> Option<Self> {
        let name = parsed.ident.to_string();

        let variants = parsed
            .variants
            .into_iter()
            .map(|(vident, value)| LuaVariant {
                name: vident.to_string(),
                value,
            })
            .collect();

        Some(Self {
            parent: None,
            name,
            doc: None,
            variants,
        })
    }
}

impl LuaExpand for LuaEnum {
    fn lua_expand(&self) -> String {
        let mut expanded = String::new();

        let name = &self.name;

        // First we write the doc string to our function, if it is present
        if let Some(ref doc) = self.doc {
            writeln!(&mut expanded, "--[[{doc}]]").unwrap();
        }

        // Depending on the nesting, luau function declarations aren't the same.
        // Global functions are declared directly as function {name}({named args}): {ret type},
        // but nested functions (included in types or )
        if self.parent.is_some() {
            writeln!(&mut expanded, "{name}: {{").unwrap();
        } else {
            writeln!(&mut expanded, "declare {name}: {{").unwrap();
        }

        for var in self.variants.iter() {
            writeln!(&mut expanded, "    {}: number,", var.name).unwrap();
        }
        writeln!(&mut expanded, "}}").unwrap();

        expanded
    }
}

/// Just an item that contains module name. It doesn't own anything, lua items describe its relationship
/// with it using the [`ItemParent`] attribute.
pub struct LuaModule {
    parent: ItemParent,
    doc: ItemDoc,
    name: String,
}

pub trait LuaItem {
    /// Get item's name
    fn name(&self) -> &str;

    /// Get item's parent (If it is present)
    fn parent(&self) -> ItemParent;
}

pub trait LuaExpand {
    fn lua_expand(&self) -> String;
}

/// Describes a lua file, which basically is similar to [`ParsedFile`], but contains
/// useful information for Lua instead.
pub struct LuaFile {
    pub mods: Vec<LuaModule>,
    pub funcs: Vec<LuaFunc>,
    pub impls: Vec<LuaType>,
    pub enums: Vec<LuaEnum>,
}

impl LuaExpand for LuaFunc {
    fn lua_expand(&self) -> String {
        let mut expanded = String::new();

        let name = &self.name;
        let ret_ty = &self.return_ty.to_string();
        let args = self.get_fmt_args();

        // First we write the doc string to our function, if it is present
        if let Some(ref doc) = self.doc {
            writeln!(&mut expanded, "--[[{doc}]]").unwrap();
        }

        // Depending on the nesting, luau function declarations aren't the same.
        // Global functions are declared directly as function {name}({named args}): {ret type},
        // but nested functions (included in types or )
        if self.parent.is_some() {
            writeln!(&mut expanded, "{name}: ({args}) -> {ret_ty}").unwrap();
        } else {
            writeln!(&mut expanded, "declare function {name}({args}): {ret_ty}").unwrap();
        }

        expanded
    }
}

impl LuaFile {
    /// Write all its contents to a specified type declaration file path
    pub fn write(self, path: impl AsRef<std::path::Path>) {
        let mut src = String::new();

        for enm in self.enums {
            let expanded = enm.lua_expand();
            writeln!(&mut src, "{}", expanded).unwrap();
        }

        for func in self.funcs {
            let expanded = func.lua_expand();
            writeln!(&mut src, "{}", expanded).unwrap();
        }

        std::fs::write(path, src).unwrap();
    }
}
