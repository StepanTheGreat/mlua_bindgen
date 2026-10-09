use macros::mlua_bindgen;

#[mlua_bindgen(alias = "my_alias")]
type Alias = mlua::Table;

#[mlua_bindgen]
mod test_mod {
    use mlua_bindgen::mlua_bindgen;

    #[mlua_bindgen(alias = "another_alias")]
    type InnerAlias = mlua::Table;
}

#[test]
fn aliases() -> mlua::Result<()> {
    Ok(())
}
