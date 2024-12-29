use std::fmt::Display;

use proc_macro2::TokenStream as TokenStream2;
use syn::{
    parse::Parse, parse2, spanned::Spanned, Expr, ExprArray, Ident, Item, ItemEnum, ItemFn,
    ItemImpl, ItemMod, Token,
};

pub const MLUA_BINDGEN_ATTR: &str = "mlua_bindgen";

/// A parsed Item kind. Unsupported items are put in as they are, to allow error checking
pub enum ItemKind {
    Impl(ItemImpl),
    Fn(ItemFn),
    Mod(ItemMod),
    Enum(ItemEnum),
    /// While
    Unsupported(Item),
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
        _ => ItemKind::Unsupported(item),
    }
}

/// The attributes that are only used by the [`mlua_bindgen`] macro. For now I'm leaving it an enum, since
/// I only have a single attribute, but in the future I'm thinking of changing it to a struct with a vector
/// of enums instead.
pub enum ItemAttrs {
    Empty,
    /// A vector of module function paths (like `[math_module, some_module, ...]`)
    Includes(Vec<syn::Path>),
}

impl Parse for ItemAttrs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(Self::Empty);
        }

        // Parse the `include` keyword
        let ident = input.parse::<Ident>()?;
        if ident != "include" {
            return Err(syn::Error::new_spanned(
                ident,
                "Only \"include\" keyword is accepted",
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
        }

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
    D: Display,
{
    syn::Error::new(span.span(), message)
}

/// Simply iterates over attributes and checks whether at least one of the attributes matches against the
/// supplied `needed` attribute string.
///
/// This is only used inside modules to check whether an item contains the `#[mlua_bindgen]` attribute.
pub fn contains_attr(attrs: &[syn::Attribute], needed: &str) -> bool {
    for attr in attrs {
        if attr.path().is_ident(needed) {
            return true;
        }
    }
    false
}

/// Convert a string into an ident token.
///
/// The reason it can't already be done via str.to_token_stream() is that
/// it will include the quote characters as well. This is workaround.
pub fn str_to_ident(input: &str) -> syn::Ident {
    syn::Ident::new(input, proc_macro2::Span::call_site())
}

pub trait LastPathIdent {
    /// Get the last ident segment form the path.
    fn last_ident(&self) -> &Ident;
}

impl LastPathIdent for syn::Path {
    fn last_ident(&self) -> &Ident {
        self.segments.last().map(|seg| &seg.ident).unwrap()
    }
}

/// Searches for a "lua" prefix in a string and returns the index of the first character AFTER the prefix.
/// 
/// # Example
/// ```
/// get_lua_prefix("LuaHello") -> [Some(3)]
/// // where 3 --------^  (points to "H")
/// ```
/// 
/// This method only searches for a prefixes like "lua" or "lua_". If the string starts with
/// "s_lua" - it will return [None]
pub fn get_lua_prefix(s: &str) -> Option<usize> {
    let lowcase = s.to_lowercase();

    // The order matters here
    for prefix in ["lua_", "lua"] {
        if lowcase.starts_with(prefix) {
            return Some(prefix.len());
        }
    }
    None
}

/// Try to remove a lua prefix and return a new string. If there's no prefix - it just returns
/// the same string back.
/// 
/// # Example
/// ```
/// let string = "LuaType".to_owned();
/// let no_prefix = remove_lua_prefix(string);
/// assert_eq!(no_prefix, "Type".to_owned());
/// 
/// // A character "s" here isn't allowed, so it won't change anything
/// assert_eq!(remove_lua_prefix("sluaType".to_owned()), "sluaType".to_owned());
/// ```
pub fn remove_lua_prefix(s: String) -> String {
    match get_lua_prefix(&s) {
        Some(up_to) => s[up_to..].to_string(),
        None => s
    }
}

mod test {
    use crate::utils::remove_lua_prefix;

    use super::get_lua_prefix;

    #[test]
    fn lua_prefix() {
        assert_eq!(get_lua_prefix("LuaType"), Some(3));
        assert_eq!(get_lua_prefix("lua_func"), Some(4));
        assert_eq!(get_lua_prefix("HLua"), None);
        assert_eq!(get_lua_prefix("slua_func"), None);
    }

    #[test]
    fn rm_lua_prefix() {
        assert_eq!(remove_lua_prefix("LuaType".to_owned()), "Type".to_owned());
        assert_eq!(remove_lua_prefix("lua_func".to_owned()), "func".to_owned());

        assert_eq!(remove_lua_prefix("HLua".to_owned()), "HLua".to_owned());
        assert_eq!(remove_lua_prefix("slua_func".to_owned()), "slua_func".to_owned());
    }
}