// Copyright 2023 the Strata authors
// SPDX-License-Identifier: GPL-3.0-or-later

use std::process::Stdio;

use anyhow::Context as _;
use piccolo::{
	self as lua,
	FromValue as _,
	IntoValue as _,
};
use smithay::reexports::calloop;
use tokio::io::{
	AsyncBufReadExt as _,
	BufReader,
};

use super::get_str_from_value;
use crate::bindings::ContextExt;

enum CommandStream {
	Stdout(String),
	Stderr(String),
}

pub fn module<'gc>(ctx: lua::Context<'gc>, comp: lua::UserData<'gc>) -> anyhow::Result<lua::Value<'gc>> {
	let proc = lua::Table::new(&ctx);

	proc.set_field(
		ctx,
		"spawn",
		lua::Callback::from_fn_with(&ctx, comp, |&comp, ctx, _, mut stack| {
			let comp = ctx.comp(comp)?;
			let (cmd, callbacks) = stack.consume::<(lua::Value, Option<lua::Table>)>(ctx)?;

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

			let child = tokio::process::Command::new(cmd[0])
				.args(&cmd[1..])
				.stdin(Stdio::piped())
				.stdout(Stdio::piped())
				.stderr(Stdio::piped())
				.spawn()?;

			let pid = child.id();

			if let Some(callbacks) = callbacks {
				let stdout =
					Option::<lua::Function>::from_value(ctx, callbacks.get_value(ctx, "stdout"))?.map(|f| ctx.stash(f));
				let stderr =
					Option::<lua::Function>::from_value(ctx, callbacks.get_value(ctx, "stderr"))?.map(|f| ctx.stash(f));

				let has_stdout = stdout.is_some();
				let has_stderr = stderr.is_some();

				let (tx, s) = calloop::channel::channel::<CommandStream>();

				let stdout_handle = has_stdout.then(|| {
					let tx = tx.clone();

					tokio::spawn(async move {
						let Some(stdout) = child.stdout else { return Ok(()) };

						let reader = BufReader::new(stdout);
						let mut lines = reader.lines();

						// Asynchronously read lines
						while let Some(line) = lines.next_line().await? {
							tx.send(CommandStream::Stdout(line))?;
						}

						anyhow::Ok(())
					})
				});

				let stderr_handle = has_stderr.then(|| {
					// let tx = tx.clone();

					tokio::spawn(async move {
						let Some(stderr) = child.stderr else { return Ok(()) };

						let reader = BufReader::new(stderr);
						let mut lines = reader.lines();

						// Asynchronously read lines
						while let Some(line) = lines.next_line().await? {
							tx.send(CommandStream::Stderr(line))?;
						}

						anyhow::Ok(())
					})
				});

				if has_stdout || has_stderr {
					comp.with_mut(|comp| {
						let reg = comp
							.loop_handle
							.insert_source(s, move |ev, _, strata| {
								match ev {
									calloop::channel::Event::Msg(stream) => {
										match stream {
											CommandStream::Stdout(s) => {
												let Some(f) = &stdout else { return };

												if let Err(e) = strata.execute_lua::<()>(|ctx, _| ctx.fetch(f), (s,)) {
													println!("{:?}", e);
												}
											}
											CommandStream::Stderr(s) => {
												let Some(f) = &stderr else { return };

												if let Err(e) = strata.execute_lua::<()>(|ctx, _| ctx.fetch(f), (s,)) {
													println!("{:?}", e);
												}
											}
										}
									}
									calloop::channel::Event::Closed => {
										println!("closed streaming!!!");
									}
								}
							})
							.map_err(|e| anyhow::anyhow!("{:?}", e))?;

						anyhow::Ok(())
					})?;
				}
			}

			stack.push_front(pid.into_value(ctx));
			Ok(lua::CallbackReturn::Return)
		}),
	);

	Ok(proc.into_value(ctx))
}
