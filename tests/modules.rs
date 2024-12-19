use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;

use math::Vector;
use mlua_bindgen::{mlua_bindgen, AsTable};

static COUNTER: AtomicU32 = AtomicU32::new(5);

#[mlua_bindgen]
mod inner {
    use macros::mlua_bindgen;

    #[mlua_bindgen]
    pub fn mul(_: &mlua::Lua, val1: f32, val2: f32) -> f32 {
        Ok(val1 * val2)
    }
}

#[mlua_bindgen(include = [inner_module])]
mod math {
    use std::sync::atomic::Ordering;

    use mlua::FromLua;
    use mlua_bindgen::mlua_bindgen;

    use crate::COUNTER;

    // We're using FromLua here to allow Vector use Self in its methods/functions
    #[derive(Clone, Debug, FromLua, PartialEq)]
    pub struct Vector {
        x: f32,
        y: f32,
    }

    impl Vector {
        pub fn new(x: f32, y: f32) -> Self {
            Self { x, y }
        }
    }

    /// A Vector object! Quite nice!
    #[mlua_bindgen]
    impl Vector {
        #[func]
        fn new(_: _, x: f32, y: f32) -> Self {
            Ok(Self::new(x, y))
        }

        //
        #[method]
        fn add(_: _, this: &Self, with: Self) -> Self {
            let res = Self {
                x: this.x + with.x,
                y: this.y + with.y,
            };

            Ok(res)
        }

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

    /// Adds something to a global counter
    #[mlua_bindgen]
    pub fn add_to_counter(_: &mlua::Lua, what: u32) {
        COUNTER.fetch_add(what, Ordering::Relaxed);

        Ok(())
    }
}

#[test]
fn modules() -> mlua::Result<()> {
    let lua = mlua::Lua::new();
    lua.globals().set("math", math_module(&lua)?)?;

    let res = lua
        .load(
            "
        -- Add value to the counter
        math.add_to_counter(38)

        -- Using inner modules
        local vec1_x = math.inner.mul(2, 5)

        -- Add 2 vectors together
        local Vector = math.Vector
        local vec1 = Vector.new(50, 35)
        local vec2 = Vector.new(-3, vec1_x)
        return vec1:add(vec2)
    ",
        )
        .eval::<Vector>()?;
    assert_eq!(res, Vector::new(47.0, 45.0));
    assert_eq!(COUNTER.load(Ordering::Relaxed), 43);

    Ok(())
}
