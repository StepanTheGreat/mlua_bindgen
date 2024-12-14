use proc_macro::TokenStream;
use syn::{
    parse_macro_input, Item
};

mod funcs;
mod impls;
mod utils;

use funcs::expand_fn;
use impls::expand_impl;

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