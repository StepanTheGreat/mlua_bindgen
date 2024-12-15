use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::ItemEnum;

use crate::utils::macro_error;

/// Enums simply expand into tables with variants as their keys.
/// Currently these enums don't support discriminants, so all values start from 0.
pub fn expand_enum(input: TokenStream2, item: ItemEnum) -> TokenStream2 {
    let name = item.ident.to_token_stream();
    let mut variants: Vec<TokenStream2> = Vec::new();

    let mut value = 0;
    for variant in item.variants.iter() {
        let vname = variant.ident.to_token_stream();
        if variant.discriminant.is_some() {
            return macro_error(
                variant, 
                "mlua_bindgen enums don't support discriminants currently"
            )
        }
        variants.push(quote! {
            table.set(stringify!(#vname), #value)?;
        });
        value += 1;
    }

    quote! {
        #input

        impl ::mlua_bindgen::AsTable for #name {
            fn as_table(lua: &::mlua::Lua) -> ::mlua::Result<::mlua::Table> {
                let table = lua.create_table()?;
                #(#variants)*
                Ok(table)
            }
        }
    }
}