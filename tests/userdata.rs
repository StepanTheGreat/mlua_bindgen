use macros::mlua_bindgen;
use mlua_bindgen::AsTable;

pub struct ResId {
    id: u64
}

#[mlua_bindgen]
impl ResId {
    #[func]
    fn new(_: &mlua::Lua, with: u64) -> Self {
        Ok(Self {
            id: with
        })
    }

    #[get]
    fn id(_: &mlua::Lua, this: &Self) -> u64 {
        Ok(this.id)
    }
}

#[test]
fn userdata() -> mlua::Result<()> {
    let lua = mlua::Lua::new();

    lua.globals().set("ResId", ResId::as_table(&lua)?)?;

    let result = lua.load("
        local res_id = ResId.new(127)        

        return res_id.id
    ").eval::<u64>()?;
    assert_eq!(result, 127);

    Ok(())
}