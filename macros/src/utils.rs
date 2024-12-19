use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use shared::funcs::FuncArg;

/// Consume the argument vector, and return 3 vectors corresponding to:
/// 1. Required argument names
/// 2. User argument names
/// 3. User argument types
pub fn into_arg_tokens(
    args: Vec<FuncArg>,
) -> (Vec<TokenStream2>, Vec<TokenStream2>, Vec<TokenStream2>) {
    let mut req_arg_names: Vec<TokenStream2> = Vec::new();
    let mut usr_arg_names: Vec<TokenStream2> = Vec::new();
    let mut usr_arg_types: Vec<TokenStream2> = Vec::new();

    for arg in args {
        if arg.required {
            req_arg_names.push(arg.name.into_token_stream());
        } else {
            usr_arg_names.push(arg.name.into_token_stream());
            usr_arg_types.push(arg.ty.into_token_stream());
        }
    }

    (req_arg_names, usr_arg_names, usr_arg_types)
}
