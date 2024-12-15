//! A confusing name, but it basically stands for "modules"

use proc_macro2::{Span, TokenStream as TokenStream2};
use syn::{Item, ItemMod};
use quote::{quote, ToTokens};

use crate::utils::{has_attr, macro_error, MLUA_BINDGEN_ATTR};

pub fn expand_mod(input: TokenStream2, item: ItemMod) -> TokenStream2 {
    let mod_name = item.ident.to_token_stream();
    let vis_param = item.vis.to_token_stream();
    let mut exports: Vec<TokenStream2> = Vec::new();
    if let Some((_, items)) = item.content {
        for mod_item in items {
            match mod_item {
                Item::Fn(mod_fn) => {
                    if !has_attr(&mod_fn.attrs, MLUA_BINDGEN_ATTR) { continue };
                    let fn_name = mod_fn.sig.ident.to_token_stream();
                    exports.push(quote! {
                        exports.set(
                            stringify!(#fn_name), 
                            lua.create_function(#mod_name::#fn_name)?
                        )?;
                    });
                },
                Item::Enum(mod_enum) => {
                    if !has_attr(&mod_enum.attrs, MLUA_BINDGEN_ATTR) { continue };
                    let enum_name = mod_enum.ident.to_token_stream();
                    exports.push(quote! {
                        exports.set(
                            stringify!(#enum_name), 
                            #mod_name::#enum_name::as_table(lua)?
                        )?;
                    });
                },
                Item::Impl(mod_impl) => {
                    if !has_attr(&mod_impl.attrs, MLUA_BINDGEN_ATTR) { continue };
                    let impl_name = mod_impl.self_ty.to_token_stream();
                    exports.push(quote! {
                        exports.set(
                            stringify!(#impl_name), 
                            #mod_name::#impl_name::as_table(lua)?
                        )?;
                    });
                },
                Item::Mod(mod_mod) => {
                    return macro_error(
                        mod_mod, 
                        "Can't implement recursive modules. You should combine them separately for now."
                    )
                },
                _ => {}
            }
        }    
    }

    // Here we're just concatenating original module name with a  `_module` suffix.
    // Ex: "math" => "math_module"
    // This is useful for distinguishing modules and functions.
    let mod_name_module = {
        syn::Ident::new(&format!("{mod_name}_module"), Span::call_site()).to_token_stream()
    };

    quote! {
        #input
        
        #vis_param fn #mod_name_module(lua: &::mlua::Lua) -> ::mlua::Result<::mlua::Table> {
            let exports = lua.create_table()?;
            #(#exports)*
            Ok(exports)
        }
    }
}