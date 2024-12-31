//! This test is going to parse the `target.rs` file in the bindgen directory, and transform it into
//! luau declaration file.

use mlua_bindgen::bindgen::BindgenTransformer;

#[test]
fn main() {
    let lua_src = BindgenTransformer::new()
        .add_input_dir("tests\\bindgen")
        .parse()
        .unwrap()
        .transform_to_lua()
        .unwrap()
        .to_string();

    std::fs::write("test.d.luau", lua_src).unwrap();
}
