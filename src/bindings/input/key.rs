// Copyright 2023 the Strata authors
// SPDX-License-Identifier: GPL-3.0-or-later

use piccolo::{
	self as lua,
	FromValue,
	IntoValue,
};

use crate::handlers::input::Key;

pub fn module<'gc>(ctx: lua::Context<'gc>, comp: lua::UserData<'gc>) -> anyhow::Result<lua::Value<'gc>> {
	let meta = lua::Table::from_value(ctx, Key::metatable(ctx)?)?;

	let keys = lua::Table::new(&ctx);
	keys.set_metatable(&ctx, Some(meta));

	Ok(keys.into_value(ctx))
}
