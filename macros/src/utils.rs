use proc_macro2::TokenStream as TokenStream2;
use shared::{funcs::FuncKind, syn_error};
use syn::ImplItemFn;
use quote::quote;

/// Parse an impl function (be it function or method), and convert into registration code.
/// 
/// There's different registration code however. Methods (method and method_mut) are registered into
/// [`mlua::UserData`], while functions are registered as table functions. For more, check [`mlua_bindgen::AsTable`])
pub fn parse_impl_function(item: &ImplItemFn, kind: FuncKind) -> TokenStream2 {
    let exfunc = ExtractedFunc::from_func_info(item, &kind);

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

    // Obviously, a method can't be used if it doesn't have the first 2 required arguments, so we have to panic.
    if trait_arg_names.len() < exfunc.req_arg_count {
        let (func_type, args_fmt) = match kind {
            FuncKind::Func => ("class function", "&Lua"),
            FuncKind::Method => ("method", "&Lua, &Self"),
            FuncKind::MethodMut => ("mutable method", "&Lua, &mut Self")
        };
        let name_str = name.to_string();
        return syn_error(
            name, 
            format!("Not enough arguments for {} \"{}\". It takes {} as its first {} arguments", func_type, &name_str, args_fmt, exfunc.req_arg_count)
        ).to_compile_error();
    }

    // We generate 3 different registration code types, 2 to be used with mlua::UserData.
    // The other one to be used with [`AsTable`]
    match kind {
        FuncKind::MethodMut => {
            quote! { 
                methods.add_method_mut::<_, (#(#usr_inp_tys),*), #return_ty>(
                    stringify!(#name), 
                    |#(#trait_arg_names), *, (#(#usr_arg_names), *)| #block
                ); 
            }
        },
        FuncKind::Method => {
            quote! { 
                methods.add_method::<_, (#(#usr_inp_tys),*), #return_ty>(
                    stringify!(#name), 
                    |#(#trait_arg_names), *, (#(#usr_arg_names), *)| #block
                ); 
            }
        },
        FuncKind::Func => {
            quote! { 
                table.set(
                    stringify!(#name), 
                    lua.create_function::<_, (#(#usr_inp_tys),*), #return_ty>(
                        |#(#trait_arg_names), *, (#(#usr_arg_names), *)| #block
                    )?
                )?; 
            }
        }
    }
}