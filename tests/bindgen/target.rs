mod imported;

use imported::imported_module;

static COUNTER: AtomicU32 = AtomicU32::new(5);

/// This module even though is not ignored, is a child of an ignored module, 
/// so will be ignored as well
#[mlua_bindgen]
mod ignored_inner {
    #[mlua_bindgen]
    pub fn even_sneakier(_: &mlua::Lua) {
        Ok(())
    }
}

/// This module and its items should be ignored
#[mlua_bindgen_ignore]
#[mlua_bindgen(include=[ignored_inner_module])]
mod ignored {

    #[mlua_bindgen]
    pub fn ignored_function(_: &mlua::Lua) {
        Ok(())
    }
}

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
        fn new(_: _, val: mlua::Either<f32, Self>) -> Self {
            Ok(match val {
                mlua::Either::Left(num) => Self::new(val),
                mlua::Either::Right(other) => Self::new(other.val)
            })
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

#[mlua_bindgen(main, include = [
    inner_module, 
    imported_module, 
    ignored_module
]
)]
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

        #[meta]
        fn __add(_: _, this: Self, with: Self) -> Self {
            Ok(Self {
                x: this.x + with.x,
                y: this.x + with.x
            })
        }

        #[meta]
        fn __tostring(_: _, this: Self) -> String {
            Ok(format!("<Vector x={}, y={}>", this.x, this.y))
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

    /// Should return a table of strings
    #[mlua_bindgen]
    pub fn do_something_better(_: &mlua::Lua, what: u32, other: String) -> [String; 3] {
        Ok(["".to_owned(), "".to_owned(), "".to_owned()])
    }

    /// The same
    #[mlua_bindgen]
    pub fn do_something_better_vec(_: &mlua::Lua, what: u32, other: String) -> Vec<String> {
        Ok(vec!["".to_owned(), "".to_owned(), "".to_owned()])
    }

    /// This function should not be in the generated bindings
    #[mlua_bindgen_ignore]
    #[mlua_bindgen]
    pub fn require(_: &mlua::Lua, module: String) -> Table {
        Ok(())
    }
}

fn main() {
    // Does something
}