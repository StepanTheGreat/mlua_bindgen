use std::collections::HashSet;

use syn::{Ident, Item, ItemMod, Path, Type, Visibility};

use crate::utils::{contains_attr, str_to_ident, syn_error, ItemAttrs, MLUA_BINDGEN_ATTR};

pub const MODULE_SUFFIX: &str = "_module";

/// This should basically include all possible items that can be placed
/// inside modules (beside modules of course)
pub enum ModuleItem {
    Fn(Ident),
    Enum(Ident),
    Impl(Type)
}

/// Basically a path, but for modules. It simplifies frefix management and other stuff
pub struct ModulePath {
    name: String,
    pub path: Path
}

impl ModulePath {
    /// Try to construct the module path from a path. It will fail if the module path doesn't contain the
    /// module suffix at the end.
    pub fn from_path(path: Path) -> syn::Result<Self> {
        let ident = path.segments.last().map(|seg|  &seg.ident).unwrap();
        let name = ident.to_string();

        let split_pos = match name.rfind(MODULE_SUFFIX) {
            Some(pos) => pos,
            None => return Err(syn_error(
                path, 
                format!("Included modules have to end with the \"{MODULE_SUFFIX}\" keyword")
            ))
        };
        let (real_name, _) = name.split_at(split_pos);

        Ok(Self {
            path,
            name: real_name.to_string()
        })
    }

    /// This will get the identity of the real name
    pub fn get_ident(&self) -> Ident {
        str_to_ident(&self.name)
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
    pub visibility: Visibility,
    pub includes: Vec<ModulePath>,
    pub items: Vec<ModuleItem>
}

pub fn parse_mod(attrs: ItemAttrs, item: ItemMod) -> syn::Result<ParsedModule> {
    let ident = item.ident;
    let visibility = item.vis;
    let mut items: Vec<ModuleItem> = Vec::new();
    let mut includes: Vec<ModulePath> = Vec::new();

    let included = match attrs {
        ItemAttrs::Empty => Vec::new(),
        ItemAttrs::Includes(paths) => paths
    };

    // To avoid stupidity, we will not accept repeated modules
    let mut already_added: HashSet<String> = HashSet::new();
    for fn_path in included {
        // let path = fn_path; // Original path, what we need
        let mod_path = match ModulePath::from_path(fn_path.clone()) {
            Ok(mod_path) => mod_path,
            Err(err) => return Err(err)
        };
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
        for mod_item in mod_items {
            let (new_item, attrs) = match mod_item {
                Item::Fn(mod_fn) => (ModuleItem::Fn(mod_fn.sig.ident), mod_fn.attrs),
                Item::Enum(mod_enum) => (ModuleItem::Enum(mod_enum.ident), mod_enum.attrs),
                Item::Impl(mod_impl) => (ModuleItem::Impl(*mod_impl.self_ty), mod_impl.attrs),
                Item::Mod(mod_mod) => {
                    return Err(syn_error(
                        mod_mod, 
                        "Can't implement recursive modules. You should combine them separately for now"
                    ))
                },
                _ => continue
            };
            if !contains_attr(&attrs, MLUA_BINDGEN_ATTR) { 
                continue 
            };
            items.push(new_item);
        }    
    };

    Ok(ParsedModule {
        ident,
        visibility,
        includes,
        items
    })
}