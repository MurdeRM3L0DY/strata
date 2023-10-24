mod parse;

use std::path::PathBuf;

use mlua::{
	FromLua,
	Function,
	RegistryKey,
	Table,
	Value, Lua, chunk,
};
use smart_default::SmartDefault;
use strata_core::UpdateFromLua;
use strata_derive::Config;
use strum::EnumString;

use crate::LUA;

use self::parse::StrataApi;

#[derive(Debug, Default, Config)]
pub struct Config {
	pub autostart: Vec<Cmd>,
	pub general: General,
	pub decorations: WindowDecorations,
	pub tiling: Tiling,
	pub animations: Animations,
	pub bindings: Vec<Keybinding>,
	// #[config(from = from_lua::Rules)]
	pub rules: Vec<Rule>,
}

#[derive(Debug)]
pub struct LuaFunction {
	pub(crate) key: RegistryKey,
}

impl LuaFunction {
	pub fn call<'lua>(&'lua self, lua: &'lua mlua::Lua) -> anyhow::Result<()> {
		lua.registry_value::<Function>(&self.key)?.call(0)?;
		Ok(())
	}
}

impl<'lua> FromLua<'lua> for LuaFunction {
	fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> mlua::Result<Self> {
		Ok(Self { key: lua.create_registry_value(Function::from_lua(value, lua)?)? })
	}
}

impl<'lua> UpdateFromLua<'lua> for LuaFunction {
	fn update_from_lua(&mut self, value: Value<'lua>, lua: &'lua Lua) -> mlua::Result<()> {
		self.key = lua.create_registry_value(Function::from_lua(value, lua)?)?;
		Ok(())
	}
}

#[derive(Debug, Default)]
pub(super) struct Rules {
	pub list: Vec<Rule>,
}

impl Rules {
	pub fn add_sequence(&mut self, rules: Table, lua: &Lua) -> mlua::Result<()> {
		for value in rules.sequence_values::<Table>() {
			let value = value?;
			if value.contains_key("triggers")? {
				self.list.push(Rule::from_lua(Value::Table(value), lua)?);
			} else {
				self.add_sequence(value, lua)?;
			}
		}

		Ok(())
	}
}

impl<'lua> FromLua<'lua> for Rules {
	fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> mlua::Result<Self> {
		let mut ret = Rules::default();

		ret.add_sequence(Table::from_lua(value, lua)?, lua)?;

		Ok(ret)
	}
}

impl Into<Vec<Rule>> for Rules {
	fn into(self) -> Vec<Rule> {
		self.list
	}
}

pub type Cmd = Vec<String>;

#[derive(Debug, SmartDefault, Config)]
pub struct General {
	#[default(10)]
	pub workspaces: u8,
	#[default(5)]
	pub gaps_in: i32,
	#[default(10)]
	pub gaps_out: i32,
	pub kb_repeat: Vec<i32>,
}

#[derive(Debug, Default, Config)]
pub struct WindowDecorations {
	pub border: Border,
	pub window: Window,
	pub blur: Blur,
	pub shadow: Shadow,
}

#[derive(Debug, SmartDefault, Config)]
pub struct Border {
	pub enable: bool,
	#[default(2)]
	pub width: u32,
	#[default("#ffffff")]
	pub active: String,
	#[default("#888888")]
	pub inactive: String,
	#[default(5.0)]
	pub radius: f64,
}

#[derive(Debug, SmartDefault, Config)]
pub struct Window {
	#[default(1.0)]
	pub opacity: f64,
}

#[derive(Debug, SmartDefault, Config)]
pub struct Blur {
	pub enable: bool,
	#[default(5)]
	pub size: u32,
	#[default(1)]
	pub passes: u32,
	#[default(true)]
	pub optimize: bool,
}

#[derive(Debug, SmartDefault, Config)]
pub struct Shadow {
	pub enable: bool,
	#[default(5)]
	pub size: u32,
	#[default(5)]
	pub blur: u32,
	#[default("#000000")]
	pub color: String,
}

#[derive(Debug, Default, Config)]
pub struct Tiling {
	pub layout: Layout,
}

#[derive(Debug, Default, EnumString, Config)]
#[strum(serialize_all = "snake_case")]
pub enum Layout {
	#[default]
	Dwindle,
}

#[derive(Debug, SmartDefault, Config)]
pub struct Animations {
	#[default(true)]
	pub enable: bool,
}

#[derive(Debug, Config)]
pub struct Keybinding {
	#[config(flat)]
	pub keys: Vec<String>,
	#[config(flat)]
	pub action: LuaFunction,
}

#[derive(Debug, Config)]
pub struct Rule {
	#[config(flat)]
	pub triggers: Vec<Trigger>,
	#[config(flat)]
	pub action: LuaFunction,
}

#[derive(Debug, Config)]
pub struct Trigger {
	#[config(flat)]
	pub event: String,
	#[config(flat)]
	pub class_name: Option<String>,
	#[config(flat)]
	pub workspace: Option<i32>,
}

impl Config {
	pub fn set(&mut self, config: Config) {
		*self = config;
	}
}

pub fn parse_config(config_dir: PathBuf, lib_dir: PathBuf) -> mlua::Result<()> {
	let lua = LUA.lock();
	let api_submod = get_or_create_module(&lua, "strata.api").unwrap(); // TODO: remove unwrap

	api_submod.set("close_window", lua.create_function(StrataApi::close_window)?)?;
	api_submod.set("switch_to_ws", lua.create_function(StrataApi::switch_to_ws)?)?;
	api_submod.set("move_window", lua.create_function(StrataApi::move_window)?)?;
	api_submod
		.set("move_window_and_follow", lua.create_function(StrataApi::move_window_and_follow)?)?;
	api_submod.set("quit", lua.create_function(StrataApi::quit)?)?;
	api_submod.set("spawn", lua.create_async_function(StrataApi::spawn)?)?;
	api_submod.set("set_config", lua.create_function(StrataApi::set_config)?)?;
	api_submod.set("get_config", lua.create_function(StrataApi::get_config)?)?;
	api_submod.set("update_config", lua.create_function(StrataApi::update_config)?)?;

	let config_path = config_dir.to_string_lossy();
	let lib_path = lib_dir.to_string_lossy();

	lua.load(chunk!(
		local paths = {
			$config_path .. "?.lua",
			$config_path .. "?/init.lua",
			$lib_path .. "/strata/?.lua",
			$lib_path .. "/?/init.lua",
		}
		for _, path in ipairs(paths) do
			package.path = path .. ";" .. package.path
		end

		require("config")
	))
	.exec()?;

	Ok(())
}

fn get_or_create_module<'lua>(lua: &'lua Lua, name: &str) -> anyhow::Result<mlua::Table<'lua>> {
	let loaded: Table = lua.globals().get::<_, Table>("package")?.get("loaded")?;
	let module = loaded.get(name)?;

	match module {
		Value::Nil => {
			let module = lua.create_table()?;
			loaded.set(name, module.clone())?;
			Ok(module)
		}
		Value::Table(table) => Ok(table),
		wat => {
			anyhow::bail!(
				"cannot register module {name} as package.loaded.{name} is already set to a value \
				 of type {type_name}",
				type_name = wat.type_name()
			)
		}
	}
}
