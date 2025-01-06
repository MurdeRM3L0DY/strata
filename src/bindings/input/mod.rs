// Copyright 2023 the Strata authors
// SPDX-License-Identifier: GPL-3.0-or-later

mod key;
mod modifier;

use piccolo::{
	self as lua,
	IntoValue,
};

use crate::{
	bindings::ContextExt,
	handlers::input::{
		Key,
		KeyPattern,
		Modifier,
	},
};

pub fn module<'gc>(ctx: lua::Context<'gc>, comp: lua::UserData<'gc>) -> anyhow::Result<lua::Value<'gc>> {
	let input = lua::Table::new(&ctx);

	input.set_field(ctx, "Key", key::module(ctx, comp)?);
	input.set_field(ctx, "Mod", modifier::module(ctx, comp)?);

	input.set(
		ctx,
		"keybind",
		lua::Callback::from_fn_with(&ctx, comp, |comp, ctx, _, mut stack| {
			let comp = ctx.comp(comp)?;
			let (mods, key, cb) = stack.consume::<(Modifier, Key, lua::Function)>(ctx)?;

			let keypat = KeyPattern {
				modifier: mods,
				key,
			};

			comp.with_mut(|comp| {
				comp.config.keybinds.insert(keypat, ctx.stash(cb));
			});

			println!("{:#?}: {:#?}", mods, key);

			Ok(lua::CallbackReturn::Return)
		}),
	)?;

	Ok(input.into_value(ctx))
}
