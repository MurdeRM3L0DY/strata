// Copyright 2023 the Strata authors
// SPDX-License-Identifier: GPL-3.0-or-later

mod key;
mod modflags;

use piccolo::{
	self as lua,
};

use crate::{
	bindings::CtxExt,
	handlers::input::{
		Key,
		KeyPattern,
		ModFlags,
	},
};

pub fn module<'gc>(ctx: lua::Context<'gc>) -> anyhow::Result<lua::Value<'gc>> {
	let input = lua::Table::new(&ctx);
	input.set(ctx, "Key", key::module(ctx)?)?;
	input.set(ctx, "Mod", modflags::module(ctx)?)?;
	input.set(
		ctx,
		"keybind",
		lua::Callback::from_fn(&ctx, |ctx, _, mut stack| {
			let comp = ctx.comp()?;
			let (mods, key, cb) = stack.consume::<(ModFlags, Key, lua::Function)>(ctx)?;

			let keypat = KeyPattern {
				mods,
				key,
			};

			comp.config().keybinds.insert(keypat, ctx.stash(cb));

			println!("{:#?}: {:#?}", mods, key);

			Ok(lua::CallbackReturn::Return)
		}),
	)?;

	Ok(lua::Value::Table(input))
}
