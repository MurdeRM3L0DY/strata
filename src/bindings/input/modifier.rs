// Copyright 2023 the Strata authors
// SPDX-License-Identifier: GPL-3.0-or-later

use piccolo::{
	self as lua,
	FromValue,
	IntoValue,
};

use crate::handlers::input::Modifier;

impl<'gc> FromValue<'gc> for Modifier {
	fn from_value(_: lua::Context<'gc>, value: lua::Value<'gc>) -> Result<Self, lua::TypeError> {
		match value {
			lua::Value::Table(mods) => {
				let mut r = Self::empty();

				for (_, v) in mods {
					match v {
						lua::Value::UserData(ud) => {
							let bits = *ud.downcast_static::<Self>().map_err(|_| {
								lua::TypeError {
									expected: "Mod (userdata)",
									found: "Unknown (userdata)",
								}
							})?;

							r |= bits;
						}
						_ => {
							return Err(lua::TypeError {
								expected: "Mod (userdata)",
								found: v.type_name(),
							});
						}
					};
				}

				Ok(r)
			}
			_ => {
				Err(lua::TypeError {
					found: value.type_name(),
					expected: "table",
				})
			}
		}
	}
}

pub fn module<'gc>(ctx: lua::Context<'gc>, comp: lua::UserData<'gc>) -> anyhow::Result<lua::Value<'gc>> {
	let mods = lua::Table::new(&ctx);
	let meta = lua::Table::new(&ctx);

	meta.set(
		ctx,
		lua::MetaMethod::Index,
		lua::Callback::from_fn(&ctx, |ctx, _, mut stack| {
			let _ = stack.pop_front();

			let k = stack.consume::<lua::String>(ctx)?;
			let k = k.to_str()?;
			let bits = Modifier::from_name(k).ok_or_else(|| anyhow::anyhow!("invalid Mod key: {}", k))?;
			let bits = lua::UserData::new_static(&ctx, bits);

			let bits_meta = lua::Table::new(&ctx);
			bits_meta.set(
				ctx,
				lua::MetaMethod::ToString,
				lua::Callback::from_fn(&ctx, |ctx, _, mut stack| {
					let this = stack.consume::<lua::UserData>(ctx)?;
					let this = this.downcast_static::<Modifier>()?;

					stack.push_front(format!("{:#?}", this).into_value(ctx));
					Ok(lua::CallbackReturn::Return)
				}),
			)?;
			bits.set_metatable(&ctx, Some(bits_meta));

			stack.push_front(lua::Value::UserData(bits));

			Ok(lua::CallbackReturn::Return)
		}),
	)?;

	mods.set_metatable(&ctx, Some(meta));

	Ok(mods.into_value(ctx))
}
