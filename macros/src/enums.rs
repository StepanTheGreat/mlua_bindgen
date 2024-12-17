use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{Expr, ItemEnum, Lit, UnOp};

use crate::utils::macro_error;

/// This will simply implement the [`mlua_bindgen::AsTable`] trait for the table, it doesn't overwrite
/// anything. The reason it's not in the separate derive, is that the same macro implements the same trait
/// for structs as well. I guess just for consistency? It would be strange if the same trait is applied differently
/// for different types (like #[derive(AsTable)] for enums, and #[mlua_bindgen] for structs).
/// 
/// And yes, I call them "structs", even though they are impl blocks just for simplicity.
/// 
/// Currently these enums don't support discriminants, so all values start from 0.
pub fn expand_enum(input: TokenStream2, item: ItemEnum) -> TokenStream2 {
    let name = item.ident.to_token_stream();
    let mut variants: Vec<TokenStream2> = Vec::new();

    let mut value = 0;
    for variant in item.variants.iter() {
        let vname = variant.ident.to_token_stream();
        if let Some((_, ref expr)) = variant.discriminant {
            
            // Trying to avoid nesting here. Plus I'm over-checking errors to avoid undefined behaviour.
            let lit = if let Expr::Lit(lit) = expr { lit } else { 
                return macro_error(
                    expr, 
                    "Failed to parse enum disciminant. Make sure to use positive integer values"
                );
            };
            let lit_int = if let Lit::Int(ref lit_int) = lit.lit { lit_int } else { 
                return macro_error(
                    expr, 
                    "Only positive integers are accepted in enum discriminants"
                );
            };

            if let Ok(val) = lit_int.base10_parse::<isize>() {
                value = val;
            } else {
                return macro_error(
                    expr, 
                    "Failed to parse the discriminant. Expected an integer value"
                );
            };
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