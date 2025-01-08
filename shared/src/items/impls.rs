use syn::{ImplItem, ImplItemFn, ItemImpl, Type};

use crate::utils::{contains_attr, syn_error};

use super::funcs::{parse_func, FuncKind, ParsedFunc};

/// An enum used to distinguish between setters and getters. When parsing these, the only way to distinguish
/// them is to look at their attribute. Functions that parse fields can take this enum to apply custom rules:
///
/// For example, a getter can't contain any arguments, while a setter can only contain one.
pub enum FieldKind {
    Getter,
    Setter,
}

impl FieldKind {
    pub const REQUIRED_ARGS: usize = 2;

    /// Tells the amount of user arguments each field can take
    pub fn user_args(&self) -> usize {
        match self {
            Self::Getter => 0,
            Self::Setter => 1,
        }
    }
}

pub struct ParsedField {
    pub func: ParsedFunc,
    pub kind: FieldKind,
}

pub struct ParsedImplFunc {
    pub func: ParsedFunc,
    pub kind: FuncKind,
}

pub struct ParsedImpl {
    /// Impl blocks don't contain Ident tokens, but rather type
    pub name: Type,
    pub fields: Vec<ParsedField>,
    pub funcs: Vec<ParsedImplFunc>,
    pub methods: Vec<ParsedImplFunc>,
    pub meta_funcs: Vec<ParsedImplFunc>
}

impl ParsedImpl {
    /// Construct an absolutely empty impl, with just name information
    pub fn from_ty(name: Type) -> Self {
        Self {
            name,
            fields: Vec::new(),
            funcs: Vec::new(),
            methods: Vec::new(),
            meta_funcs: Vec::new()
        }
    }
}

/// Parse an impl block and its inner functions into a [`ParsedImpl`]
pub fn parse_impl(input: ItemImpl) -> syn::Result<ParsedImpl> {
    let name = input.self_ty;
    let mut fields: Vec<ParsedField> = Vec::new();
    let mut methods: Vec<ParsedImplFunc> = Vec::new();
    let mut funcs: Vec<ParsedImplFunc> = Vec::new();
    let mut meta_funcs: Vec<ParsedImplFunc> = Vec::new();

    for impl_item in input.items {
        if let ImplItem::Fn(impl_fn) = impl_item {
            if contains_attr(&impl_fn.attrs, "method") {
                methods.push(parse_impl_func(impl_fn, FuncKind::Method)?);
            } else if contains_attr(&impl_fn.attrs, "method_mut") {
                methods.push(parse_impl_func(impl_fn, FuncKind::MethodMut)?);
            } else if contains_attr(&impl_fn.attrs, "func") {
                funcs.push(parse_impl_func(impl_fn, FuncKind::Func)?);
            } else if contains_attr(&impl_fn.attrs, "meta") {
                meta_funcs.push(parse_impl_func(impl_fn, FuncKind::Meta)?);
            } else if contains_attr(&impl_fn.attrs, "get") {
                fields.push(parse_field(impl_fn, FieldKind::Getter)?);
            } else if contains_attr(&impl_fn.attrs, "set") {
                fields.push(parse_field(impl_fn, FieldKind::Setter)?);
            } else {
                return Err(syn_error(impl_fn, "No attributes? If that's intentional - you should move this function to a normal impl block, since this macro ignores non-attributed functions"));
            }
        }
    }

    Ok(ParsedImpl {
        name: *name,
        fields,
        methods,
        funcs,
        meta_funcs
    })
}

/// Parse a lua [`UserData`] field into a [`ParsedField`]
pub fn parse_field(input: ImplItemFn, kind: FieldKind) -> syn::Result<ParsedField> {
    let func = match parse_func(input, &FuncKind::MethodMut) {
        Ok(func) => func,
        Err(err) => return Err(err),
    };

    let user_arg_count = func.user_arg_count();

    // Here we're checking that the setter contains EXACTLY 1 user argument, and the getter - 0
    if user_arg_count != kind.user_args() {
        let msg = match kind {
            FieldKind::Getter => {
                "Getters can't contain more than 2 default arguments (&Lua and &Self)"
            }
            FieldKind::Setter => {
                "Setter have to contain exactly 3 arguments (&Lua, &mut Self and 1 user argument)"
            }
        };
        return Err(syn_error(func.name, msg));
    }

    Ok(ParsedField { func, kind })
}

/// Parse a lua [`UserData`] method/function into a [`ParsedImplFunc`]
pub fn parse_impl_func(input: ImplItemFn, kind: FuncKind) -> syn::Result<ParsedImplFunc> {
    let func = parse_func(input, &kind)?;
    Ok(ParsedImplFunc { func, kind })
}
