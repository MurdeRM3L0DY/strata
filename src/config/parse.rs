use crate::{
	lua::ConfigCommands,
	CHANNEL,
	CONFIG,
	LUA,
};
use mlua::{
	chunk,
	FromLua,
	Lua,
	Result,
	Table,
	Value,
};
use std::path::PathBuf;
use strata_core::UpdateFromLua;

pub struct StrataApi;

impl StrataApi {
	pub async fn spawn<'lua>(lua: &'lua Lua, cmd: Value<'lua>) -> Result<()> {
		let cmd: Vec<String> = FromLua::from_lua(cmd, lua)?;

		// TODO: add log

		let channel = CHANNEL.lock().unwrap();
		channel.sender.send(ConfigCommands::Spawn(cmd.join(" "))).unwrap();

		Ok(())
	}

	pub fn switch_to_ws<'lua>(lua: &'lua Lua, id: Value<'lua>) -> Result<()> {
		let id: u8 = FromLua::from_lua(id, lua)?;

		// TODO: add log

		let channel = CHANNEL.lock().unwrap();
		channel.sender.send(ConfigCommands::SwitchWS(id)).unwrap();

		Ok(())
	}

	pub fn move_window<'lua>(lua: &'lua Lua, id: Value<'lua>) -> Result<()> {
		let id: u8 = FromLua::from_lua(id, lua)?;

		// TODO: add log

		let channel = CHANNEL.lock().unwrap();
		channel.sender.send(ConfigCommands::MoveWindow(id)).unwrap();

		Ok(())
	}

	pub fn move_window_and_follow<'lua>(lua: &'lua Lua, id: Value<'lua>) -> Result<()> {
		let id: u8 = FromLua::from_lua(id, lua)?;

		// TODO: add log

		let channel = CHANNEL.lock().unwrap();
		channel.sender.send(ConfigCommands::MoveWindowAndFollow(id)).unwrap();

		Ok(())
	}

	pub fn close_window<'lua>(_lua: &'lua Lua, _: Value<'lua>) -> Result<()> {
		let channel = CHANNEL.lock().unwrap();
		channel.sender.send(ConfigCommands::CloseWindow).unwrap();

		Ok(())
	}

	pub fn quit<'lua>(_lua: &'lua Lua, _: Value<'lua>) -> Result<()> {
		let channel = CHANNEL.lock().unwrap();
		channel.sender.send(ConfigCommands::Quit).unwrap();

		Ok(())
	}

	pub fn set_config(lua: &Lua, config: Value) -> Result<()> {
		CONFIG.write().set(FromLua::from_lua(config, lua)?);

		Ok(())
	}

	pub fn get_config(_lua: &Lua, _args: Value) -> Result<()> {
		// TODO
		unimplemented!()
	}

	pub fn update_config(lua: &Lua, args: Value) -> Result<()> {
		CONFIG.write().update_from_lua(args, lua)
	}
}
