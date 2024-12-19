use syn::{Expr, Ident, ItemEnum, Lit};

use crate::utils::syn_error;

type VariantValue = usize;

/// Contains general enum information, that is important both for macros and bindgen parsers
pub struct ParsedEnum<'a> {
    pub ident: &'a Ident,
    pub variants: Vec<(&'a Ident, VariantValue)>
}

/// Parse an [`ItemEnum`] into [`ParsedEnum`].
pub fn parse_enum<'a>(item: &'a ItemEnum) -> syn::Result<ParsedEnum<'a>> {
    let ident = &item.ident;
    let mut variants: Vec<(&'a Ident, VariantValue)> = Vec::new();

    let mut value: VariantValue = 0;
    for variant in item.variants.iter() {
        let vident = &variant.ident;
        if let Some((_, ref expr)) = variant.discriminant {
            // Trying to avoid nesting here. Plus I'm over-checking errors to avoid undefined behaviour.
            let lit = if let Expr::Lit(lit) = expr { lit } else { 
                return Err(syn_error(
                    expr, 
                    "Failed to parse enum disciminant. Make sure to use positive integer values"
                ));
            };
            let lit_int = if let Lit::Int(ref lit_int) = lit.lit { lit_int } else { 
                return Err(syn_error(
                    expr, 
                    "Only positive integers are accepted in enum discriminants"
                ));
            };

            if let Ok(val) = lit_int.base10_parse::<VariantValue>() {
                value = val;
            } else {
                return Err(syn_error(
                    expr, 
                    "Failed to parse the discriminant. Expected an integer value"
                ));
            };
        }
        variants.push((vident, value));
        value += 1;
    }

    Ok(ParsedEnum {
        ident,
        variants
    })
}