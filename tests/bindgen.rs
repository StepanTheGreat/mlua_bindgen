//! This test is going to parse the `target.rs` file in the bindgen directory, and transform it into
//! luau declaration file.

#[cfg(feature="bindgen")]
use mlua_bindgen::bindgen::BindgenTransformer;
use mlua_bindgen::error::Error;

#[cfg(feature="bindgen")]
#[test]
fn main() -> Result<(), Error>{

    let lua_src = BindgenTransformer::new()
        .add_input_dir("./tests/bindgen")
        .parse()?
        .transform_to_lua()?
        .to_string();

    std::fs::write("test.d.luau", lua_src)?;
    Ok(())
}
