use shared::{
    enums::ParsedEnum, funcs::ParsedFunc, impls::{FieldKind, ParsedImpl}, mods::{ModuleItem, ModulePath, ParsedModule}, ToTokens
};
use std::fmt::Write;
use syn::{token::Mod, Pat};

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
}

/// A luau function that contains its name, doc, return type, named [`LuaArg`] and its parent module name
pub struct LuaFunc {
    name: String,
    doc: ItemDoc,
    return_ty: LuaType,
    args: Vec<LuaArg>,
}

impl LuaFunc {
    pub fn from_parsed(parsed: ParsedFunc) -> Option<Self> {
        let name = parsed.name.to_string();
        let return_ty = LuaType::from_syn_ty(&parsed.return_ty);

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
                ty: LuaType::from_syn_ty(&arg.ty),
                optional: false, // TODO: In the future, the argument should be optional if it's of type Option<T>
            });
        }

        Some(Self {
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

    /// The same as [`LuaFunc::as_ty`], but for impl functions (class functions or methods).
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
            ", ".to_owned()
        } else {
            "".to_owned()
        };

        // TODO: Sometimes arguments can be empty, so a trailing comma can cause issues in the future.
        format!("({self_arg}{self_comma}{args}) -> {return_ty}")
    }
}

/// In luau described as both type and table
pub struct LuaStruct {
    name: String,
    doc: ItemDoc,
    fields: Vec<LuaField>,
    funcs: Vec<LuaFunc>,
    methods: Vec<LuaFunc>,
}

impl LuaStruct {
    pub fn from_parsed(parsed: ParsedImpl) -> Option<Self> {
        let name = parsed.name.to_token_stream().to_string();

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
                let fty = LuaType::from_syn_ty(&field.func.return_ty);
                fields.push(LuaField {
                    name: fname,
                    ty: fty,
                });
            }
        }

        Some(Self {
            name,
            doc: None,
            funcs,
            fields,
            methods,
        })
    }
}

impl LuaExpand for LuaStruct {
    fn lua_expand(&self, inside_parent: bool) -> (String, String) {
        let mut expanded = String::new();
        let mut global_ty = String::new();

        let name = &self.name;

        // First we expand the type

        if let Some(ref doc) = self.doc {
            writeln!(&mut global_ty, "--[[{doc}]]").unwrap();
        }

        writeln!(&mut global_ty, "export type {name} = {{").unwrap();

        for field in self.fields.iter() {
            let fname = field.name.clone();
            let fty = field.ty.clone();
            writeln!(&mut global_ty, "    {fname}: {fty},").unwrap();
        }

        for func in self.methods.iter() {
            let fname = func.name.clone();
            let fty = func.as_ty_impl(name, true);
            writeln!(&mut global_ty, "    {fname}: {fty},").unwrap();
        }

        writeln!(&mut global_ty, "}}").unwrap();

        // Now we expand the table

        if let Some(ref doc) = self.doc {
            writeln!(&mut expanded, "--[[{doc}]]").unwrap();
        }

        if inside_parent {
            writeln!(&mut expanded, "{name}: {{").unwrap();
        } else {
            writeln!(&mut expanded, "declare {name}: {{").unwrap();
        }

        for func in self.funcs.iter() {
            let fname = func.name.clone();
            let fty = func.as_ty_impl(name, false);
            writeln!(&mut expanded, "    {fname}: {fty},").unwrap();
        }

        let comma = if inside_parent { "," } else { "" };

        writeln!(&mut expanded, "}}{comma}").unwrap();

        // Now finally return

        (global_ty, expanded)
    }
}

pub struct LuaEnum {
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
            .map(|(vident, _)| LuaVariant {
                name: vident.to_string(),
            })
            .collect();

        Some(Self {
            name,
            doc: None,
            variants,
        })
    }
}

impl LuaExpand for LuaEnum {
    fn lua_expand(&self, inside_parent: bool) -> (String, String) {
        let mut expanded = String::new();

        let name = &self.name;

        // First we write the doc string to our function, if it is present
        if let Some(ref doc) = self.doc {
            writeln!(&mut expanded, "--[[{doc}]]").unwrap();
        }

        // Depending on the nesting, luau function declarations aren't the same.
        // Global functions are declared directly as function {name}({named args}): {ret type},
        // but nested functions (included in types or )
        if inside_parent {
            writeln!(&mut expanded, "{name}: {{").unwrap();
        } else {
            writeln!(&mut expanded, "declare {name}: {{").unwrap();
        }

        for var in self.variants.iter() {
            writeln!(&mut expanded, "    {}: number,", var.name).unwrap();
        }

        let comma = if inside_parent { "," } else { "" };

        writeln!(&mut expanded, "}}{comma}").unwrap();

        (String::new(), expanded)
    }
}

/// Just an item that contains module name. It doesn't own anything, lua items describe its relationship
/// with it using the [`ItemParent`] attribute.
pub struct LuaModule {
    /// If this module is the main (entrypoint) module
    ismain: bool,
    doc: ItemDoc,
    name: String,
    includes: Vec<ModulePath>,
    mods: Vec<LuaModule>,
    pub funcs: Vec<LuaFunc>,
    pub impls: Vec<LuaStruct>,
    pub enums: Vec<LuaEnum>
}

