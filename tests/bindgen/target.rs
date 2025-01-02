mod imported;

use imported::imported_module;

static COUNTER: AtomicU32 = AtomicU32::new(5);

#[mlua_bindgen]
mod super_inner {
    use macros::mlua_bindgen;

    #[mlua_bindgen]
    pub fn add(_: &mlua::Lua, val1: f32, val2: f32) -> f32 {
        Ok(val1 + val2)
    }

    #[mlua_bindgen]
    pub fn subtract(_: &mlua::Lua, val1: f32, val2: f32) -> f32 {
        Ok(val1 - val2)
    }

    #[derive(Clone, Debug, FromLua, PartialEq)]
    pub struct CoolNumber {
        val: f64
    }

    impl CoolNumber {
        pub fn new(val: f64) -> Self {
            Self { val }
        }
    }

    #[mlua_bindgen]
    impl CoolNumber {
        #[func]
        fn new(_: _, val: f64) -> Self {
            Ok(Self::new(val))
        }

        #[get]
        fn value(_: _, this: &Self) -> f32 {
            Ok(this.val as f32)
        }
    }
}

#[mlua_bindgen(include = [super_inner_module])]
mod inner {
    use macros::mlua_bindgen;

    #[mlua_bindgen]
    pub fn mul(_: &mlua::Lua, val1: f32, val2: f32) -> f32 {
        Ok(val1 * val2)
    }

    #[mlua_bindgen]
    pub enum Numbers {
        Num1,
        Num2,
        Num3,
        // Forgot 4?
        Num5 = 5
    }

    /// Adds something to a global counter
    #[mlua_bindgen]
    pub fn do_something(_: &mlua::Lua, what: u32) -> f32 {
        Ok(0.75)
    }
}

#[mlua_bindgen(main, include = [inner_module, imported_module])]
mod main {
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

        #[method]
        fn add(_: _, this: &Self, with: Self) -> Self {
            let res = Self {
                x: this.x + with.x,
                y: this.y + with.y,
            };

            Ok(res)
        }

        #[method]
        fn hello(_: _, this: &Self) {
            // Do something
            Ok(())
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

    #[mlua_bindgen]
    enum GreatEnum {
        Var1,
        Var2,
        Var4 = 3,
        Var100 = 100,
        Var101,
    }

    /// Adds something to a global counter
    #[mlua_bindgen]
    pub fn do_something_better(_: &mlua::Lua, what: u32, other: String) -> [String; 3] {
        Ok(["".to_owned(), "".to_owned(), "".to_owned()])
    }
}

fn main() {
    // Does something
}