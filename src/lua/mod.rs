use std::cell::Ref;

pub fn init(lua: Ref<'_, mlua::Lua>) -> mlua::Result<()> {
	let g = lua.globals();
	let path = g.get::<_, mlua::Table>("package")?.get::<_, mlua::Table>("loaded")?;

	let strata = lua.create_table()?;

	Ok(())
}
