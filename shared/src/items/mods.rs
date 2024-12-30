use std::collections::HashSet;

use syn::{Ident, Item, ItemMod, Path, Visibility};

use crate::utils::{
    contains_attr, ToIdent, syn_error, ItemAttribute, ItemAttributes, LastPathIdent, MLUA_BINDGEN_ATTR
};

use super::{enums::{parse_enum, ParsedEnum}, funcs::{parse_func, FuncKind, ParsedFunc}, impls::{parse_impl, ParsedImpl}};

pub const MODULE_SUFFIX: &str = "_module";

/// This should basically include all possible items that can be placed
/// inside modules (beside modules of course)
pub enum ModuleItem {
    Fn(ParsedFunc),
    Enum(ParsedEnum),
    Impl(ParsedImpl),
}

/// Basically a path, but for modules. It simplifies frefix management and other stuff
pub struct ModulePath {
    name: String,
    pub path: Path,
}

impl ModulePath {
    /// Try to construct the module path from a path. It will fail if the module path doesn't contain the
    /// module suffix at the end.
    pub fn from_path(path: Path) -> syn::Result<Self> {
        let ident = path.last_ident();
        let name = ident.to_string();

        let split_pos = match name.rfind(MODULE_SUFFIX) {
            Some(pos) => pos,
            None => {
                return Err(syn_error(
                    path,
                    format!("Included modules have to end with the \"{MODULE_SUFFIX}\" keyword"),
                ))
            }
        };
        let (real_name, _) = name.split_at(split_pos);

        Ok(Self {
            path,
            name: real_name.to_string(),
        })
    }

    /// This will get the identity of the real name
    pub fn get_ident(&self) -> Ident {
        self.name.as_str().to_ident()
    }

    /// Returns module's name without the prefix
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Returns a full name, including the prefix
    pub fn name_prefixed(&self) -> String {
        self.name.clone() + MODULE_SUFFIX
    }
}

/// A parsed module should contain:
/// - It's identity (name)
/// - Other modules it includes (without the "_module" prefix)
/// - Its inner items (functions, enums, impls)
pub struct ParsedModule {
    pub ident: Ident,
    pub ismain: bool,
    pub visibility: Visibility,
    pub includes: Vec<ModulePath>,
    pub items: Vec<ModuleItem>,
}

/// Try parse an ItemMod into a ParsedModule.
/// 
/// `parse_items` tells whether to parse module items entirely, or just create dummy
/// items from their ident/types. This is just to avoid useless parsing when macro expands
pub fn parse_mod(attrs: ItemAttributes, item: ItemMod, parse_items: bool) -> syn::Result<ParsedModule> {
    let ident = item.ident;
    let mut ismain = false;
    let visibility = item.vis;
    let mut items: Vec<ModuleItem> = Vec::new();
    let mut includes: Vec<ModulePath> = Vec::new();

    let mut included = Vec::new();
    // Iterate over all attributes in the list.
    // Here we care only about 2 attributes: Includes and IsMain
    for attr in attrs.0 {
        match attr {
            ItemAttribute::Includes(paths) => {
                included = paths
            },
            ItemAttribute::IsMain => {
                ismain = true
            },
            ItemAttribute::Preserve => {}
        }
    }

    // To avoid stupidity, we will not accept repeated modules
    let mut already_added: HashSet<String> = HashSet::new();
    for fn_path in included {
        // let path = fn_path; // Original path, what we need
        let mod_path = ModulePath::from_path(fn_path.clone())?;
        let mod_name = mod_path.name();

        // Yep, give him an error. That's silly
        if !already_added.contains(&mod_name) {
            already_added.insert(mod_name);
        } else {
            return Err(syn_error(fn_path, "Modules can't be repeatedly added"));
        }
        includes.push(mod_path);
    }

    // Now we iterate actual module items, and if they contain the [`MLUA_BINDGEN_ATTR`] attribute -
    // we add their module registration code to the exports.
    if let Some((_, mod_items)) = item.content {
        // TODO: Parsing module items for macros is expensive and useless. 
        // TODO: Make a simplified version, where only the name get passed.
        for mod_item in mod_items {
            let new_item = match mod_item {
                Item::Fn(mod_fn) => {
                    if !contains_attr(&mod_fn.attrs, MLUA_BINDGEN_ATTR) {
                        continue;
                    }

                    ModuleItem::Fn(
                        if parse_items { 
                            parse_func(mod_fn, &FuncKind::Func)? 
                        } else {
                            ParsedFunc::from_ident(mod_fn.sig.ident)
                        }
                    )
                },
                Item::Enum(mod_enum) => {
                    if !contains_attr(&mod_enum.attrs, MLUA_BINDGEN_ATTR) {
                        continue;
                    }

                    ModuleItem::Enum(
                        if parse_items { 
                            parse_enum(mod_enum)?
                        } else {
                            ParsedEnum::from_ident(mod_enum.ident)
                        }
                    )
                },
                Item::Impl(mod_impl) => {
                    if !contains_attr(&mod_impl.attrs, MLUA_BINDGEN_ATTR) {
                        continue;
                    }

                    ModuleItem::Impl(
                        if parse_items {
                            parse_impl(mod_impl)?
                        } else {
                            ParsedImpl::from_ty(*mod_impl.self_ty)
                        }
                    )
                },
                Item::Mod(mod_mod) => return Err(syn_error(
                    mod_mod,
                    "Can't implement recursive modules. You should combine them separately for now",
                )),
                _ => continue,
            };

            items.push(new_item);
        }
    };

    Ok(ParsedModule {
        ismain,
        ident,
        visibility,
        includes,
        items,
    })
}