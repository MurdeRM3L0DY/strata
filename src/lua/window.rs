use crate::CHANNEL;

use super::ConfigCommands;

fn window_close<'lua>(_: &'lua mlua::Lua, _: mlua::Value<'lua>) -> mlua::Result<()> {
	Ok(())
}

fn window_move<'lua>(_: &'lua mlua::Lua, id: u8) -> mlua::Result<()> {
	let channel = CHANNEL.lock().unwrap();
	channel.sender.send(ConfigCommands::MoveWindow(id)).unwrap();
	Ok(())
}

pub fn lua_module<'lua>(lua: &'lua mlua::Lua) -> mlua::Result<mlua::Table<'lua>> {
	let obj = lua.create_table()?;

	obj.set("close", lua.create_function(window_close)?)?;

	Ok(obj)
}
