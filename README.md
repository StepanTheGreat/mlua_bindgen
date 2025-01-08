# mlua_bindgen
## A proc-macro library and tool that simplifies working with mlua.

This project focuses on procedural macros that abstract most of the boilerplate while using the `mlua`
crate, while also providing a way to automatically generate luau bindings, recognized by luau LSP.

*Note: bindings generation works, but is highly unstable, and most often needs corrections.*
*Feel free to open an issue if you find a bug while using it.*

## Features:
- Functions
- Userdata (i.e. structs implemented using `mlua_bindgen`)
- Type functions (check the examples below for more information)
- Enums (with integer variants)
- Modules (a collection of mlua compatible types, all collected to a table)
- Module inclusion (i.e. an ability to include another mlua module inside a module)
- "Lua" prefix removal (i.e. naming your function/type `LuaType` will result in `Type` name in modules)
- Basic bindgen API (check the issues below)

## A quick example:
```rust
struct MyStruct {
    field: u32
}

impl mlua::UserData for MyStruct {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) { 
        fields.add_field_method_get::<_, u32>("field", |_: &Lua, this: &Self| Ok(this.field));
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {}
}
```

With this macro can also be expressed as:
```rust
struct MyStruct {
    field: u32
}

#[mlua_bindgen]
impl MyStruct {
    #[get]
    fn field(_: &Lua, this: &Self) -> u32 {
        Ok(this.field)
    }

    #[set]
    fn field(_: &Lua, this: &mut Self, new_val: u32) {
        this.field = new_val;
        Ok(())
    }
}
```

## What's supported:

### Functions
```rust
#[mlua_bindgen]
fn cool(_: &mlua::Lua, sm: u32, hi: bool) -> u32 {
   Ok(50)
}
```
### Userdata
```rust
#[mlua_bindgen]
impl MyType {
    #[get]
    fn x(_: _, this: &Self) -> f32 {
        Ok(this.x)
    }

    #[set]
    fn x(_: _, this: &mut Self, to: f32) {
        this.x = to;
        Ok(())
    }

    #[method_mut]
    fn rename(_: _, this: &mut Self, to: &str) {
        this.name = to;
        Ok(())
    }

    #[func]
    fn make_new(_: _, ud: AnyUserData, name: &str) -> Self {
        Ok(Self {
            name
        })
    }
}
```
### Enums
```rust
#[mlua_bindgen]
enum Colors {
    Red,
    Green,
    Blue
}

// Will automatically implement AsTable
let lua = Lua::new();
let lua_enum: Table = Colors::as_table(&lua)?;
// Now it's a lua table:
// Colors = {
//  Red = 0,
//  Green = 1,
//  Blue = 2,
//}
```
### Modules
```rust
fn important() {

}

#[mlua_bindgen]
mod math {
    #[mlua_bindgen]
    pub fn mul(_: &mlua::Lua, val1: f32, val2: f32) -> f32 {
        Ok(val1 * val2)
    }

    /// Auto prefix removal to avoid name collision. In the lua module, this function instead will be
    /// exported as "important", while in rust it's name stays the same
    #[mlua_bindgen]
    pub fn lua_important(_: &mlua::Lua) {
        important();
        Ok(())
    }
}

// You can nest modules. In this example, `math` will be a part of the `utils` module.
// And yes, the same can be done for the `math` module as well, but this is not shown here for simplicity.
#[mlua_bindgen(include = [math_module])]
mod utils {
    #[mlua_bindgen]
    pub fn rust_hello(_: &mlua::Lua, who: String) {
        println!("Hello to {who}");
        Ok(())
    }
}

// This will automatically create a function that will 
// return ALL module items and included modules in a table.

lua.globals().set("utils", utils_module(&lua)?)?;
lua.load('
    utils.rust_hello("Lua!")
').exec()?;
//
// >> Hello to Lua!
//
```

## Compatibility table
| Crate version | `mlua` version |
| ----          | ----           |
| 0.2           | 0.10.1         |

## Some known issues
1. You can't declare modules inside modules (You can connect them though)
2. There's no mechanism against type dublication in bindgen (types in Luau are global scope, meaning that using the same type name in different modules will result in 2 different type declarations) 
3. Module names are required to be unique; the current bindgen implementation simply can't work with modules that
share the same name, since it doesn't understand the crate module tree.

*If you got a bug while using this crate, feel free to file an issue*

## Maintenance
I'm making this crate for a personal project, so I can lose interest in developing/maintaining it at any time.
For now though, I think I should have at least a week of motivation to make *some* work on it.

## Licensing
I'm licensing this crate under both MIT and Apache-2.0, like all the other rust crates.