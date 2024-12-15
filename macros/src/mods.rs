//! A confusing name, but it basically stands for "modules"

use proc_macro2::TokenStream as TokenStream2;
use syn::ItemMod;
use quote::quote;

pub fn expand_mod(input: TokenStream2, item: ItemMod) -> TokenStream2 {
    
    quote! {}
}