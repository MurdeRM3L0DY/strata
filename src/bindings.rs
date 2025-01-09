// Copyright 2023 the Strata authors
// SPDX-License-Identifier: GPL-3.-or-later

use anyhow::Context as _;
use piccolo::{
	self as lua,
	FromValue as _,
};

use crate::state::FrozenCompositor;

mod input;
mod proc;

trait ContextExt<'gc> {
	fn comp(self, ud: lua::UserData<'gc>) -> anyhow::Result<&'gc FrozenCompositor>;
}

impl<'gc> ContextExt<'gc> for lua::Context<'gc> {
	fn comp(self, ud: lua::UserData<'gc>) -> anyhow::Result<&'gc FrozenCompositor> {
		Ok(ud.downcast_static::<FrozenCompositor>()?)
	}
}

pub fn create<'gc>(ctx: lua::Context<'gc>, comp: &FrozenCompositor) -> anyhow::Result<lua::UserData<'gc>> {
	let comp = lua::UserData::new_static(&ctx, comp.clone());

	let index = lua::Table::new(&ctx);

	index.set(ctx, "input", input::module(ctx, comp)?)?;
	index.set(ctx, "proc", proc::module(ctx, comp)?)?;
	index.set(
		ctx,
		"quit",
		lua::Callback::from_fn_with(&ctx, comp, |&comp, ctx, _, _| {
			let comp = ctx.comp(comp)?;

			comp.with(|comp| {
				comp.quit();
			});

			Ok(lua::CallbackReturn::Return)
		}),
	)?;

	let meta = lua::Table::new(&ctx);
	meta.set_field(ctx, lua::MetaMethod::Index.name(), index);
	comp.set_metatable(&ctx, Some(meta));

	Ok(comp)
}

pub fn get_str_from_value<'gc>(ctx: lua::Context<'gc>, value: lua::Value<'gc>) -> anyhow::Result<&'gc str> {
	Ok(value.into_string(ctx).context("Invalid `string`")?.to_str()?)
}
