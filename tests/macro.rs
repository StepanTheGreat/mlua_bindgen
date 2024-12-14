use mlua_bindgen::mlua_bindgen;
use mlua::Function;

pub struct ResId {
    id: u128
}

#[mlua_bindgen]
impl ResId {
    #[func]
    fn new(_: &mlua::Lua) -> Self {
        Ok(Self {
            id: 0
        })
    }
}

/// This function does pretty cool things!
/// Sounds pretty cool to me
#[mlua_bindgen]
pub fn cool_fn(l: &mlua::Lua, sm: u32, hi: bool) -> u32 {
    Ok(50)
}

pub struct Vector {
    x: f32,
    y: f32
}

/// This function does pretty cool things!
/// Sounds pretty cool to me
#[mlua_bindgen]
fn cool(l: &mlua::Lua, sm: u32, hi: bool) -> Function {
    Ok(l.create_function::<_, _, _>(|_, ()| {Ok(())}).unwrap())
}

/// A Vector object! Quite nice!
#[mlua_bindgen]
impl Vector {
    #[get]
    fn x(_: _, this: &Self) -> f32 {
        Ok(this.x)
    }

    #[get]
    fn y(_: _, this: &Self) -> f32 {
        Ok(this.y)
    }

    #[set]
    fn x(_: _, this: &mut Self, to: f32) {
        this.x = to;
        Ok(())
    }

    #[set]
    fn y(_: _, this: &mut Self, to: f32) {
        this.y = to;
        Ok(())
    }
}

#[test]
fn main() {
    let l = mlua::Lua::new();
    let func = l.create_function(cool_fn).unwrap();
    let res = func.call::<u32>((32, true)).unwrap();
    assert_eq!(res, 50);
}