impl LuaModule {
    pub fn from_parsed(parsed: ParsedModule) -> Option<Self> {
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

        Some(Self {
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

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn is_main(&self) -> bool {
        self.ismain
    }

    /// Get the module names that this module needs
    pub fn get_included(&self) -> &Vec<ModulePath> {
        &self.includes
    }

    /// Check whether this module is of provided path.
    pub fn is(&self, path: &ModulePath) -> bool {
        self.name == path.name()
    }

    /// Insert the module, and remove its name 
    pub fn insert_module(&mut self, module: LuaModule) {
        let mod_name = &module.name;
        // Remove the name of this module from required modules
        self.includes.retain(|mod_path| {
            &mod_path.name() != mod_name
        });
        self.mods.push(module);
    }
}

impl LuaExpand for LuaModule {
    fn lua_expand(&self, inside_parent: bool) -> (String, String) {
        let mut global = String::new();
        let mut expanded = String::new();

        let name = &self.name;

        // First we write the doc string to our function, if it is present
        if let Some(ref doc) = self.doc {
            writeln!(&mut expanded, "--[[{doc}]]").unwrap();
        }

        // Depending on the nesting, luau function declarations aren't the same.
        // Global functions are declared directly as function {name}({named args}): {ret type},
        // but nested functions (included in types or )
        if inside_parent {
            writeln!(&mut expanded, "{name}: {{").unwrap();
        } else {
            writeln!(&mut expanded, "declare {name}: {{").unwrap();
        }

        for lua_impl in self.impls.iter() {
            let (child_global, child_expand) = lua_impl.lua_expand(true);
            let child_expand = add_tabs(child_expand, 1);
            write!(&mut global, "{child_global}").unwrap();
            write!(&mut expanded, "{child_expand}").unwrap();
        }

        for lua_enum in self.enums.iter() {
            let (_, child_expand) = lua_enum.lua_expand(true);
            let child_expand = add_tabs(child_expand, 1);
            write!(&mut expanded, "{child_expand}").unwrap();
        }

        for lua_func in self.funcs.iter() {
            let (_, child_expand) = lua_func.lua_expand(true);
            let child_expand = add_tabs(child_expand, 1);
            write!(&mut expanded, "{child_expand}").unwrap();
        }

        for lua_mod in self.mods.iter() {
            let (child_global, child_expand) = lua_mod.lua_expand(true);
            let child_expand = add_tabs(child_expand, 1);
            if !child_global.is_empty() {
                write!(&mut global, "{child_global}").unwrap();
            }
            write!(&mut expanded, "{child_expand}").unwrap();
        }

        let comma = if inside_parent { "," } else { "" };
        writeln!(&mut expanded, "}}{comma}").unwrap();

        (global, expanded)
    }
}

/// I'm not sure thy it's a trait, but okay - maybe for consistency.
pub trait LuaExpand {
    /// Lua expand will take a reference to self, and expand to 2 strings:
    /// 1. A global declaration (for example `export type`)
    /// 2. A nested declaration (for modules)
    fn lua_expand(&self, inside_parent: bool) -> (String, String);
}

/// Describes a lua file, which basically is similar to [`ParsedFile`], but contains
/// useful information for Lua instead.
pub struct LuaFile {
    pub mods: Vec<LuaModule>,
    pub funcs: Vec<LuaFunc>,
    pub impls: Vec<LuaStruct>,
    pub enums: Vec<LuaEnum>,
}

impl LuaExpand for LuaFunc {
    fn lua_expand(&self, inside_parent: bool) -> (String, String) {
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
        if inside_parent {
            writeln!(&mut expanded, "{name}: ({args}) -> {ret_ty},").unwrap();
        } else {
            writeln!(&mut expanded, "declare function {name}({args}): {ret_ty}").unwrap();
        }

        (String::new(), expanded)
    }
}

impl LuaFile {
    /// Write all its contents to a specified type declaration file path
    pub fn write(self, path: impl AsRef<std::path::Path>) {
        let mut src = String::new();

        for strct in self.impls {
            let (global_expanded, expanded) = strct.lua_expand(false);
            writeln!(&mut src, "{global_expanded}").unwrap();
            writeln!(&mut src, "{expanded}").unwrap();
        }

        for enm in self.enums {
            let (_, expanded) = enm.lua_expand(false);
            writeln!(&mut src, "{expanded}").unwrap();
        }

        for func in self.funcs {
            let (_, expanded) = func.lua_expand(false);
            writeln!(&mut src, "{expanded}").unwrap();
        }

        for module in self.mods {
            let (global_expanded, expanded) = module.lua_expand(false);
            if !global_expanded.is_empty() {
                writeln!(&mut src, "{global_expanded}").unwrap();
            }
            writeln!(&mut src, "{expanded}").unwrap();
        }

        std::fs::write(path, src).unwrap();
    }
}
