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

    /// Test documentation 1
    /// 
    /// Test documentation 2
    #[mlua_bindgen]
    pub fn ignored_function(_: &mlua::Lua) {
        Ok(())
    }
}

#[mlua_bindgen]
mod super_inner {
    use macros::mlua_bindgen;

    /// Test documentation 1
    /// 
    /// Test documentation 2
    #[mlua_bindgen]
    pub fn add(_: &mlua::Lua, val1: f32, val2: f32) -> f32 {
        Ok(val1 + val2)
    }

    /// Test documentation 1
    /// 
    /// Test documentation 2
    #[mlua_bindgen]
    pub fn subtract(_: &mlua::Lua, val1: f32, val2: f32) -> f32 {
        Ok(val1 - val2)
    }

    /// Test documentation 1
    /// 
    /// Test documentation 2
    #[derive(Clone, Debug, FromLua, PartialEq)]
    pub struct CoolNumber {
        val: f64
    }

    impl CoolNumber {
        pub fn new(val: f64) -> Self {
            Self { val }
        }
    }

    /// Test documentation 1
    /// 
    /// Test documentation 2
    #[mlua_bindgen]
    impl CoolNumber {
        /// Test documentation 1
        /// 
        /// Test documentation 2
        #[func]
        fn new(_: _, val: mlua::Either<f32, Self>) -> Self {
            Ok(match val {
                mlua::Either::Left(num) => Self::new(val),
                mlua::Either::Right(other) => Self::new(other.val)
            })
        }

        /// Test documentation 1
        /// 
        /// Test documentation 2
        #[get]
        fn value(_: _, this: &Self) -> f32 {
            Ok(this.val as f32)
        }
    }
}

/// Test documentation 1
/// 
/// Test documentation 2
#[mlua_bindgen(include = [super_inner_module])]
mod inner {
    use macros::mlua_bindgen;

    /// Test documentation 1
    /// 
    /// Test documentation 2
    #[mlua_bindgen]
    pub fn mul(_: &mlua::Lua, val1: f32, val2: f32) -> f32 {
        Ok(val1 * val2)
    }

    /// Test documentation 1
    /// 
    /// Test documentation 2
    #[mlua_bindgen]
    pub enum Numbers {
        /// Great
        Num1,
        Num2,

        /// Here's a detailed
        /// explanation
        /// 
        /// of `why`
        /// 
        /// This is great
        Num3,
        /// Forgot 4?
        Num5 = 5
    }

    /// Adds something to a global counter
    #[mlua_bindgen]
    pub fn do_something(_: &mlua::Lua, one: mlua::Buffer, two: mlua::Vector) -> f32 {
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
        /// New vector!
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

        #[method_mut]
        fn hello_mut(_: _, this: &mut Self) {
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
    pub fn do_something_better(_: &mlua::Lua, what: GreatEnum, other: String) -> [String; 3] {
        Ok(["".to_owned(), "".to_owned(), "".to_owned()])
    }

    /// The same
    #[mlua_bindgen]
    pub fn do_something_better_vec(_: &mlua::Lua, what: u32, other: String) -> Vec<String> {
        Ok(vec!["".to_owned(), "".to_owned(), "".to_owned()])
    }

    #[mlua_bindgen]
    pub fn more_values(_: &mlua::Lua, what: Option<u32>, other: Option<Vec<u32>>) -> (u32, Vec<String>) {
        Ok((20, "".to_owned()))
    }

    #[mlua_bindgen]
    pub fn more_values_maps(_: &mlua::Lua, what: Option<Vec<HashMap<u32, Vec<String>>>>) -> (u32, Vec<String>) {
        Ok((20, vec!["".to_owned()]))
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