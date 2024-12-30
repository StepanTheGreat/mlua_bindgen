//! A confusing name, but it basically stands for "modules"

use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use shared::{
    items::mods::{parse_mod, ModuleItem},
    utils::{remove_lua_prefix, ToIdent, ItemAttributes},
};
use syn::ItemMod;

use shared::mods::MODULE_SUFFIX;

/// This function expands modules. The task is a bit more complicated, since now we not only
/// include inner items, but also parse macro attributes for a list of arguments like
/// ```
/// #[mlua_bindgen(include = [...])]
/// ```
///
/// This is used to import other modules into the module space, and I think that's the best solution overall
/// (In terms of parsing and convenience)
pub fn expand_mod(attrs: ItemAttributes, input: TokenStream2, item: ItemMod) -> TokenStream2 {
    let parsed_mod = match parse_mod(attrs, item, false) {
        Ok(parsed_mod) => parsed_mod,
        Err(err) => return err.into_compile_error(),
    };
    let mod_name = parsed_mod.ident.to_token_stream();
    let vis_param = parsed_mod.visibility.to_token_stream();

    // This is the container for all registration code. I called it exports because...
    // it "exports" its inner items into a separate function.
    let mut exports: Vec<TokenStream2> = Vec::new();

    for included in parsed_mod.includes {
        let path = included.path.to_token_stream();
        let name = included.get_ident().to_token_stream();
        exports.push(quote! {
            exports.set(
                stringify!(#name),
                #path(lua)?
            )?;
        });
    }

    for exported in parsed_mod.items {
        exports.push(match exported {
            ModuleItem::Enum(item) => {
                let name = item.ident.to_token_stream();
                
                // Sometimes making wrappers is annoying because of name collisions.
                // Unprefixed names exist in this case to separate lua functions/types from original
                // rust functions/types. 
                //
                // This function will remove any "lua" or "lua_" prefixes from the string,
                // and automatically insert them in the table. THIS, however, doesn't rename the type/function's name
                // - only their table key.
                let unprefixed_name = remove_lua_prefix(name.to_string());

                quote! {
                    exports.set(
                        #unprefixed_name,
                        #mod_name::#name::as_table(lua)?
                    )?;
                }
            }
            ModuleItem::Fn(item) => {
                let name = item.name.to_token_stream();
                let unprefixed_name = remove_lua_prefix(name.to_string());

                quote! {
                    exports.set(
                        #unprefixed_name,
                        lua.create_function(#mod_name::#name)?
                    )?;
                }
            }
            ModuleItem::Impl(item) => {
                let name = item.name.to_token_stream();
                let unprefixed_name = remove_lua_prefix(name.to_string());

                quote! {
                    exports.set(
                        #unprefixed_name,
                        #mod_name::#name::as_table(lua)?
                    )?;
                }
            }
        });
    }

    let mod_name_module = format!("{mod_name}{MODULE_SUFFIX}").to_ident();

    quote! {
        #input

        #vis_param fn #mod_name_module(lua: &::mlua::Lua) -> ::mlua::Result<::mlua::Table> {
            let exports = lua.create_table()?;
            #(#exports)*
            Ok(exports)
        }
    }
}
