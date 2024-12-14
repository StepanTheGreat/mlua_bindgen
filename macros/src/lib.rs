use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::{
    parse_macro_input, parse_quote, punctuated::Punctuated, 
    token::Comma, FnArg, 
    ImplItem, ImplItemFn, Item, ItemFn, ItemImpl, ItemMod, Visibility
};
use quote::{quote, ToTokens};

struct ExtractedFunc {
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
struct FuncInfo<'a> {
    name: TokenStream2,
    block: TokenStream2,
    return_ty: TokenStream2,
    inputs: &'a Punctuated<FnArg, Comma>
}

trait ExtractFuncInfo {
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
    fn from_func_info(item: &impl ExtractFuncInfo, kind: &FuncKind) -> Self {
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
                    FnArg::Receiver(_) => panic!("Can't contain the self argument."),
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

/// A simple enum representing 3 possible combinations for Lua UserData functions
enum FuncKind {
    /// Lua method
    Method,
    /// Lua mutable method (can mutate self)
    MethodMut,
    /// Lua class method
    Func,
}

/// A simple enum representing 2 types of attribute methods for Lua custom UserData types
enum FieldKind {
    Getter,
    Setter
}

/// Parses the lua method and generates code to automatically register it for the custom UserData struct.
fn parse_function(item: &ImplItemFn, kind: FuncKind) -> TokenStream2 {
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
        return syn::Error::new_spanned(
            name, 
            format!("Not enough arguments for {} \"{}\". It takes {} as its first {} arguments.", func_type, &name_str, args_fmt, exfunc.req_arg_count)
        ).into_compile_error();
    }

    // We generate 2 different register codes, for 2 different mutability types.
    match kind {
        FuncKind::MethodMut => {
            quote! { methods.add_method_mut::<_, (#(#usr_inp_tys),*), #return_ty>(stringify!(#name), |#(#trait_arg_names), *, (#(#usr_arg_names), *)| #block); }
        },
        FuncKind::Method => {
            quote! { methods.add_method::<_, (#(#usr_inp_tys),*), #return_ty>(stringify!(#name), |#(#trait_arg_names), *, (#(#usr_arg_names), *)| #block); }
        },
        FuncKind::Func => {
            quote! { 
                table.set::<String, mlua::Function>(stringify!(#name), lua.create_function::<_,_,_>(|#(#trait_arg_names), *, (#(#usr_arg_names), *)| #block)?); 
            }

            //quote! { methods.add_function::<_, (#(#usr_inp_tys),*), #return_ty>(stringify!(#name), |#(#trait_arg_names), *, (#(#usr_arg_names), *)| #block); }
        }
    }
}

fn parse_field(item: &ImplItemFn, kind: FieldKind) -> TokenStream2 {
    // This is completely unsound, but for now that's our workaround
    let exfunc = ExtractedFunc::from_func_info(item, &FuncKind::MethodMut);

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
    let name_str = name.to_string();

    if trait_arg_names.len() < exfunc.req_arg_count {
        let (field_type, args_fmt) = match kind {
            FieldKind::Getter => ("getter", "&Lua, &Self"),
            FieldKind::Setter => ("setter", "&Lua, &mut Self"),
        };
        return syn::Error::new_spanned(
            name, 
            format!("Not enough arguments for {} \"{}\". It takes {} as its first {} arguments.", field_type, &name_str, args_fmt, exfunc.req_arg_count)
        ).into_compile_error();
    }

    // Here we're checking that the setter contains EXACTLY 1 argument, and the getter - 0
    if let FieldKind::Getter = kind  {
        if usr_arg_names.len() > 0 {
            return syn::Error::new_spanned(
                name, 
                format!("Getter {} can't contain more than 2 default arguments.", &name_str)
            ).into_compile_error();
        }
    } else if let FieldKind::Setter = kind {
        if usr_arg_names.len() != 1 {
            return syn::Error::new_spanned(
                name, 
                format!("Setter {} should contain exactly 3 arguments (2 default and 1 user argument).", &name_str)
            ).into_compile_error();
        }
    }

    match kind {
        FieldKind::Getter => {
            quote! {
                fields.add_field_method_get::<_, #return_ty>(stringify!(#name), |#(#trait_arg_names), *| #block);
            }
        },
        FieldKind::Setter => {
            quote! {
                fields.add_field_method_set::<_, (#(#usr_inp_tys), *)>(stringify!(#name), |#(#trait_arg_names), *, (#(#usr_arg_names), *)| #block);
            }
        }
    }
}

/// Expand the impl block. This will overwrite the entire impl block
/// with an implementation of [`mlua::UserData`].
fn expand_impl(input: ItemImpl) -> TokenStream2 {
    let mut methods = Vec::new();
    let mut fields = Vec::new();
    let mut funcs = Vec::new();

    // Analyze the `impl` block
    let impl_name = input.self_ty.to_token_stream();

    // Convert the generated code to a TokenStream
    for itm in input.items.iter() {
        if let ImplItem::Fn(ref func) = itm {
            for attr in func.attrs.iter() {
                let p = attr.path();
                if p.is_ident("doc") {
                    // Ignore the documentation
                    continue;
                } else if p.is_ident("method") {
                    methods.push(parse_function(&func, FuncKind::Method));
                } else if p.is_ident("method_mut") {
                    methods.push(parse_function(&func, FuncKind::MethodMut));
                } else if p.is_ident("func") {
                    funcs.push(parse_function(&func, FuncKind::Func));
                } else if p.is_ident("get") {
                    fields.push(parse_field(&func, FieldKind::Getter));
                } else if p.is_ident("set") {
                    fields.push(parse_field(&func, FieldKind::Setter));
                } else {
                    return syn::Error::new_spanned(
                        attr, 
                        "Incorrect attributes. Only method, method_mut, func or doc can be used"
                    ).into_compile_error().into();
                }
            }
        } else {
            panic!("can't use impl items other than functions in export_lua macro")
        }
    }
    let methods = methods.iter();
    
    if funcs.len() == 0 {
        quote! {
            impl mlua::UserData for #impl_name {
                fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) { 
                    #(#fields)*
                }
        
                fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) { 
                    #(#methods)*
                }
            }
        }
    } else {
        quote! {
            impl mlua::UserData for #impl_name {
                fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) { 
                    #(#fields)*
                }
        
                fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) { 
                    #(#methods)*
                }
            }
    
            // impl mlua_bindgen::UserDataTable for #impl_name {
            //     fn as_table(lua: &mlua::Lua) -> mlua::Result<mlua::Table> {
            //         let table = lua.create_table()?;
            //         #(#funcs)*
    
            //         Ok(table)
            //     }
            // }
        }
    }.into()
}

/// Expand functions. This doesn't overwrite anything, rather adds an IntoLua check for the function
/// to ensure that it accepts and returns correct arguments.
fn expand_fn(input: ItemFn) -> TokenStream2 {
    let mut extracted = ExtractedFunc::from_func_info(&input, &FuncKind::Func);
    let name = extracted.name;
    let block = extracted.block;
    let return_ty = extracted.return_ty;
    let usr_arg_names = extracted.user_arg_names;
    let usr_arg_types = extracted.user_arg_types;
    let pub_param = match input.vis {
        Visibility::Public(_) => quote! {pub},
        Visibility::Restricted(_) | Visibility::Inherited => quote! {}
    };

    let lua_arg = extracted.trait_arg_names.remove(0);

    quote! {
        #pub_param fn #name(#lua_arg, (#(#usr_arg_names), *): (#(#usr_arg_types), *)) -> mlua::Result<#return_ty> #block

        const _:fn(&mlua::Lua) = |l| {
            _ = l.create_function(#name);
        };
    }
}

/// # mlua_bindgen
/// A generative attribute macro and also bindgen marker that can transform rust items (like impl blocks/functions) into mlua acceptible structures.
/// It basically removes boilerplate code from type registration, while also serving role as a marker for generating lua declaration types.
/// 
/// ## An example:
/// ```
/// struct MyStruct {
///     field: u32
/// }
/// 
/// impl mlua::UserData for MyStruct {
///     fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) { 
///         fields.add_field_method_get::<_, u32>("field", |_: &Lua, this: &Self| Ok(this.field));
///     }
///
///     fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {}
/// }
///```
/// 
/// With this macro can also be expressed as:
/// ```
/// struct MyStruct {
///     field: u32
/// }
/// 
/// #[mlua_bindgen]
/// impl MyStruct {
///     #[get]
///     fn field(_: &Lua, this: &Self) -> u32 {
///         Ok(this.field)
///     }
/// 
///     #[set]
///     fn field(_: &Lua, this: &mut Self, new_val: u32) {
///         this.field = new_val;
///         Ok(())
///     }
/// }
/// ```
/// 
/// ## What's supported:
/// 
/// ### Functions
/// ```
/// #[mlua_bindgen]
/// fn cool(_: &mlua::Lua, sm: u32, hi: bool) -> u32 {
///    Ok(50)fn has_bindgen_attr(attrs: &[syn::Attribute]) -> bool {
///     for attr in attrs {
///         if attr.path().is_ident("mlua_bindgen") {
///             return true
///         }
///     }
///     false
/// }
/// #[mlua_bindgen]
/// impl MyType {
///     #[get]
///    fn x(_: _, this: &Self) -> f32 {
///        Ok(this.x)
///    }
///
///    #[set]
///    fn x(_: _, this: &mut Self, to: f32) {
///        this.x = to;
///        Ok(())fn has_bindgen_attr(attrs: &[syn::Attribute]) -> bool {
///     for attr in attrs {
///         if attr.path().is_ident("mlua_bindgen") {
///             return true
///         }
///     }
///     false
/// }
///    }
///
///    #[method_mut]
///    fn rename(_: _, this: &mut Self, to: &str) {
///        this.name = to;
///        Ok(())
///    }
/// 
///    #[func]
///    fn make_new(_: _, ud: AnyUserData, name: &str) -> Self {
///        Ok(Self {
///            name
///        })
///    }
/// }
/// ```
#[proc_macro_attribute]
pub fn mlua_bindgen(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let item = {
        let _input = input.clone();
        parse_macro_input!(_input as Item)
    };

    match item {
        Item::Impl(item) => expand_impl(item),
        Item::Fn(item) => expand_fn(item),
        _ => syn::Error::new_spanned(
            item, 
            "This macro can only be used on Impl blocks and Functions."
        ).into_compile_error()
    }.into()
}