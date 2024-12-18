use std::fmt::format;

use quote::ToTokens;
use syn::{punctuated::Punctuated, token::{Comma, Do}, Attribute, Expr, FnArg, ImplItem, Lit, MetaNameValue, ReturnType};

use crate::{ExtractedFunc, ExtractedStructItem, FuncArg, LuaType};

/// Simply iterates over item attributes and checks if it has the [`mlua_bindgen`] attribute
pub fn has_bindgen_attr(attrs: &[Attribute]) -> bool {
    for attr in attrs {
        if attr.path().is_ident("mlua_bindgen") {
            return true
        }
    }
    false
}

/// Try to extract a documentation string from the attribute list. Returns an [`Option`]
/// 
/// Since an item can contain multiple lines of docstring, we iterate over ALL doc attributes,
/// concatenate a string and then return it in its full
pub fn extract_doc(attrs: &[Attribute]) -> Option<String> {
    let mut res_str = String::new();
    for attr in attrs {
        if attr.path().is_ident("doc") {
            let doc_str = &attr.meta.require_name_value().unwrap().value;
            if let Expr::Lit(lit) = doc_str {
                if let Lit::Str(litstr) = &lit.lit {
                    let newline = if res_str.len() != 0 {"\n"}  else {""}; 
                    res_str.push_str(&(newline.to_owned()+&litstr.value()));
                }
                
            }
        }
    }

    if res_str.len() > 0 { Some(res_str) } else { None };
}

/// A bit redundant type, but for now let it be a separate enum.
pub enum StructItem {
    Getter,
    Setter,
    Func,
    Method,
}

/// Check if which type the item is through its attributes. It returns an Option, meaning
/// that some items simply don't contain anything.
pub fn which_struct_item(attrs: &[Attribute]) -> Option<StructItem> {
    for attr in attrs {
        let p = attr.path();
        if p.is_ident("func") {
            return Some(StructItem::Func);    
        } else if p.is_ident("method") {
            return Some(StructItem::Method);
        } else if p.is_ident("get") {
            return Some(StructItem::Getter);
        } else if p.is_ident("set") {
            return Some(StructItem::Setter);
        } 
    }
    None
}

/// Iterates over function arguments, and converts them into a [`FuncArg`] vector
pub fn get_func_args(f_args: &Punctuated<FnArg, Comma>) -> Vec<FuncArg> {
    let mut args: Vec<FuncArg> = Vec::new();
    for (ind, arg) in f_args.iter().enumerate() {
        if ind == 0 {
            // The first argument is always Lua, we ignore it
            continue;
        }

        match arg {
            FnArg::Receiver(_) => unreachable!("mlua_bindgen doesn't accept self arguments"),
            FnArg::Typed(pat_ty) => {
                let arg_name = pat_ty.pat.to_token_stream().to_string();
                let arg_ty = LuaType::from_type(*pat_ty.ty.clone());
                args.push(FuncArg { 
                    name: arg_name, 
                    ty: arg_ty 
                });
            }
        }
    }
    args
}

/// Transforms [`ReturnType`] into a [`LuaType`]
pub fn get_ret_type(output: &ReturnType) -> LuaType {
    match output {
        ReturnType::Default => LuaType::Nil,
        ReturnType::Type(_, ty) => LuaType::from_type((**ty).clone())
    }
}

pub fn stringify_func(func: ExtractedFunc) -> String {
    let name = func.name;
    let doc = func.doc.unwrap_or_default();
    let ret = func.ret.to_string();
    format!("
    --[[
    {doc}
    ]]
    function m.{name}(): {ret}
    end
    ")
}