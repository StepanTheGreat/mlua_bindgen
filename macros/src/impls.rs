use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use shared::{
    funcs::FuncKind,
    impls::{parse_impl, FieldKind, ParsedField, ParsedImplFunc},
};
use syn::ItemImpl;

use crate::utils::into_arg_tokens;

/// This will parse the supplied impl function (and its [`FieldKind`]), extract neccessary information,
/// then transform into a field registration code for mlua.
pub fn expand_field(input: ParsedField) -> TokenStream2 {
    let (func, kind) = (input.func, input.kind);

    let name = &func.name;
    let block = &func.block;
    let return_ty = &func.return_ty;

    let (req_arg_names, user_arg_names, user_arg_types) = into_arg_tokens(func.args);

    // It could be concatenated, but I'll probably leave it for readability reasons.
    match kind {
        FieldKind::Getter => quote! {
            fields.add_field_method_get::<_, #return_ty>(
                stringify!(#name),
                |#(#req_arg_names), *| #block
            );
        },
        FieldKind::Setter => quote! {
            fields.add_field_method_set::<_, (#(#user_arg_types), *)>(
                stringify!(#name),
                |#(#req_arg_names), *, (#(#user_arg_names), *)| #block
            );
        },
    }
}

pub fn expand_impl_func(input: ParsedImplFunc) -> TokenStream2 {
    let (func, kind) = (input.func, input.kind);

    let name = &func.name;
    let block = &func.block;
    let return_ty = &func.return_ty;

    let (req_arg_names, user_arg_names, user_arg_types) = into_arg_tokens(func.args);

    // It could be concatenated, but I'll probably leave it for readability reasons.
    match kind {
        FuncKind::Func => quote! {
            table.set(
                stringify!(#name),
                lua.create_function::<_, (#(#user_arg_types),*), #return_ty>(
                    |#(#req_arg_names), *, (#(#user_arg_names), *)| #block
                )?
            )?;
        },
        FuncKind::Method => quote! {
            methods.add_method::<_, (#(#user_arg_types),*), #return_ty>(
                stringify!(#name),
                |#(#req_arg_names), *, (#(#user_arg_names), *)| #block
            );
        },
        FuncKind::MethodMut => quote! {
            fields.add_method_mut::<_, (#(#user_arg_types), *)>(
                stringify!(#name),
                |#(#req_arg_names), *, (#(#user_arg_names), *)| #block
            );
        },
    }
}

/// Expand the impl block. This will overwrite the entire impl block
/// with an implementation of [`mlua::UserData`] + [`mlua_bindgen::AsTable`]
pub fn expand_impl(input: ItemImpl) -> TokenStream2 {
    let parsed_impl = match parse_impl(input) {
        Ok(parsed) => parsed,
        Err(err) => return err.into_compile_error(),
    };
    let impl_name = parsed_impl.name;

    let fields: Vec<TokenStream2> = parsed_impl.fields.into_iter().map(expand_field).collect();

    let funcs: Vec<TokenStream2> = parsed_impl
        .funcs
        .into_iter()
        .map(expand_impl_func)
        .collect();

    let methods: Vec<TokenStream2> = parsed_impl
        .methods
        .into_iter()
        .map(expand_impl_func)
        .collect();

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
