use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::{parse, Item};

mod funcs;
mod impls;
mod utils;
mod enums;
mod mods;

use funcs::expand_fn;
use impls::expand_impl;
use enums::expand_enum;
use mods::expand_mod;
use utils::macro_error;

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
/// ```
/// ### UserData
/// ```
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
/// ### Enums
/// ```
/// #[mlua_bindgen]
/// enum Colors {
///     Red,
///     Green,
///     Blue
/// }
///
/// // Will automatically implement AsTable
/// let lua = Lua::new();
/// let lua_enum: Table = Colors::as_table(&lua)?;
/// // Now it's a lua table:
/// // Colors = {
/// //  Red = 0,
/// //  Green = 1,
/// //  Blue = 2,
/// //}
/// ```
/// ### Modules
/// ```rust
/// #[mlua_bindgen]
/// mod math {
///     #[mlua_bindgen]
///     pub fn mul(_: &mlua::Lua, val1: f32, val2: f32) -> f32 {
///         Ok(val1 * val2)
///     }
/// }
///
// // You can nest modules. In this example, `math` will be a part of the `utils` module.
// // And yes, the same can be done for the `math` module as well, but this is not shown here for simplicity.
/// #[mlua_bindgen(include = [math_module])]
/// mod utils {
///     #[mlua_bindgen]
///     pub fn rust_hello(_: &mlua::Lua, who: String) {
///         println!("Hello to {who}");
///         Ok(())
///     }
/// }
///
/// // This will automatically create a function that will 
/// // return ALL module items and included modules in a table.  
///
/// lua.globals().set("utils", utils_module(&lua)?)?;
/// lua.load('
///     utils.rust_hello("Lua!")
/// ').exec()?;
/// //
/// // >> Hello to Lua!
/// //
/// ```
#[proc_macro_attribute]
pub fn mlua_bindgen(attr: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse::<Item>(input.clone()).expect("Failed to parse the item");
    // Some items require original input, so we keep it as well
    let input = TokenStream2::from(input);

    match item {
        Item::Impl(item) => expand_impl(item),
        Item::Fn(item) => expand_fn(item),
        Item::Enum(item) => expand_enum(input, item),
        Item::Mod(item) => expand_mod(attr, input, item),
        Item::Struct(item) => {
            macro_error(
                item, 
                "If you want to implement a custom UserData type, you should put this macro on an impl block instead"
            )
        },
        _ => {
            macro_error(
                item, 
                "This macro can only be used on Impl blocks, Functions, Enums and Mod blocks"
            )
        }
    }.into()
}

