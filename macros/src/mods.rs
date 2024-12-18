//! A confusing name, but it basically stands for "modules"

use proc_macro2::TokenStream as TokenStream2;
use shared::{items::mods::{parse_mod, ModuleItem}, str_to_ident, ItemAttrs};
use syn::ItemMod;
use quote::{quote, ToTokens};

use shared::mods::MODULE_SUFFIX;

/// This function expands modules. The task is a bit more complicated, since now we not only
/// include inner items, but also parse macro attributes for a list of arguments like 
/// ```
/// #[mlua_bindgen(include = [...])]
/// ```
/// 
/// This is used to import other modules into the module space, and I think that's the best solution overall
/// (In terms of parsing and convenience)
pub fn expand_mod(attrs: ItemAttrs, input: TokenStream2, item: ItemMod) -> TokenStream2 {
    let parsed_mod = match parse_mod(attrs, item) {
        Ok(parsed_mod) => parsed_mod,
        Err(err) => return err.into_compile_error()
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
                let name = item.to_token_stream();
                quote! {
                    exports.set(
                        stringify!(#name), 
                        #mod_name::#name::as_table(lua)?
                    )?;
                }
            },
            ModuleItem::Fn(item) => {
                let name = item.to_token_stream();
                quote! {
                    exports.set(
                        stringify!(#name), 
                        lua.create_function(#mod_name::#name)?
                    )?;
                }
            },
            ModuleItem::Impl(item) => {
                let name = item.to_token_stream();
                quote! {
                    exports.set(
                        stringify!(#name), 
                        #mod_name::#name::as_table(lua)?
                    )?;
                }
            }
        });
    };

    let mod_name_module = str_to_ident(&format!("{mod_name}{MODULE_SUFFIX}"));

    quote! {
        #input
        
        #vis_param fn #mod_name_module(lua: &::mlua::Lua) -> ::mlua::Result<::mlua::Table> {
            let exports = lua.create_table()?;
            #(#exports)*
            Ok(exports)
        }
    }
}

// fn expand_old() {
//     for fn_path in included.included {
//         let path = fn_path.to_token_stream(); // Original path, what we need

//         // Here we're getting the last segment in the path, and converting into token stream. It will be the module
//         // name itself
//         let fn_name = fn_path.segments.last().map(|seg|  seg.ident.to_string()).unwrap();

//         // Now a bit of my personal stupidity, but we need to remove the `_module` suffix from the module function.
//         // It would be silly to call `math_module.mul(a, b)` inside Lua, so we remove that.
//         // And since _module itself isn't a banned word, we want to only remove the last appearance of it, so that
//         // `my_module_module` would transform into `my_module` and not into `my`. 
//         let split_pos = match fn_name.rfind(MODULE_SUFFIX) {
//             Some(pos) => pos,
//             None => return syn::Error::new_spanned(
//                 fn_path, 
//                 &format!("Included modules have to end with the \"{MODULE_SUFFIX}\" keyword")
//             ).into_compile_error()
//         };
//         let (left_fn_name, _) = fn_name.split_at(split_pos);
//         let left_fn_name = str_to_ident(left_fn_name);

//         exports.push(quote! {
//             exports.set(
//                 stringify!(#left_fn_name), 
//                 #path(lua)?
//             )?;
//         });

//         // Yep, give him an error. That's silly
//         if !already_added.contains(&fn_name) {
//             already_added.insert(fn_name);
//         } else {
//             return syn::Error::new_spanned(
//                 fn_path, 
//                 "Modules can't be repeatedly added"
//             ).into_compile_error()
//         }
//     }

//     // Now we iterate actual module items, and if they contain the [`MLUA_BINDGEN_ATTR`] attribute - 
//     // we add their module registration code to the exports.
//     if let Some((_, items)) = item.content {
//         for mod_item in items {
//             match mod_item {
//                 Item::Fn(mod_fn) => {
//                     if !has_attr(&mod_fn.attrs, MLUA_BINDGEN_ATTR) { continue };
//                     let fn_name = mod_fn.sig.ident.to_token_stream();
//                     exports.push(quote! {
//                         exports.set(
//                             stringify!(#fn_name), 
//                             lua.create_function(#mod_name::#fn_name)?
//                         )?;
//                     });
//                 },
//                 Item::Enum(mod_enum) => {
//                     if !has_attr(&mod_enum.attrs, MLUA_BINDGEN_ATTR) { continue };
//                     let enum_name = mod_enum.ident.to_token_stream();
//                     exports.push(quote! {
//                         exports.set(
//                             stringify!(#enum_name), 
//                             #mod_name::#enum_name::as_table(lua)?
//                         )?;
//                     });
//                 },
//                 Item::Impl(mod_impl) => {
//                     if !has_attr(&mod_impl.attrs, MLUA_BINDGEN_ATTR) { continue };
//                     let impl_name = mod_impl.self_ty.to_token_stream();
//                     exports.push(quote! {
//                         exports.set(
//                             stringify!(#impl_name), 
//                             #mod_name::#impl_name::as_table(lua)?
//                         )?;
//                     });
//                 },
//                 Item::Mod(mod_mod) => {
//                     // I guess could be useful
//                     return macro_error(
//                         mod_mod, 
//                         "Can't implement recursive modules. You should combine them separately for now"
//                     )
//                 },
//                 _ => {}
//             }
//         }    
//     }

//     // Here we're just concatenating original module name with a  `_module` suffix.
//     // Ex: "math" => "math_module"
//     // I guess it helps distinguishing from modules and their module functions. For me at least
//     // it makes more sense.
//     let mod_name_module = str_to_ident(&format!("{mod_name}{MODULE_SUFFIX}"));

//     // We keep the original input, and just add our module function on top.
//     quote! {
//         #input
        
//         #vis_param fn #mod_name_module(lua: &::mlua::Lua) -> ::mlua::Result<::mlua::Table> {
//             let exports = lua.create_table()?;
//             #(#exports)*
//             Ok(exports)
//         }
//     }
// }