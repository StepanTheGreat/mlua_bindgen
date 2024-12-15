# mlua_bindgen
## A proc-macro library and tool that simplifies working with mlua.

This project focuses on procedural macros that abstract most of the boilerplate while using the `mlua`
crate, while also providing a way to automatically generate luau bindings, recognized by luau LSP.

(Currently the bindings don't work)

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
#[mlua_bindgen]
mod utils {
    #[mlua_bindgen]
    pub fn rust_hello(_: &mlua::Lua, who: String) {
        println!("Hello to {who}");
        Ok(())
    }
}

// This will automatically create a function that will 
// return ALL module items (besides for other modules) in a table.  

lua.globals().set("utils", utils_module(&lua)?)?;
lua.load('
    utils.rust_hello("Lua!")
').exec()?;
//
// >> Hello to Lua!
//
```

## TODO
- Module connection. Basically, creating modules is simple, but uniting them into larger modules (that 
recursively contain other modules) is a bit difficult to think about. Especially since this tool then has
to analyze these modules, and make bindings for them. Furthermore, it will be difficult to distinguish entry_point 
modules and inner modules.
- Bindings generation. This is supposed to analyze specified rust files for marked `mlua_bindgen` attributes,
collect neccessary information (type names, documentation, variable names, variable types, ...) and transform
into a bindings file that luau-lsp can understand.

## Some issues
1. You can't declare modules inside modules
2. There's no way to connect modules, for now you have to manually connect them

## Maintenance
I'm making this crate for a personal project, so I can lose interest in developing/maintaining it at any time.
For now though, I think I should have at least a week of motivation to make *some* work on it.

## Licensing
I'm licensing this crate under both MIT and Apache-2.0, like all the other rust crates.