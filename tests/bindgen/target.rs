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

fn main() {
    // Does something
}