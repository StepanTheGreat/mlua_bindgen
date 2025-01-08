use macros::mlua_bindgen;
use mlua::FromLua;
use mlua_bindgen::AsTable;

#[derive(FromLua, Clone)]
pub struct ResId {
    id: u64,
}

#[mlua_bindgen]
impl ResId {
    #[func]
    fn new(_: &mlua::Lua, with: u64) -> Self {
        Ok(Self { id: with })
    }

    #[get]
    fn id(_: &mlua::Lua, this: &Self) -> u64 {
        Ok(this.id)
    }

    #[method]
    fn do_something(_: &mlua::Lua, _this: &Self) {
        Ok(())
    }

    #[method_mut]
    fn do_something_mut(_: &mlua::Lua, _this: &mut Self) {
        Ok(())
    }

    #[meta]
    fn __add(_: &mlua::Lua, a: Self, b: Self) -> Self {
        Ok(Self {id: a.id+b.id })
    }

    #[meta]
    fn __eq(_: &mlua::Lua, this: Self, val: Self) -> bool {
        Ok(this.id == val.id)
    }

    #[meta]
    fn __tostring(_: &mlua::Lua, this: Self) -> String {
        Ok(format!("<ResId {}>", this.id))
    }
}

#[test]
fn userdata() -> mlua::Result<()> {
    let lua = mlua::Lua::new();

    lua.globals().set("ResId", ResId::as_table(&lua)?)?;

    lua.load("
        -- Assert inner value setters and getters
        local res_id = ResId.new(127) 
        assert(res_id.id == 127)

        -- Check if the __tostring works correctly
        assert(tostring(res_id) == '<ResId 127>')

        local a = ResId.new(50)
        local b = ResId.new(25)
        local c = ResId.new(75)


        -- Add two userdata together
        a = a + b
        
        -- Compare two userdata together
        assert(a == c)
        assert(a.id == c.id)
    ").exec()?;

    Ok(())
}
