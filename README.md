# mlua_bindgen
## A proc-macro library and a tool that generates luau type definition files.

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
   Ok(50)fn has_bindgen_attr(attrs: &[syn::Attribute]) -> bool {
    for attr in attrs {
        if attr.path().is_ident("mlua_bindgen") {
            return true
        }
    }
    false
}
#[mlua_bindgen]
impl MyType {
    #[get]
   fn x(_: _, this: &Self) -> f32 {
       Ok(this.x)
   }

   #[set]
   fn x(_: _, this: &mut Self, to: f32) {
       this.x = to;
       Ok(())fn has_bindgen_attr(attrs: &[syn::Attribute]) -> bool {
    for attr in attrs {
        if attr.path().is_ident("mlua_bindgen") {
            return true
        }
    }
    false
}
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

## Maintenance
I'm making this crate for a personal project, so I can lose interest in developing/maintaining it at any time.
For now though, I think I should have at least a week of motivation to make *some* work on it.

## Licensing
I'm licensing this crate under both MIT and Apache-2.0, like all the other rust crates.