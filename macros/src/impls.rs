use proc_macro2::TokenStream as TokenStream2;
use shared::{funcs::{parse_func, FuncKind}, syn_error};
use syn::{
    ImplItem, ImplItemFn, ItemImpl
};
use quote::{quote, ToTokens};

use crate::utils::*;

/// This will parse the supplied impl function (and its [`FieldKind`]), extract neccessary information,
/// then transform into a field registration code for mlua. 
pub fn parse_field(input: ImplItemFn, kind: FieldKind) -> TokenStream2 {
    // This is completely unsound, but for now that's our workaround
    let exfunc = match parse_func(input, &FuncKind::MethodMut) {
        Ok(func) => func,
        Err(err) => return err.into_compile_error()
    };

    let name_str = name.to_string(); // This is used mostly for errors

    if trait_arg_names.len() < exfunc.req_arg_count {
        let (field_type, args_fmt) = match kind {
            FieldKind::Getter => ("getter", "&Lua, &Self"),
            FieldKind::Setter => ("setter", "&Lua, &mut Self"),
        };
        return syn_error(
            name, 
            format!("Not enough arguments for {} \"{}\". It takes {} as its first {} arguments", field_type, &name_str, args_fmt, exfunc.req_arg_count)
        ).to_compile_error();
    }

    // Here we're checking that the setter contains EXACTLY 1 argument, and the getter - 0
    match kind {
        FieldKind::Getter => {
            if !usr_arg_names.is_empty() {
                return syn_error(
                    name, 
                    format!("Getter {} can't contain more than 2 default arguments", &name_str)
                ).to_compile_error();
            }
        },
        FieldKind::Setter => {
            if usr_arg_names.len() != 1 {
                return syn_error(
                    name, 
                    format!("Setter {} should contain exactly 3 arguments (2 default and 1 user argument)", &name_str)
                ).to_compile_error();
            }
        }
    };

    // It could be concatenated, but I'll probably leave it for readability reasons.

    match kind {
        FieldKind::Getter => {
            quote! {
                fields.add_field_method_get::<_, #return_ty>(
                    stringify!(#name), 
                    |#(#trait_arg_names), *| #block
                );
            }
        },
        FieldKind::Setter => {
            quote! {
                fields.add_field_method_set::<_, (#(#usr_inp_tys), *)>(
                    stringify!(#name), 
                    |#(#trait_arg_names), *, (#(#usr_arg_names), *)| #block
                );
            }
        }
    }
}

/// Expand the impl block. This will overwrite the entire impl block
/// with an implementation of [`mlua::UserData`] + [`mlua_bindgen::AsTable`]
pub fn expand_impl(input: ItemImpl) -> TokenStream2 {
    let mut methods = Vec::new();
    let mut fields = Vec::new();
    let mut funcs = Vec::new();

    // Analyze the impl block
    let impl_name = input.self_ty.to_token_stream();

    // Convert the generated code to a TokenStream
    for itm in input.items.iter() {
        if let ImplItem::Fn(ref func) = itm {
            // I made a mistake myself, because of which I wasted almost half and hour, but basically, this macro doesn't
            // disallow you from using normal functions with #[mlua_bindgen] impl blocks.
            // 
            // This isn't supposed to be an error, rather a tip for some that would stumble on this "unexpected" use.
            // In any case, this variable basically describes if the attribute has any of the required attributes not to be considered
            // "useless"
            let mut has_required_attrs: bool = false;

            for attr in func.attrs.iter() {
                let path: &syn::Path = attr.path();

                // Ignore documentation
                if path.is_ident("doc") { continue };

                // Only assign true if the attribute is one of the specified or just incorrect.
                has_required_attrs = true;

                if path.is_ident("method") {
                    methods.push(parse_impl_function(func, FuncKind::Method));
                } else if path.is_ident("method_mut") {
                    methods.push(parse_impl_function(func, FuncKind::MethodMut));
                } else if path.is_ident("func") {
                    funcs.push(parse_impl_function(func, FuncKind::Func));
                } else if path.is_ident("get") {
                    fields.push(parse_field(func, FieldKind::Getter));
                } else if path.is_ident("set") {
                    fields.push(parse_field(func, FieldKind::Setter));
                } else {
                    return syn_error(
                        attr, 
                        "Incorrect attributes. Only method, method_mut, func or doc can be used"
                    ).to_compile_error();
                }
            }

            if !has_required_attrs {
                return syn_error(
                    itm, 
                    "No attributes? If that's intentional - you should move this function to a normal impl block, since this macro ignores non-attributed functions"
                ).to_compile_error();
            }
        } else {
            panic!("can't use impl items other than functions in export_lua macro")
        }
    }

    quote! {   
        impl ::mlua::UserData for #impl_name {
            fn add_fields<F: ::mlua::UserDataFields<Self>>(fields: &mut F) { 
                #(#fields)*
            }
    
            fn add_methods<M: ::mlua::UserDataMethods<Self>>(methods: &mut M) { 
                #(#methods)*
            }
        }

        impl ::mlua_bindgen::AsTable for #impl_name {
            fn as_table(lua: &::mlua::Lua) -> ::mlua::Result<::mlua::Table> {
                let table = lua.create_table()?;
                #(#funcs)*
                Ok(table)
            }
        }
    }
}