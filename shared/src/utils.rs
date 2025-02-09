use std::fmt::Display;

use proc_macro2::TokenStream as TokenStream2;
use syn::{
    parse::Parse, parse2, spanned::Spanned, token::Comma, Attribute, Expr, ExprArray, Ident, Item, ItemEnum, ItemFn, ItemImpl, ItemMod, Token
};

pub const MLUA_BINDGEN_ATTR: &str = "mlua_bindgen";
/// The reason it's not `MLUA_BINDGEN_IGNORE_ATTR`, is that my intellisense constantly recommends my the
/// default `MLUA_BINDGEN_ATTR`, so I renamed it to emphasize the `IGNORE` part at the start, 
/// thus avoiding mistakes.
pub const MLUA_IGNORE_BINDGEN_ATTR: &str = "mlua_bindgen_ignore";

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

/// A container for [`mlua_bindgen`] macro attributes
pub struct ItemAttributes(pub Vec<ItemAttribute>);

impl ItemAttributes {
    /// Create an empty ItemAttribute list
    pub fn empty() -> Self {
        Self(Vec::new())
    }
}

/// Kinds of attributes accepted by the macro
///
/// Some of the attributes can only be applied to specific items like modules
pub enum ItemAttribute {
    /// A vector of module function paths (like `[math_module, some_module, ...]`)
    Includes(Vec<syn::Path>),
    /// An attribute only useful in bindgen, that signifies that this is the main module entrypoint
    IsMain,
    /// An attribute that tells to keep the original name, without removing its Lua prefix.
    Preserve,
    /// Tell the bindgen to ignore this element when generating bindings. Useful when replacing standard
    /// functions like `require`
    BindgenIgnore,
    /// Tells the macro to call a post-init function under provided path before returning a module table.
    /// Useful if you need to manually modify the table.
    PostInitFunc(syn::Path),
}

impl Parse for ItemAttributes {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut attrs = Vec::new();

        if input.is_empty() {
            return Ok(ItemAttributes(attrs));
        }

        // We're looping here, since multiple attributes can be used at the same time
        loop {
            // Parse an attribute keyword
            let ident = input.parse::<Ident>()?;

            let new_attr = if ident == "include" {
                //? include = [path::my_module, local_module]

                // Parse the `=` sign
                input.parse::<Token![=]>()?;

                // Then we expect a list of expressions: `[expr1, expr2]`
                let items = input.parse::<ExprArray>()?;

                // Finally, we collect these expressions into the included vector.
                // (Or to be precise, only the ones that are Path)
                let included: Vec<syn::Path> = items
                    .elems
                    .into_iter()
                    .filter_map(|item| {
                        if let Expr::Path(path) = item {
                            Some(path.path)
                        } else {
                            None
                        }
                    })
                    .collect();

                ItemAttribute::Includes(included)
            } else if ident == "post_init" {
                //? post_init = my::path::to::func

                // Parse the `=` sign
                input.parse::<Token![=]>()?;

                let expr = input.parse::<Expr>()?;

                if let Expr::Path(path) = expr {
                    ItemAttribute::PostInitFunc(path.path)
                } else {
                    return Err(syn_error(
                        expr,
                        "Expected a function path to a post_init function",
                    ));
                }
            } else if ident == "main" {
                //? main

                ItemAttribute::IsMain
            } else if ident == "preserve" {
                return Err(syn::Error::new_spanned(
                    ident,
                    "The `preserve` attribute is currently not supported",
                ));
            } else if ident == "bindgen_ignore" {
                return Err(syn::Error::new_spanned(
                    ident,
                    "The `bindgen_ignore` attribute is currently not supported",
                ));
            } else {
                return Err(syn::Error::new_spanned(
                    ident,
                    "Unknown keyword. Only `main`, `preserve` and `include` can be used",
                ));
            };

            attrs.push(new_attr);

            if input.is_empty() {
                break;
            } else {
                // Since there's still some input present, we expect a comma separator
                input.parse::<Comma>()?;
            }
        }

        Ok(Self(attrs))
    }
}

/// Parse tokens into [`ItemAttrs`]
pub fn parse_attributes(input: TokenStream2) -> syn::Result<ItemAttributes> {
    parse2::<ItemAttributes>(input)
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

/// A trait for converting types into the Ident token
///
/// Currently only strings are supported
pub trait ToIdent {
    /// Convert a string into an ident token.
    ///
    /// The reason it can't already be done via str.to_token_stream() is that
    /// it will include the quote characters as well. This is workaround.
    fn to_ident(&self) -> syn::Ident;
}

impl ToIdent for &str {
    fn to_ident(&self) -> syn::Ident {
        syn::Ident::new(self, proc_macro2::Span::call_site())
    }
}

impl ToIdent for String {
    fn to_ident(&self) -> syn::Ident {
        syn::Ident::new(self, proc_macro2::Span::call_site())
    }
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
        None => s,
    }
}

#[cfg(test)]
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
        assert_eq!(
            remove_lua_prefix("slua_func".to_owned()),
            "slua_func".to_owned()
        );
    }
}
