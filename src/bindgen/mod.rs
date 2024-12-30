//! Bindgen functionality. This is supposed to be accessed via the "bindgen" feature.

//! # Lua Bindgen
//! ## Generate Lua bindings for your Rust types!

use shared::mods::{parse_mod, ParsedModule};
use shared::utils::{parse_attributes, ItemAttributes, LastPathIdent, MLUA_BINDGEN_ATTR};
use std::fs;
use std::path::Path;
use std::{collections::HashMap, path::PathBuf};
use syn::Item;
use syn::{Attribute, Type};
use types::{LuaFile, LuaModule};
use utils::{find_attr, get_attribute_args};

mod types;
mod utils;
mod expand;

/// A collection of all mlua_bindgen items in a single structure
pub struct ParsedFile {
    pub mods: Vec<ParsedModule>,
}

impl ParsedFile {
    /// Transform all parsed structure into Lua compatible structures
    pub fn transform_to_lua<'a>(self) -> anyhow::Result<LuaFile<'a>> {
        let mut lua_file = LuaFile::new();

        // TODO: This is an absolute mess of a code, please fix this later
    
        // Here we parse and collect all lua modules. 
        // We also check if any of them is main, and if so - we add it to a separate main_mod variable,
        // which will be important for us later
        let mut main_mod: Option<LuaModule> = None;
        let mut mod_map = HashMap::new();
        for parsed_mod in self.mods {
            let lua_mod = LuaModule::from_parsed(parsed_mod)?;
            if lua_mod.ismain {
                if main_mod.is_some() {
                    panic!("Found more than 2 main modules. Only 1 can be present at the same time");
                } else {
                    main_mod = Some(lua_mod);
                }
            } else {
                mod_map.insert(lua_mod.name.clone(), lua_mod);
            }
        }
        println!("Got modules: {:}", mod_map.len());

        let main_mod = main_mod.expect("Bindgen requires a main module to be present");

        // Now we need to insert modules appropriately, starting from the main module
        loop {
            // A tuple of parent module, and a list of modules to be inserted into them
            let mut insertions: HashMap<String, Vec<String>> = HashMap::new();
            for (mod_name, module) in mod_map.iter() {
                if module.includes.is_empty() { continue };

                // Iterate over all module paths this module requires
                for needed_path in module.includes.iter() {
                    // Check whether it exists in the map
                    if let Some(needed_mod) = mod_map.get(&needed_path.name()) {
                        // Also make sure that the found module doesn't include anything. If it does - 
                        // we need to resolve that module first.
                        if needed_mod.includes.is_empty() {
                            if insertions.contains_key(mod_name) {
                                insertions.get_mut(mod_name).unwrap().push(needed_mod.name.clone());
                            } else {
                                insertions.insert(mod_name.clone(),  vec![needed_mod.name.clone()]);
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

        // Now we just need to insert all collected modules into the main namespace

        for module in mod_map.into_values() {
            let contains = main_mod.includes.iter()
                .find(|path| module.is(path))
                .is_some();

            if contains {
                lua_file.add_item(module)    
            }
        }

        // Now we remove all the main module items into the main file instead
        lua_file.add_items(main_mod.enums);
        lua_file.add_items(main_mod.funcs);
        lua_file.add_items(main_mod.impls);

        Ok(lua_file)
    }
}

/// Find an [`MLUA_BINDGEN_ATTR`] argument, then convert it into tokens,
/// then parse it into [`ItemAttrs`]. It can fail at any stage and return [`None`]
fn get_bindgen_attrs(item_attrs: &[Attribute]) -> Option<ItemAttributes> {
    let attr = find_attr(item_attrs, MLUA_BINDGEN_ATTR)?;

    match get_attribute_args(attr) {
        Some(attr_tokens) => {
            match parse_attributes(attr_tokens) {
                Ok(attrs) => Some(attrs),
                Err(_err) => None
            }
        },
        None => Some(ItemAttributes::empty())
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

pub struct BindgenTransformer {
    pub in_paths: Vec<PathBuf>,
    pub out_path: Option<PathBuf>
}

impl BindgenTransformer {
    pub fn new() -> Self {
        Self {
            in_paths: Vec::new(),
            out_path: None
        }
    }
}