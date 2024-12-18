//! This library isn't supposed to be exported, it only exists to share some common
//! code between the macro and bindgen APIs.
use std::fmt::{Debug, Display};

use proc_macro2::TokenStream as TokenStream2;
use syn::{parse::Parse, parse2, spanned::Spanned, token::Token, Expr, ExprArray, Ident, Item, ItemEnum, ItemFn, ItemImpl, ItemMod, Token};

pub mod items;

/// A parsed Item kind. Unsupported items are put in as they are, to allow error checking
pub enum ItemKind {
    Impl(ItemImpl),
    Fn(ItemFn),
    Mod(ItemMod),
    Enum(ItemEnum),
    /// While
    Unsupported(Item)
}

/// Parse an item and return corresponding to it enum. It should be the first thing a macro/bindgen
/// should do when they encounter an item.
pub fn parse_item(input: TokenStream2) -> ItemKind {
    let item = parse2::<Item>(input.clone()).expect("Failed to parse the item");

    match item {
        Item::Impl(item) => ItemKind::Impl(item),
        Item::Fn(item) => ItemKind::Fn(item),
        Item::Enum(item) => ItemKind::Enum(item),
        Item::Mod(item) => ItemKind::Mod(item),
        _ => ItemKind::Unsupported(item)
    }
}

/// The attributes that are only used by the [`mlua_bindgen`] macro. For now I'm leaving it an enum, since
/// I only have a single attribute, but in the future I'm thinking of changing it to a struct with a vector
/// of enums instead. 
pub enum ItemAttrs {
    Empty,
    /// A vector of module function paths (like `[math_module, some_module, ...]`)
    Includes(Vec<syn::Path>)
}

impl Parse for ItemAttrs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(Self::Empty);
        }

        // Parse the `include` keyword
        let ident = input.parse::<Ident>()?;
        if ident.to_string() != "include" {
            return Err(syn::Error::new_spanned(
                ident, 
                "Only \"include\" keyword is accepted"
            ));
        }

        // Parse the `=` sign
        input.parse::<Token![=]>()?;

        // Then we expect a list of expressions: `[expr1, expr2]`
        let items = input.parse::<ExprArray>()?;
        
        // Finally, we collect these expressions into the included vector.
        // (Or to be precise, only the ones that are Path)
        let mut included: Vec<syn::Path> = Vec::new();
        for item in items.elems {
            if let Expr::Path(path) = item {
                included.push(path.path);
            }
        };

        Ok(Self::Includes(included))
    }
}

/// Parse tokens into [`ItemAttrs`]
pub fn parse_attrs(input: TokenStream2) -> syn::Result<ItemAttrs> {
    parse2::<ItemAttrs>(input)
}

/// Constructs a quick error;
pub fn syn_error<S, D>(span: S, message: D) -> syn::Error
where
    S: Spanned,
    D: Display 
{
    syn::Error::new(span.span(), message)
}