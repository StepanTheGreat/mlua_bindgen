use proc_macro2::TokenStream as TokenStream2;
use shared::{aliases::parse_alias, utils::ItemAttributes};
use syn::ItemType;

/// There's nothing to expand, since aliases are bindgen-only concepts
pub fn expand_alias(input: TokenStream2, item: ItemType, attrs: ItemAttributes) -> TokenStream2 {
    if let Err(err) = parse_alias(item, attrs) {
        return err.to_compile_error();
    }

    input
}