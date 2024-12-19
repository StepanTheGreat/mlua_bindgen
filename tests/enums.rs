use macros::mlua_bindgen;
use mlua_bindgen::AsTable;

#[mlua_bindgen]
enum GreatEnum {
    Var1,
    Var2,
    Var4 = 3,
    Var100 = 100,
    Var101,
}

#[test]
fn enums() -> mlua::Result<()> {
    let lua = mlua::Lua::new();
    lua.globals().set("GreatEnum", GreatEnum::as_table(&lua)?)?;
    lua.load(
        "
        assert(GreatEnum.Var1 == 0)
        assert(GreatEnum.Var2 == 1)
        assert(GreatEnum.Var4 == 3)
        assert(GreatEnum.Var100 == 100)
        assert(GreatEnum.Var101 == 101)
    ",
    )
    .exec()?;

    Ok(())
}
