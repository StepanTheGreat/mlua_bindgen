# TODO
- Use [userdata](https://docs.rs/mlua/latest/mlua/struct.Lua.html#method.create_proxy) proxies for custom types (lol)
- Heavy documentation. The entire library is poorly documented, so I think I should spend a fair amount
of time on documenting everything even better.
- Bindings generation somewhat works, but needs a huge overwrite (extremely bad written).
- Support more types with generics (i.e. `Vec<u8>` -> `{number}` and so on)
after macro generation)
- #[bindgen_ignore] tag for excluding methods from participating in bindgen (useful when you overwrite default functions like `require`)
- Simple generics with bindgen. For example: 
```rust
#[mlua_bindgen]
fn do_something<T: FromLua>(_: &mlua::Lua, val: T) -> T {
    ...
}
```
⇩ ⇩ ⇩
```lua
declare function do_something<T>(val: T): T;
```