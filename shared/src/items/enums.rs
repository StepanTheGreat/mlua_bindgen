use syn::{Expr, Ident, ItemEnum, Lit};

use crate::utils::syn_error;

pub type LuaVariantType = usize;

/// Contains general enum information, that is important both for macros and bindgen parsers
pub struct ParsedEnum {
    pub ident: Ident,
    pub variants: Vec<(Ident, LuaVariantType)>,
}

/// Parse an [`ItemEnum`] into [`ParsedEnum`].
pub fn parse_enum(item: ItemEnum) -> syn::Result<ParsedEnum> {
    let ident = item.ident;
    let mut variants: Vec<(Ident, LuaVariantType)> = Vec::new();

    let mut value: LuaVariantType = 0;
    for variant in item.variants.into_iter() {
        let vident = variant.ident;
        if let Some((_, ref expr)) = variant.discriminant {
            // Trying to avoid nesting here. Plus I'm over-checking errors to avoid undefined behaviour.
            let lit = if let Expr::Lit(lit) = expr {
                lit
            } else {
                return Err(syn_error(
                    expr,
                    "Failed to parse enum disciminant. Make sure to use positive integer values",
                ));
            };
            let lit_int = if let Lit::Int(ref lit_int) = lit.lit {
                lit_int
            } else {
                return Err(syn_error(
                    expr,
                    "Only positive integers are accepted in enum discriminants",
                ));
            };

            if let Ok(val) = lit_int.base10_parse::<LuaVariantType>() {
                value = val;
            } else {
                return Err(syn_error(
                    expr,
                    "Failed to parse the discriminant. Expected an integer value",
                ));
            };
        }
        variants.push((vident, value));
        value += 1;
    }

    Ok(ParsedEnum { ident, variants })
}
