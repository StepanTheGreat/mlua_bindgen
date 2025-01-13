use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    punctuated::Punctuated,
    token::{Brace, Comma, Paren},
    Block, FnArg, Ident, ImplItemFn, ItemFn, Pat, ReturnType, Type, TypeTuple, Visibility,
};

use crate::utils::{contains_attr, syn_error, MLUA_IGNORE_BINDGEN_ATTR};

pub struct CommonFuncInfo {
    pub ident: Ident,
    pub bindgen_ignore: bool,
    pub visibility: Visibility,
    pub block: Block,
    pub ret_ty: ReturnType,
    pub args: Punctuated<FnArg, Comma>,
}

/// Simply a getter for functions, since syn separates functions into [`ImplItemFn`] and [`ItemFn`].
/// With this I can minimize the amount of dublicated code
pub trait CommonFunc {
    fn get_info(self) -> CommonFuncInfo;
}

impl CommonFunc for ImplItemFn {
    fn get_info(self) -> CommonFuncInfo {
        CommonFuncInfo {
            ident: self.sig.ident,
            bindgen_ignore: contains_attr(&self.attrs, MLUA_IGNORE_BINDGEN_ATTR),
            visibility: self.vis,
            block: self.block,
            ret_ty: self.sig.output,
            args: self.sig.inputs,
        }
    }
}

impl CommonFunc for ItemFn {
    fn get_info(self) -> CommonFuncInfo {
        CommonFuncInfo {
            ident: self.sig.ident,
            bindgen_ignore: contains_attr(&self.attrs, MLUA_IGNORE_BINDGEN_ATTR),
            visibility: self.vis,
            block: *self.block,
            ret_ty: self.sig.output,
            args: self.sig.inputs,
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
    /// Lua table's meta-method
    Meta,
}

pub struct FuncArg {
    pub name: Pat,
    pub ty: Type,
    /// A required argument is the one that's important for mlua API. Usually it's a `&Lua` reference, or
    /// `&Self` / `&mut Self` in methods.
    pub required: bool,
}

impl ToTokens for FuncArg {
    fn into_token_stream(self) -> proc_macro2::TokenStream
    where
        Self: Sized,
    {
        let (name, ty) = (self.name, self.ty);
        quote! { #name: #ty }
    }

    fn to_token_stream(&self) -> proc_macro2::TokenStream {
        let (name, ty) = (&self.name, &self.ty);
        quote! { #name: #ty }
    }

    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let (name, ty) = (&self.name, &self.ty);
        tokens.extend(quote! { #name: #ty });
    }
}

/// The absolute extracted function. The huge difference between this and [`FuncInfo`] is that the latter contains
/// only the most basic information about the function. This contains additional information like user argument types,
/// user argument names, and so on.
pub struct ParsedFunc {
    pub name: Ident,
    pub bindgen_ignore: bool,
    pub visibility: Visibility,
    pub block: Block,
    pub args: Vec<FuncArg>,
    pub return_ty: Type,
}

impl ParsedFunc {
    /// Create an empty parsed function from name.
    ///
    /// Only use this inside macro, since it only cares about the name
    pub fn from_ident(name: Ident) -> Self {
        Self {
            name,
            bindgen_ignore: false,
            visibility: Visibility::Inherited,
            block: Block {
                brace_token: Brace::default(),
                stmts: Vec::new(),
            },
            args: Vec::new(),
            return_ty: Type::Verbatim(TokenStream::new()),
        }
    }
}

impl ParsedFunc {
    /// Get the amount of user arguments
    pub fn user_arg_count(&self) -> usize {
        self.args.iter().filter(|arg| !arg.required).count()
    }

    /// Get the amount of required arguments
    pub fn req_arg_count(&self) -> usize {
        self.args.iter().filter(|arg| arg.required).count()
    }
}

pub fn parse_func(item: impl CommonFunc, kind: &FuncKind) -> syn::Result<ParsedFunc> {
    let info = item.get_info();
    let name = info.ident;
    let bindgen_ignore = info.bindgen_ignore;
    let block = info.block;
    let visibility = info.visibility;

    // Signature output returns both the return type and the arrow ("-> u32", as example), so we filter it with
    // the match statement here, and convert to tokens
    let return_ty = match info.ret_ty {
        ReturnType::Default => Type::Tuple(TypeTuple {
            paren_token: Paren::default(),
            elems: Punctuated::new(),
        }),
        ReturnType::Type(_, ty) => *ty,
    };

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

    let mut args: Vec<FuncArg> = Vec::new();
    let req_arg_count = match kind {
        FuncKind::Method | FuncKind::MethodMut => 2, // Lua + Self
        FuncKind::Meta => 1,                         // Lua
        FuncKind::Func => 1,                         // Lua
    };
    for (ind, inp_ty) in info.args.into_iter().enumerate() {
        let is_required = ind < req_arg_count;

        let (pat, ty) = match inp_ty {
            FnArg::Receiver(_) => return Err(syn_error(inp_ty, "Can't contain the self argument")),
            FnArg::Typed(ty) => (ty.pat, ty.ty),
        };

        args.push(FuncArg {
            name: *pat,
            ty: *ty,
            required: is_required,
        });
    }

    if args.len() < req_arg_count {
        let args_fmt = match kind {
            FuncKind::Func => "&Lua",
            FuncKind::Method => "&Lua, &Self",
            FuncKind::MethodMut => "&Lua, &mut Self",
            FuncKind::Meta => "&Lua, &impl FromLua",
        };
        return Err(syn_error(
            name,
            format!(
                "Not enough arguments. It should take {} as its first {} arguments",
                args_fmt, req_arg_count
            ),
        ));
    }

    Ok(ParsedFunc {
        name,
        bindgen_ignore,
        visibility,
        block,
        return_ty,
        args,
    })
}
