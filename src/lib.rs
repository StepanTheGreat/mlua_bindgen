pub use macros::*;

/// A trait that allows registering a type to a table. For example in Roblox, custom types can be
/// registered to scopes using tables:
/// 
/// ```lua
/// 
/// local color         =       Color3.new(0.5, 1, 0.5, 1)
///        ^ Userdata              ^ Table
/// ```
/// 
/// Functions assigned to these tables are constructors. This makes it quite convenient to
/// both document and also construct types.
pub trait AsTable {
    /// Register this type to a specified table (could be for example the `lua.globals()` table)
    fn as_table(lua: &mlua::Lua) -> mlua::Result<mlua::Table>;
}

// A trait for constructing Lua modules. They can also contain other modules inside
// pub trait LuaModule {
//     fn get_module()
// }