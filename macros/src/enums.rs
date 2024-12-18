use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use shared::items::enums::parse_enum;
use syn::ItemEnum;

/// This will simply implement the [`mlua_bindgen::AsTable`] trait for the table, it doesn't overwrite
/// anything. The reason it's not in the separate derive, is that the same macro implements the same trait
/// for structs as well. I guess just for consistency? It would be strange if the same trait is applied differently
/// for different types (like #[derive(AsTable)] for enums, and #[mlua_bindgen] for structs).
/// 
/// And yes, I call them "structs", even though they are impl blocks just for simplicity.
/// 
/// Currently these enums don't support discriminants, so all values start from 0.
pub fn expand_enum(input: TokenStream2, item: ItemEnum) -> TokenStream2 {
    let parsed_enum = match parse_enum(&item) {
        Ok(item) => item,
        Err(err) => return err.to_compile_error()
    };

    let name = parsed_enum.ident.to_token_stream();
    let variants: Vec<TokenStream2> = parsed_enum.variants
        .iter()
        .map(|(ident, value)| {
            let vname = ident.to_string();
            quote! { table.set(#vname, #value)?; }
        })
        .collect();

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