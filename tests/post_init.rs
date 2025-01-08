use mlua::Value;
use mlua_bindgen::mlua_bindgen;

fn modify_module(_: &mlua::Lua, table: &mlua::Table) -> mlua::Result<()> {
    // Let's create a new variable
    table.set("my_secret_key", 256)?;

    // Let's remove this function
    table.set("remove_me", Value::Nil)?;

    Ok(())
}

#[mlua_bindgen(main, post_init=modify_module)]
mod module {
    use mlua_bindgen::mlua_bindgen;

    #[mlua_bindgen]
    pub fn my_function(_: &mlua::Lua) -> u32 {
        Ok(5)
    }

    #[mlua_bindgen]
    pub fn remove_me(_: &mlua::Lua) -> u32 {
        Ok(222)
    }
}

#[test]
fn post_init() -> mlua::Result<()> {
    let lua = mlua::Lua::new();
    lua.globals().set("module", module_module(&lua)?)?;

    lua.load(
        "
        
        -- Call our module function, of course
        assert(module.my_function() == 5)

        -- We removed that function in the post_init function
        assert(module.remove_me == nil)

        -- We set it through our post_init function
        assert(module.my_secret_key == 256)
    ",
    )
    .exec()?;

    Ok(())
}
