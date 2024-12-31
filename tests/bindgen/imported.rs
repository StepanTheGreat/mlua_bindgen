#[mlua_bindgen]
pub mod imported {
    use macros::mlua_bindgen;

    #[mlua_bindgen]
    pub fn say_hi(_: &mlua::Lua, to: String) {
        println!("Hi to {to}");
        Ok(())
    }
}