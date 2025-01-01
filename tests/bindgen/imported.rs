struct ConflictingType {
    something: u32
}

fn conflicting_func(_inp: String) {
    println!("I have a conflicting name.");
}

#[mlua_bindgen]
pub mod imported {
    use macros::mlua_bindgen;

    /// This type should be renamed to "ConflictingType" in the generated bindings
    pub struct LuaConflictingType(pub ConflictingType);

    #[mlua_bindgen]
    impl LuaConflictingType {
        #[func]
        pub fn create(_: &mlua::Lua) -> Self {
            Ok(Self(ConflictingType { something: 500 }))
        }   

        #[method]
        pub fn conflict(_: &mlua::Lua, this: &Self) {
            // Idk, doing some conflicting?
            Ok(())
        }
    }

    #[mlua_bindgen]
    pub fn lua_conflicting_func(_: &mlua::Lua, val: String) {
        conflicting_func(val);
        Ok(())
    }
}