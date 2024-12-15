use proc_macro2::TokenStream as TokenStream2;
use syn::{ItemFn, Visibility};
use quote::quote;

use crate::utils::*;

/// Expand functions. This will overwrite the original function and also add a static type check
/// to ensure that it has proper arguments/return types (by mlua rules of course)
pub fn expand_fn(input: ItemFn) -> TokenStream2 {
    let mut extracted = ExtractedFunc::from_func_info(&input, &FuncKind::Func);
    let name = extracted.name;
    let block = extracted.block;
    let return_ty = extracted.return_ty;
    let usr_arg_names = extracted.user_arg_names;
    let usr_arg_types = extracted.user_arg_types;
    let pub_param = match input.vis {
        Visibility::Public(_) => quote! {pub},
        Visibility::Restricted(_) | Visibility::Inherited => TokenStream2::new()
    };

    // We popped this value to use it separately as the first argument in the function.
    // The mlua call convention looks like this: fn myfunc(lua: &Lua, (arg1, arg2): (Type1, Type2))
    // Thus, we handle this case separately.
    let lua_arg = extracted.trait_arg_names.remove(0);

    quote! {
        #pub_param fn #name(#lua_arg, (#(#usr_arg_names), *): (#(#usr_arg_types), *)) -> ::mlua::Result<#return_ty> #block

        const _:fn(&::mlua::Lua) = |l| {
            _ = l.create_function(#name);
        };
    }
}