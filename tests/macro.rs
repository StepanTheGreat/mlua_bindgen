use mlua_bindgen::{mlua_bindgen, AsTable};
use mlua::Function;
use mlua::FromLua;

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

#[mlua_bindgen]
fn new(_: &mlua::Lua) -> ResId {
    Ok(ResId { id: 0 })
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

// #[derive(FromLua)]
#[mlua_bindgen]
enum GreatEnum {
    Var1,
    Var2
}

#[test]
fn main() {
    let lua = mlua::Lua::new();
    let func = lua.create_function(cool_fn).unwrap();
    let res = func.call::<u32>((32, true)).unwrap();
    assert_eq!(res, 50);

    lua.globals().set("ResId", ResId::as_table(&lua).unwrap()).unwrap();
    lua.globals().set("GreatEnum", GreatEnum::as_table(&lua).unwrap()).unwrap();
    // ResId::as_table(&lua, &lua.globals()).unwrap();

    let result = lua.load("
        local res_id = ResId.new(50)
        assert(GreatEnum.Var1, 0)
        

        return res_id.id
    ").eval::<u64>().unwrap();
    assert_eq!(result, 50);
}