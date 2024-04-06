// Copyright 2023 the Strata authors
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
	cell::RefCell,
	process::Command,
	rc::Rc,
};

use piccolo::{
	self as lua,
	FromValue,
};

use crate::state::Compositor;

trait CtxExt<'gc> {
	fn comp(self) -> anyhow::Result<&'gc Rc<RefCell<StrataComp>>>;
}

impl<'gc> CtxExt<'gc> for lua::Context<'gc> {
	fn comp(self) -> anyhow::Result<&'gc Rc<RefCell<StrataComp>>> {
		let comp = lua::UserData::from_value(self, self.globals().get(self, "strata"))?;
		Ok(comp.downcast_static::<Rc<RefCell<StrataComp>>>()?)
	}
}

pub mod input;

pub fn metatable<'gc>(ctx: lua::Context<'gc>) -> anyhow::Result<lua::Table<'gc>> {
	let index = lua::Table::new(&ctx);
	index.set(ctx, "input", input::module(ctx)?)?;
	index.set(
		ctx,
		"spawn",
		lua::Callback::from_fn(&ctx, |ctx, _, mut stack| {
			let (cmd, _) = stack.consume::<(lua::String, lua::Value)>(ctx)?;
			let _ = Command::new(cmd.to_str()?).spawn()?;

			Ok(lua::CallbackReturn::Return)
		}),
	)?;
	index.set(
		ctx,
		"quit",
		lua::Callback::from_fn(&ctx, |ctx, _, _| {
			let comp = ctx.comp()?;
			comp.borrow().quit();

			Ok(lua::CallbackReturn::Return)
		}),
	)?;

	let meta = lua::Table::new(&ctx);
	meta.set(ctx, lua::MetaMethod::Index, index)?;

	Ok(meta)
}
