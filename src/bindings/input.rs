// Copyright 2023 the Strata authors
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Context as _;
use piccolo::{
	self as lua,
	FromValue as _,
	IntoValue as _,
};
use smithay::input::keyboard::xkb::keysym_from_name;

use super::get_str_from_value;
use crate::{
	bindings::ContextExt,
	config::StrataXkbConfig,
	handlers::input::{
		Key,
		KeyPattern,
		Modifiers,
	},
};

fn keys<'gc>(ctx: lua::Context<'gc>, comp: lua::UserData<'gc>) -> anyhow::Result<lua::Table<'gc>> {
	let meta = lua::Table::new(&ctx);

	meta.set(
		ctx,
		lua::MetaMethod::Index,
		lua::Callback::from_fn(&ctx, |ctx, _, mut stack| {
			let (_, k) = stack.consume::<(lua::Table, lua::String)>(ctx)?;
			let k = k.to_str()?;

			stack.push_front(lua::Value::Integer(keysym_from_name(k, 0).raw() as i64));

			Ok(lua::CallbackReturn::Return)
		}),
	)?;

	let keys = lua::Table::new(&ctx);
	keys.set_metatable(&ctx, Some(meta));

	Ok(keys)
}

fn modifiers<'gc>(ctx: lua::Context<'gc>, comp: lua::UserData<'gc>) -> anyhow::Result<lua::Table<'gc>> {
	let meta = lua::Table::new(&ctx);

	meta.set(
		ctx,
		lua::MetaMethod::Index,
		lua::Callback::from_fn(&ctx, |ctx, _, mut stack| {
			let (_, k) = stack.consume::<(lua::Table, lua::String)>(ctx)?;
			let k = k.to_str()?;
			let bits = Modifiers::from_name(k).ok_or_else(|| anyhow::anyhow!("invalid Mod key: {}", k))?;

			stack.push_front(lua::Value::Integer(bits.bits() as i64));

			Ok(lua::CallbackReturn::Return)
		}),
	)?;

	let modifiers = lua::Table::new(&ctx);
	modifiers.set_metatable(&ctx, Some(meta));

	Ok(modifiers)
}

pub fn module<'gc>(ctx: lua::Context<'gc>, comp: lua::UserData<'gc>) -> anyhow::Result<lua::Value<'gc>> {
	let input = lua::Table::new(&ctx);

	input.set_field(ctx, "Keys", keys(ctx, comp)?);
	input.set_field(ctx, "Modifiers", modifiers(ctx, comp)?);

	input.set_field(
		ctx,
		"keybind",
		lua::Callback::from_fn_with(&ctx, comp, |&comp, ctx, _, mut stack| {
			let comp = ctx.comp(comp)?;
			let (modifiers, key, cb) = stack.consume::<(Modifiers, Key, lua::Function)>(ctx)?;

			let keypat = KeyPattern {
				modifiers,
				key,
			};

			comp.with_mut(|comp| {
				comp.config.input_config.global_keybinds.insert(keypat, ctx.stash(cb));
			});

			Ok(lua::CallbackReturn::Return)
		}),
	);

	input.set_field(
		ctx,
		"setup",
		lua::Callback::from_fn_with(&ctx, comp, |&comp, ctx, _, mut stack| {
			let comp = ctx.comp(comp)?;
			let (cfg,) = stack.consume::<(lua::Table,)>(ctx)?;

			comp.with_mut(|comp| {
				if let lua::Value::Table(repeat_info) = cfg.get_value(ctx, "repeat_info") {
					let rate = i64::from_value(ctx, repeat_info.get_value(ctx, "rate"))
						.context("`repeat_info.rate` is invalid")?;
					let delay = i64::from_value(ctx, repeat_info.get_value(ctx, "delay"))
						.context("`repeat_info.delay` is invalid")?;

					comp.seat
						.get_keyboard()
						.context("Unable to get keyboard")?
						.change_repeat_info(rate.abs() as i32, delay.abs() as i32);
				}

				if let Some(xkbconfig) = Option::<lua::Table>::from_value(ctx, cfg.get_value(ctx, "xkbconfig"))
					.context("`xkbconfig` is invalid")?
				{
					StrataXkbConfig::update(comp, |cfg| {
						let Some(cfg) = cfg else { return Ok(()) };

						if let Some(layout) = Option::<lua::String>::from_value(ctx, xkbconfig.get_value(ctx, "layout"))
							.context("`xkbconfig.layout` is invalid")?
							.map(|s| get_str_from_value(ctx, s.into()))
							.and_then(|s| s.ok())
						{
							cfg.layout.replace_range(.., layout);
						}

						if let Some(rules) = Option::<lua::String>::from_value(ctx, xkbconfig.get_value(ctx, "rules"))
							.context("`xkbconfig.rules` is invalid")?
							.map(|s| get_str_from_value(ctx, s.into()))
							.and_then(|s| s.ok())
						{
							cfg.rules.replace_range(.., rules);
						}

						if let Some(model) = Option::<lua::String>::from_value(ctx, xkbconfig.get_value(ctx, "model"))
							.context("`xkbconfig.model` is invalid")?
							.map(|s| get_str_from_value(ctx, s.into()))
							.and_then(|s| s.ok())
						{
							cfg.model.replace_range(.., model);
						}

						if let Some(options) =
							Option::<lua::String>::from_value(ctx, xkbconfig.get_value(ctx, "options"))
								.context("`xkbconfig.options` is invalid")?
								.map(|s| get_str_from_value(ctx, s.into()))
								.and_then(|s| s.ok())
						{
							if let Some(s) = cfg.options.as_mut() {
								s.replace_range(.., options);
							} else {
								cfg.options = Some(options.to_string());
							}
						}

						if let Some(variant) =
							Option::<lua::String>::from_value(ctx, xkbconfig.get_value(ctx, "variant"))
								.context("`xkbconfig.variant` is invalid")?
								.map(|s| get_str_from_value(ctx, s.into()))
								.and_then(|s| s.ok())
						{
							cfg.variant.replace_range(.., variant);
						}

						Ok(())
					})?;
				}

				// if let lua::Value::Table(keybinds) = cfg.get_value(ctx, "keybinds") {
				// 	for (_, keybind) in keybinds {
				// 		match keybind {
				// 			lua::Value::Table(keybind) => {
				// 				let modifiers = Modifiers::from_value(ctx, keybind.get_value(ctx, 1))?;
				// 				let key = Key::from_value(ctx, keybind.get_value(ctx, 2))?;
				// 				let cb = lua::Function::from_value(ctx, keybind.get_value(ctx, 3))?;
				//
				// 				comp.config.input_config.global_keybinds.insert(
				// 					KeyPattern {
				// 						modifiers,
				// 						key,
				// 					},
				// 					ctx.stash(cb),
				// 				);
				// 			}
				// 			_ => {
				// 				todo!()
				// 			}
				// 		}
				// 	}
				// }

				anyhow::Ok(())
			})?;

			Ok(lua::CallbackReturn::Return)
		}),
	);

	Ok(input.into())
}
