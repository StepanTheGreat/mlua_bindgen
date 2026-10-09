use syn::{Ident, ItemType, spanned::Spanned};

use crate::utils::{ItemAttribute, ItemAttributes, MLUA_IGNORE_BINDGEN_ATTR, contains_attr, parse_documentation_attributes, syn_error};

/// A parsed alias type that features a name, documentation and its alias value   
pub struct ParsedAlias {
    pub ident: Ident,
    pub docs: Option<String>,
    pub alias: String,
    pub bindgen_ignore: bool,
}

impl ParsedAlias {
    /// An empty constructor exclusively to avoid enum parsing on macro expansion
    pub fn from_ident(ident: Ident) -> Self {
        Self {
            ident,
            bindgen_ignore: false,
            alias: "".to_owned(),
            docs: None
        }
    }
}

/// Parse an [`ItemType`] into [`ParsedTypeAlias`].
pub fn parse_alias(item: ItemType, attrs: ItemAttributes) -> syn::Result<ParsedAlias> {
    let bindgen_ignore = contains_attr(&item.attrs, MLUA_IGNORE_BINDGEN_ATTR);

    let docs = parse_documentation_attributes(&item.attrs);
    let mut alias = None;

    for attr in attrs.0 {
        if let ItemAttribute::Alias(alias_value) = attr {
            alias = Some(alias_value);
            break;
        }
    }

    let alias = alias.ok_or(syn_error(
        item.span(), 
        "No alias found for a type, add `#[mlua_bindgen(alias = \"my alias\"]`"
    ))?;

    let ident = item.ident;

    Ok(ParsedAlias { ident, docs, alias, bindgen_ignore })
}
