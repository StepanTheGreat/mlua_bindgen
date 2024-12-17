
use macros::mlua_bindgen;
use mlua_bindgen::AsTable;

#[mlua_bindgen]
enum GreatEnum {
    Var1,
    Var2,
    Var4 = 3,
    Var100 = 100,
    Var101,
    Var90 = 90,
    Var91 = 20,
}

#[test]
fn enums() -> mlua::Result<()> {
    let lua = mlua::Lua::new();
    lua.globals().set("GreatEnum", GreatEnum::as_table(&lua)?)?;
    let res = lua.load("
        return GreatEnum.Var1 + GreatEnum.Var2
    ").eval::<u32>()?;

    assert_eq!(res, 1);
    // Var1 is 0, Var2 is 1. So 0+1 = 1

    Ok(())
}