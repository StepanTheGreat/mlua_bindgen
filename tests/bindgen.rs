//! This test is going to parse the `target.rs` file in the bindgen directory, and transform it into
//! luau declaration file.

#[cfg(feature = "bindgen")]
use mlua_bindgen::bindgen;

#[cfg(feature = "bindgen")]
#[test]
fn main() {
    bindgen::load_file("./tests/bindgen/target.rs")
        .unwrap()
        .transform_to_lua()
        .unwrap()
        .write("test.d.luau");
}
