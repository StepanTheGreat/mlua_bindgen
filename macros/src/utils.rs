use std::fmt::Display;

use proc_macro2::{Span, TokenStream as TokenStream2};
use shared::syn_error;
use syn::{
    parse_quote, punctuated::Punctuated, token::Comma, Attribute, FnArg, ImplItemFn, ItemFn
};
use quote::{quote, ToTokens};

pub const MLUA_BINDGEN_ATTR: &str = "mlua_bindgen";

/// The absolute extracted function. The huge difference between this and [`FuncInfo`] is that the latter contains
/// only the most basic information about the function. This contains additional information like user argument types,
/// user argument names, and so on.
pub struct ExtractedFunc {
    pub name: TokenStream2,
    pub block: TokenStream2,
    pub return_ty: TokenStream2,
    /// User input types. Basically a list of types that user uses as an argument.
    /// 
    /// Ex: `fn(a: u32, b: f32)` will be `[u32, f32]`
    pub user_arg_types: Vec<TokenStream2>,
    /// User argument names. This is a list of user defined arguments for functions    
    ///
    /// Ex: `fn(a: u32, b: f32)` will be `["a", "b"]`
    pub user_arg_names: Vec<TokenStream2>,
    /// Trait required argument names. A user can rename default arguments (like &[`mlua::Lua`]), hence we're storing them separately
    pub trait_arg_names: Vec<TokenStream2>,
    /// The amount of required first arguments
    pub req_arg_count: usize
}

/// To avoid dublication, we represent the same information of both ImplItemFn and ItemFn in a single data struct.
/// This way the extraction is in the same place, and we don't need separate methods in the [`ExtractedFunc`] implementation.
pub struct FuncInfo<'a> {
    pub name: TokenStream2,
    pub block: TokenStream2,
    pub return_ty: TokenStream2,
    pub inputs: &'a Punctuated<FnArg, Comma>
}

/// A simple trait to simplify common function information extraction.
pub trait ExtractFuncInfo {
    fn get_info<'a>(&'a self) -> FuncInfo<'a>;
}

impl ExtractFuncInfo for ImplItemFn {
    fn get_info<'a>(&'a self) -> FuncInfo<'a> {
        FuncInfo {
            name: self.sig.ident.to_token_stream(),
            block: self.block.to_token_stream(),
            return_ty: match self.sig.output.clone() {
                syn::ReturnType::Type(_, ty) => *ty,
                syn::ReturnType::Default => parse_quote!{ () },
            }.to_token_stream(),
            inputs: &self.sig.inputs
        }
    }
}

impl ExtractFuncInfo for ItemFn {
    fn get_info<'a>(&'a self) -> FuncInfo<'a> {
        FuncInfo {
            name: self.sig.ident.to_token_stream(),
            block: self.block.to_token_stream(),
            return_ty: match self.sig.output.clone() {
                syn::ReturnType::Type(_, ty) => *ty,
                syn::ReturnType::Default => parse_quote!{ () },
            }.to_token_stream(),
            inputs: &self.sig.inputs
        }
    }
}

impl ExtractedFunc {
    pub fn from_func_info(item: &impl ExtractFuncInfo, kind: &FuncKind) -> Self {
        let info = item.get_info();
        let name = info.name; 
        let block = info.block; // Get the method's code block
    
        // Signature output returns both the return type and the arrow ("-> u32", as example), so we filter it with
        // the match statement here, and convert to tokens
        let return_ty = info.return_ty;
        
        // A Lua UserData method requires these arguments:
        // - &Lua
        // - &Self
        // - (... args) 
        // Since users can define their own argument names for their method (i.e. _: &Lua), we only collect argument names
        // for the first 2 required arguments.
        // For the other user arguments (if present), we only care about their type for the generics arguments,
        // so we push them into a separate vector.
        // We also store user argument names, to paste them into a scoped function like so:
        // fn func(aa, b) {}  ==>  |(aa, b)| {}
    
        let mut user_arg_types: Vec<TokenStream2> = Vec::new();
        let mut user_arg_names: Vec<TokenStream2> = Vec::new();
        let mut trait_arg_names: Vec<TokenStream2> = Vec::with_capacity(2);
        let req_arg_count = match kind {
            FuncKind::Method | FuncKind::MethodMut => 2,
            FuncKind::Func => 1
        };
        for (ind, inp_ty) in info.inputs.iter().enumerate() {
            if ind < req_arg_count {
                trait_arg_names.push(inp_ty.to_token_stream());
            } else {
                // Extract the pattern (variable name) and type name
                let (pat, ty): (TokenStream2, TokenStream2) = match inp_ty {
                    FnArg::Receiver(_) => panic!("Can't contain the self argument"),
                    FnArg::Typed(ty) => {
                        (ty.pat.to_token_stream(), ty.ty.to_token_stream())
                    }
                };

                user_arg_names.push(pat);
                user_arg_types.push(ty);
            }
        };

        Self { 
            name, 
            block, 
            return_ty, 
            user_arg_types, 
            user_arg_names, 
            trait_arg_names,
            req_arg_count,
        } 
    }
}

/// A simple enum representing 3 possible function types. Basically the same explanation as with the [`FieldKind`]
pub enum FuncKind {
    /// Lua method
    Method,
    /// Lua mutable method (can mutate self)
    MethodMut,
    /// Lua class method
    Func,
}

/// An enum used to distinguish between setters and getters. When parsing these, the only way to distinguish
/// them is to look at their attribute. Functions that parse fields can take this enum to apply custom rules:
/// 
/// For example, a getter can't contain any arguments, while a setter can only contain one.
pub enum FieldKind {
    Getter,
    Setter
}

/// Parse an impl function (be it function or method), and convert into registration code.
/// 
/// There's different registration code however. Methods (method and method_mut) are registered into
/// [`mlua::UserData`], while functions are registered as table functions. For more, check [`mlua_bindgen::AsTable`])
pub fn parse_function(item: &ImplItemFn, kind: FuncKind) -> TokenStream2 {
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