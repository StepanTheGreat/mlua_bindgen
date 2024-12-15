use proc_macro2::TokenStream as TokenStream2;
use syn::{
    ImplItem, ItemImpl
};
use quote::{quote, ToTokens};

use crate::utils::*;

/// Expand the impl block. This will overwrite the entire impl block
/// with an implementation of [`mlua::UserData`].
pub fn expand_impl(input: ItemImpl) -> TokenStream2 {
    let mut methods = Vec::new();
    let mut fields = Vec::new();
    let mut funcs = Vec::new();

    // Analyze the `impl` block
    let impl_name = input.self_ty.to_token_stream();

    // Convert the generated code to a TokenStream
    for itm in input.items.iter() {
        if let ImplItem::Fn(ref func) = itm {
            for attr in func.attrs.iter() {
                let p = attr.path();
                if p.is_ident("doc") {
                    // Ignore the documentation
                    continue;
                } else if p.is_ident("method") {
                    methods.push(parse_function(&func, FuncKind::Method));
                } else if p.is_ident("method_mut") {
                    methods.push(parse_function(&func, FuncKind::MethodMut));
                } else if p.is_ident("func") {
                    funcs.push(parse_function(&func, FuncKind::Func));
                } else if p.is_ident("get") {
                    fields.push(parse_field(&func, FieldKind::Getter));
                } else if p.is_ident("set") {
                    fields.push(parse_field(&func, FieldKind::Setter));
                } else {
                    return syn::Error::new_spanned(
                        attr, 
                        "Incorrect attributes. Only method, method_mut, func or doc can be used"
                    ).into_compile_error().into();
                }
            }
        } else {
            panic!("can't use impl items other than functions in export_lua macro")
        }
    }

    // Not implementing this trait if there are no functions.
    let table_impl = if funcs.len() > 0 { 
        quote! {
            impl mlua_bindgen::AsTable for #impl_name {
                fn as_table(lua: &mlua::Lua) -> mlua::Result<mlua::Table> {
                    let table = lua.create_table()?;
                    #(#funcs)*
                    Ok(table)
                }
            }
        }
    } else {
        TokenStream2::new()
    };

    quote! {   
        impl mlua::UserData for #impl_name {
            fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) { 
                #(#fields)*
            }
    
            fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) { 
                #(#methods)*
            }
        }

        #table_impl
    }.into()
}