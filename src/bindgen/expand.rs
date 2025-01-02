//! Everything related to expanding (i.e. transforming rust structures into luau source code strings)

use super::{
    types::{LuaEnum, LuaFunc, LuaModule, LuaStruct},
    utils::add_tabs,
};
use std::fmt::Write;

/// I'm not sure thy it's a trait, but okay - maybe for consistency.
pub trait LuaExpand {
    /// Lua expand will take a reference to self, and expand to 2 strings:
    /// 1. A global declaration (for example `export type`)
    /// 2. A nested declaration (for modules)
    fn lua_expand(&self, inside_parent: bool) -> (String, String);
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
