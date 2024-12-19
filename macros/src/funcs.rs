use proc_macro2::TokenStream as TokenStream2;
use shared::funcs::{parse_func, FuncKind};
use syn::ItemFn;
use quote::{quote, ToTokens};

use crate::utils::into_arg_tokens;

/// Expand functions. This will overwrite the original function and also add a static type check
/// to ensure that it has proper arguments/return types (by mlua rules of course)
pub fn expand_fn(input: ItemFn) -> TokenStream2 {
    let mut parsed = match parse_func(input, &FuncKind::Func) {
        Ok(parsed) => parsed,
        Err(err) => return err.to_compile_error()
    };

    let name = parsed.name;
    let block = parsed.block;
    let return_ty = parsed.return_ty;
    let pub_param = parsed.visibility;

    // The lua argument is added separately from user arguments, so we pop it and add it separately.
    let lua_arg = parsed.args.remove(0).into_token_stream();
    let (
        _,
        user_arg_names,
        user_arg_types
    ) = into_arg_tokens(parsed.args);

    quote! {
        #pub_param fn #name(#lua_arg, (#(#user_arg_names), *): (#(#user_arg_types), *)) -> ::mlua::Result<#return_ty> #block

        const _:fn(&::mlua::Lua) = |l| {
            _ = l.create_function(#name);
        };
    }
}