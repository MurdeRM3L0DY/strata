// Copyright 2023 the Strata authors
// SPDX-License-Identifier: GPL-3.0-or-later

use std::process::Stdio;

use anyhow::Context as _;
use piccolo::{
	self as lua,
	IntoValue as _,
};

use super::meta::Child;
use crate::util::get_str_from_value;

pub fn api<'gc>(ctx: lua::Context<'gc>, comp: lua::UserData<'gc>) -> anyhow::Result<lua::Value<'gc>> {
	let proc = lua::Table::new(&ctx);

	proc.set_field(
		ctx,
		"spawn",
		lua::Callback::from_fn_with(&ctx, comp, |&comp, ctx, _, mut stack| {
			let (cmd, opts) = stack.consume::<(lua::Value, Option<lua::Table>)>(ctx)?;

			let cmd = match cmd {
				lua::Value::Table(cmd) => {
					if cmd.length() == 0 {
						return Err(anyhow::anyhow!("expected a `table<string>`\ncommand list is empty").into());
					}

					cmd.iter()
						.map(|(_, v)| {
							get_str_from_value(ctx, v)
								.context("expected a `table<string>`\none of the values is not a valid `string`\n")
						})
						.collect::<Result<Vec<_>, _>>()
				}
				lua::Value::String(cmd) => Ok(vec![cmd.to_str()?; 1]),
				v => {
					return Err(anyhow::anyhow!(
						"{:?}",
						lua::TypeError {
							expected: "`string` or `table<string>`",
							found: v.type_name(),
						}
					)
					.into());
				}
			}?;

			let child = std::process::Command::new(cmd[0])
				.args(&cmd[1..])
				.stdin(Stdio::piped())
				.stdout(Stdio::piped())
				.stderr(Stdio::piped())
				.spawn()?;
			stack.push_front(Child::new_userdata(ctx, comp, child)?.into());

			Ok(lua::CallbackReturn::Return)
		}),
	);

	Ok(proc.into_value(ctx))
}
