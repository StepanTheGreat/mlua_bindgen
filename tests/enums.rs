use macros::mlua_bindgen;

#[allow(dead_code)]
#[mlua_bindgen]
#[derive(Debug, PartialEq)]
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

    // We're also going to test convertations back from unsigned integers
    assert_eq!(GreatEnum::from_usize(100), Some(GreatEnum::Var100));
    assert_eq!(GreatEnum::from_usize(102), None);

    Ok(())
}
