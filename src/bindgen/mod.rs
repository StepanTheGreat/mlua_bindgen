//! Bindgen functionality. This is supposed to be accessed via the "bindgen" feature.

//! # Lua Bindgen
//! ## Generate Lua bindings for your Rust types!

use shared::mods::{parse_mod, ParsedModule};
use shared::utils::{parse_attributes, ItemAttributes, MLUA_BINDGEN_ATTR};
use std::fs;
use std::{collections::HashMap, path::PathBuf};
use syn::Attribute;
use syn::Item;
use types::{LuaFile, LuaModule};
use utils::{find_attr, get_attribute_args};
use walkdir::WalkDir;

use crate::error::Error;

mod expand;
mod types;
mod utils;

/// The char that will be used to diffirentiate between userdata types and tables
pub(crate) const USERDATA_CHAR: &str = "u";

/// A collection of all mlua_bindgen items in a single structure
pub struct ParsedFile {
    mods: Vec<ParsedModule>,
}

impl ParsedFile {
    /// Create self from a collection of other parsed files.
    ///
    /// This is useful when parsing a lot of files together and then reuniting all found modules
    fn from_parsed_files(parsed_files: Vec<ParsedFile>) -> Self {
        let mut mods = Vec::new();

        for parsed_file in parsed_files {
            mods.extend(parsed_file.mods.into_iter());
        }

        Self { mods }
    }

    /// Transform all parsed structure into Lua structures
    pub fn transform_to_lua<'a>(self) -> Result<LuaFile<'a>, Error> {
        let mut lua_file = LuaFile::new();

        // TODO: This is an absolute mess of a code, please fix this later

        // Here we parse and collect all lua modules.
        // We also check if any of them is main, and if so - we add it to a separate main_mod variable,
        // which will be important for us later
        let mut main_mod: Option<LuaModule> = None;
        let mut mod_map: HashMap<String, LuaModule> = HashMap::new();
        for parsed_mod in self.mods {
            let lua_mod = LuaModule::from_parsed(parsed_mod)?;
            if lua_mod.ismain {
                if main_mod.is_some() {
                    return Err(Error::MainModules { many: true });
                } else {
                    main_mod = Some(lua_mod);
                }
            } else {
                if mod_map.contains_key(&lua_mod.name) {
                    return Err(Error::Unimplemented { 
                        message: format!("Found a colliding module name: \"{}\". mlua_bindgen currently can only work with unique module names, as it doesn't understand the module tree.", &lua_mod.name)
                    })
                } 
                mod_map.insert(lua_mod.name.clone(), lua_mod);
            }
        }

        let main_mod = main_mod.ok_or(Error::MainModules { many: false })?;

        // Now we need to insert modules appropriately, starting from the main module
        loop {
            // A tuple of parent module, and a list of modules to be inserted into them
            let mut insertions: HashMap<String, Vec<String>> = HashMap::new();
            for (mod_name, module) in mod_map.iter() {
                if module.includes.is_empty() {
                    continue;
                };

                // Iterate over all module paths this module requires
                for needed_path in module.includes.iter() {
                    // Check whether it exists in the map
                    if let Some(needed_mod) = mod_map.get(&needed_path.name()) {
                        // Also make sure that the found module doesn't include anything. If it does -
                        // we need to resolve that module first.
                        if needed_mod.includes.is_empty() {
                            if insertions.contains_key(mod_name) {
                                insertions
                                    .get_mut(mod_name)
                                    .unwrap()
                                    .push(needed_mod.name.clone());
                            } else {
                                insertions.insert(mod_name.clone(), vec![needed_mod.name.clone()]);
                            }
                        }
                    }
                }
            }

            if insertions.is_empty() {
                // No more insertions to make, the modules are prepared
                break;
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
            let contains = main_mod
                .includes
                .iter()
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
        Some(attr_tokens) => match parse_attributes(attr_tokens) {
            Ok(attrs) => Some(attrs),
            Err(_err) => None,
        },
        None => Some(ItemAttributes::empty()),
    }
}

/// Parsed a file.
fn parse_file(file: syn::File) -> syn::Result<ParsedFile> {
    let mut mods: Vec<ParsedModule> = Vec::new();

    for item in file.items {
        if let Item::Mod(mod_item) = item {
            if let Some(attrs) = get_bindgen_attrs(&mod_item.attrs) {
                let mut parsed_mod = parse_mod(attrs, mod_item, true)?;
                // Of course we need to check if the bindgen_ignore marker is present, and if so - don't add the
                // module.
                if !parsed_mod.bindgen_ignore {
                    parsed_mod.clean_ignored();
                    mods.push(parsed_mod);
                }
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

/// A builder struct for setting input files, the output file and starting the parsing process.
pub struct BindgenTransformer {
    pub in_paths: Vec<PathBuf>,
    pub out_path: Option<PathBuf>,
}

impl BindgenTransformer {
    pub fn new() -> Self {
        Self {
            in_paths: Vec::new(),
            out_path: None,
        }
    }

    /// Add a rust source file path to process
    pub fn add_input_file(mut self, file: impl Into<PathBuf>) -> Self {
        self.in_paths.push(file.into());
        self
    }

    fn push_dir(&mut self, dir: PathBuf, depth: usize) {
        for entry in WalkDir::new::<PathBuf>(dir)
            .max_depth(depth)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }

            let file_name = entry.file_name();
            let file_name = match file_name.to_str() {
                Some(file_name) => file_name,
                None => continue,
            };

            if file_name.ends_with(".rs") {
                self.in_paths.push(entry.path().into());
            }
        }
    }

    /// Add an entire directory and its inner files.
    pub fn add_input_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.push_dir(dir.into(), 1);
        self
    }

    /// Add a directory and its inner files based on depth (i.e. files of inner directories, and so on)
    ///
    /// - 0 Means only the the file path itself
    /// - 1 is for the file path itself and its inner files
    /// - 2... to account for inner-inner files and so on.
    pub fn add_input_dir_with_depth(mut self, dir: impl Into<PathBuf>, depth: usize) -> Self {
        self.push_dir(dir.into(), depth);
        self
    }

    /// Set the output declaration file.
    ///
    /// A luau declaration file should end with `.d.lua`
    pub fn set_output_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.out_path = Some(path.into());
        self
    }

    /// Start parsing the files and collect modules into a [ParsedFile]
    pub fn parse(self) -> Result<ParsedFile, Error> {
        let mut parsed_files = Vec::new();

        for in_path in self.in_paths {
            let src = fs::read_to_string(in_path)?;
            let file = syn::parse_file(&src)?;
            parsed_files.push(parse_file(file)?);
        }

        Ok(ParsedFile::from_parsed_files(parsed_files))
    }
}
