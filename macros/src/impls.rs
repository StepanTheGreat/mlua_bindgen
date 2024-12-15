use proc_macro2::TokenStream as TokenStream2;
use syn::{
    ImplItem, ImplItemFn, ItemImpl
};
use quote::{quote, ToTokens};

use crate::utils::*;

/// This will parse the supplied impl function (and its [`FieldKind`]), extract neccessary information,
/// then transform into a field registration code for mlua. 
pub fn parse_field(item: &ImplItemFn, kind: FieldKind) -> TokenStream2 {
    // This is completely unsound, but for now that's our workaround
    let exfunc = ExtractedFunc::from_func_info(item, &FuncKind::MethodMut);
    
    let (usr_inp_tys, return_ty, name, trait_arg_names, usr_arg_names, block) = {
        (
            exfunc.user_arg_types,
            exfunc.return_ty,
            exfunc.name,
            exfunc.trait_arg_names,
            exfunc.user_arg_names,
            exfunc.block
        )
    };

    let name_str = name.to_string(); // This is used mostly for errors

    if trait_arg_names.len() < exfunc.req_arg_count {
        let (field_type, args_fmt) = match kind {
            FieldKind::Getter => ("getter", "&Lua, &Self"),
            FieldKind::Setter => ("setter", "&Lua, &mut Self"),
        };
        return macro_error(
            name, 
            format!("Not enough arguments for {} \"{}\". It takes {} as its first {} arguments.", field_type, &name_str, args_fmt, exfunc.req_arg_count)
        );
    }

    // Here we're checking that the setter contains EXACTLY 1 argument, and the getter - 0
    match kind {
        FieldKind::Getter => {
            if usr_arg_names.len() > 0 {
                return macro_error(
                    name, 
                    format!("Getter {} can't contain more than 2 default arguments.", &name_str)
                );
            }
        },
        FieldKind::Setter => {
            if usr_arg_names.len() != 1 {
                return macro_error(
                    name, 
                    format!("Setter {} should contain exactly 3 arguments (2 default and 1 user argument).", &name_str)
                );
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
            for attr in func.attrs.iter() {
                let path: &syn::Path = attr.path();
                if path.is_ident("doc") {
                    // Ignore documentation
                    continue;
                } else if path.is_ident("method") {
                    methods.push(parse_function(&func, FuncKind::Method));
                } else if path.is_ident("method_mut") {
                    methods.push(parse_function(&func, FuncKind::MethodMut));
                } else if path.is_ident("func") {
                    funcs.push(parse_function(&func, FuncKind::Func));
                } else if path.is_ident("get") {
                    fields.push(parse_field(&func, FieldKind::Getter));
                } else if path.is_ident("set") {
                    fields.push(parse_field(&func, FieldKind::Setter));
                } else {
                    return macro_error(
                        attr, 
                        "Incorrect attributes. Only method, method_mut, func or doc can be used"
                    );
                }
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
    }.into()
}