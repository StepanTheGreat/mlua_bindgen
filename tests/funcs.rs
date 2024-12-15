use macros::mlua_bindgen;

#[mlua_bindgen]
pub fn cool_fn(_: &mlua::Lua, _: u32, _: bool) -> u32 {
    Ok(50)
}

#[test]
fn functions() -> mlua::Result<()> {
    let lua = mlua::Lua::new();
    let func = lua.create_function(cool_fn)?;
    let res = func.call::<u32>((32, true))?;
    assert_eq!(res, 50);

    Ok(())
}