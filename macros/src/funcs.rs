use proc_macro2::TokenStream as TokenStream2;
use shared::{funcs::{parse_func, FuncKind}, syn_error};
use syn::ItemFn;
use quote::{quote, ToTokens};

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

    if parsed.args.is_empty() {
        return syn_error(
            name,
            "A lua function has to contain at least a single &Lua argument"
        ).into_compile_error();
    }

    let mut usr_arg_names: Vec<TokenStream2> = Vec::new();
    let mut usr_arg_types: Vec<TokenStream2> = Vec::new();

    // The lua argument is added separately from user arguments, so we pop it and add it separately.
    let lua_arg = parsed.args.remove(0).into_token_stream();

    for arg in parsed.args {
        usr_arg_names.push(arg.name.into_token_stream());
        usr_arg_types.push(arg.ty.into_token_stream());
    }

    quote! {
        #pub_param fn #name(#lua_arg, (#(#usr_arg_names), *): (#(#usr_arg_types), *)) -> ::mlua::Result<#return_ty> #block

        const _:fn(&::mlua::Lua) = |l| {
            _ = l.create_function(#name);
        };
    }
}