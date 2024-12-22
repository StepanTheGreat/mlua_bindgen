use syn::{Attribute, Meta};
use proc_macro2::TokenStream as TokenStream2;


const TAB: &str = "    ";

/// Take an attribute, and convert its meta list arguments into a TokenStream. This
/// can fail if the meta token isn't a list. 
pub fn get_attribute_args(attr: &Attribute) -> Option<TokenStream2> {
    match &attr.meta {
        Meta::List(meta_list) => {
            Some(meta_list.tokens.clone())
        },
        _ => None
    }
}

/// This is the same as [`contains_attr`], but will actually return a reference to the attribute
/// if it can find it.
pub fn find_attr<'a>(attrs: &'a[syn::Attribute], needed: &str) -> Option<&'a Attribute> {
    for attr in attrs {
        if attr.path().is_ident(needed) {
            return Some(attr);
        }
    }
    None
}

/// Will transform a string and return a new one, with a specific amount of tabs.
///
/// It's supposed to be used with integers bigger than 0
pub fn add_tabs(s: String, amount: usize) -> String {
    let tab = TAB.repeat(amount);
    s.lines().map(|line| format!("{tab}{line}\n")).collect()
}

mod tests {
    use crate::bindgen::utils::add_tabs;

    #[test]
    fn tabs() {
        let s = "print(my_var)\nif everything.is_ok() then\n    print('Great!')\nend".to_owned();
        assert_eq!(
            add_tabs(s, 1),
            "    print(my_var)\n    if everything.is_ok() then\n        print('Great!')\n    end\n".to_owned()
        );
    }
}