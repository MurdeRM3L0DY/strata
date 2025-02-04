// Copyright 2023 the Strata authors
// SPDX-License-Identifier: GPL-3.0-or-later

use bitflags::bitflags;
use piccolo::{
	self as lua,
};
use smithay::{
	backend::input::{
		AbsolutePositionEvent,
		Axis,
		AxisSource,
		Event,
		InputBackend,
		PointerAxisEvent,
		PointerButtonEvent,
		PointerMotionEvent,
	},
	input::{
		keyboard::{
			Keysym,
			ModifiersState,
		},
		pointer::{
			AxisFrame,
			ButtonEvent,
			MotionEvent,
			RelativeMotionEvent,
		},
	},
	utils::SERIAL_COUNTER,
};

use crate::{
	state::Compositor,
	workspaces::FocusTarget,
};

#[derive(Debug)]
pub struct Mods {
	pub flags: Modifier,
	pub state: ModifiersState,
}

// complete list, for future reference
//
// Shift_L Shift_R
// Control_L Control_R
// Meta_L Meta_R
// Alt_L Alt_R
// Super_L Super_R
// Hyper_L Hyper_R
// ISO_Level2_Latch
// ISO_Level3_Shift ISO_Level3_Latch ISO_Level3_Lock
// ISO_Level5_Shift ISO_Level5_Latch ISO_Level5_Lock

// const KEY_Shift_L = 0xffe1;
// const KEY_Shift_R = 0xffe2;
// const KEY_Control_L = 0xffe3;
// const KEY_Control_R = 0xffe4;
// const KEY_Caps_Lock = 0xffe5;
// const KEY_Shift_Lock = 0xffe6;
//
// const KEY_Meta_L = 0xffe7;
// const KEY_Meta_R = 0xffe8;
// const KEY_Alt_L = 0xffe9;
// const KEY_Alt_R = 0xffea;
// const KEY_Super_L = 0xffeb;
// const KEY_Super_R = 0xffec;
// const KEY_Hyper_L = 0xffed;
// const KEY_Hyper_R = 0xffee;
//
//
// const KEY_ISO_Lock = 0xfe01;
// const KEY_ISO_Level2_Latch = 0xfe02;
// const KEY_ISO_Level3_Shift = 0xfe03;
// const KEY_ISO_Level3_Latch = 0xfe04;
// const KEY_ISO_Level3_Lock = 0xfe05;
// const KEY_ISO_Level5_Shift = 0xfe11;
// const KEY_ISO_Level5_Latch = 0xfe12;
// const KEY_ISO_Level5_Lock = 0xfe13;
// const KEY_ISO_Group_Shift = 0xff7e;
// const KEY_ISO_Group_Latch = 0xfe06;
// const KEY_ISO_Group_Lock = 0xfe07;
// const KEY_ISO_Next_Group = 0xfe08;
// const KEY_ISO_Next_Group_Lock = 0xfe09;
// const KEY_ISO_Prev_Group = 0xfe0a;
// const KEY_ISO_Prev_Group_Lock = 0xfe0b;
// const KEY_ISO_First_Group = 0xfe0c;
// const KEY_ISO_First_Group_Lock = 0xfe0d;
// const KEY_ISO_Last_Group = 0xfe0e;
// const KEY_ISO_Last_Group_Lock = 0xfe0f;
bitflags! {
	#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
	pub struct Modifier: u16 {
		const Shift_L = 1;
		const Shift_R = 1 << 1;
		const Control_L = 1 << 2;
		const Control_R = 1 << 3;
		const Alt_L = 1 << 4;
		const Alt_R = 1 << 5;
		const Super_L = 1 << 6;
		const Super_R = 1 << 7;
		const ISO_Level3_Shift = 1 << 8;
		const ISO_Level5_Shift = 1 << 9;
		const Hyper_L = 1 << 10;
		const Hyper_R = 1 << 11;
	}
}

impl<'gc> lua::FromValue<'gc> for Modifier {
	fn from_value(_: lua::Context<'gc>, value: lua::Value<'gc>) -> Result<Self, lua::TypeError> {
		match value {
			// lua::Value::Table(mods) => {
			// 	let mut r = Self::empty();
			//
			// 	for (_, v) in mods {
			// 		match v {
			// 			lua::Value::Integer(bits) => {
			// 				r |= Self::from_bits(bits as u16).ok_or(lua::TypeError {
			// 					expected: "Modifier (integer)",
			// 					found: "Invalid (integer)",
			// 				})?;
			// 			}
			// 			_ => {
			// 				return Err(lua::TypeError {
			// 					expected: "Modifier (integer)",
			// 					found: v.type_name(),
			// 				});
			// 			}
			// 		};
			// 	}
			//
			// 	Ok(r)
			// }
			lua::Value::Nil => Ok(Modifier::empty()),
			lua::Value::Integer(bits) => {
				Ok(Modifier::from_bits(bits as u16).ok_or(lua::TypeError {
					expected: "Modifier (integer)",
					found: "Invalid (integer)",
				})?)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key(Keysym);

impl From<Keysym> for Key {
	fn from(value: Keysym) -> Self {
		Self(value)
	}
}

impl<'gc> lua::FromValue<'gc> for Key {
	fn from_value(_: lua::Context<'gc>, value: lua::Value<'gc>) -> Result<Self, lua::TypeError> {
		match value {
			lua::Value::Integer(key) => Ok(Keysym::new(key as u32).into()),
			_ => {
				Err(lua::TypeError {
					found: value.type_name(),
					expected: "integer",
				})
			}
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyPattern {
	pub modifier: Modifier,
	pub key: Key,
}

impl Compositor {
	pub fn set_input_focus(&mut self, target: FocusTarget) {
		let keyboard = self.seat.get_keyboard().unwrap();
		let serial = SERIAL_COUNTER.next_serial();
		keyboard.set_focus(self, Some(target), serial);
	}

	pub fn set_input_focus_auto(&mut self) {
		let under = self.surface_under();
		if let Some(d) = under {
			self.set_input_focus(d.0);
		}
	}

	pub fn pointer_motion<I: InputBackend>(&mut self, event: I::PointerMotionEvent) -> anyhow::Result<()> {
		let serial = SERIAL_COUNTER.next_serial();
		let delta = (event.delta_x(), event.delta_y()).into();

		self.set_input_focus_auto();

		if let Some(ptr) = self.seat.get_pointer() {
			let location = self.workspaces.current().clamp_coords(ptr.current_location() + delta);

			let under = self.surface_under();

			ptr.motion(
				self,
				under.clone(),
				&MotionEvent {
					location,
					serial,
					time: event.time_msec(),
				},
			);

			ptr.relative_motion(
				self,
				under,
				&RelativeMotionEvent {
					delta,
					delta_unaccel: event.delta_unaccel(),
					utime: event.time(),
				},
			)
		}

		Ok(())
	}

	pub fn pointer_motion_absolute<I: InputBackend>(
		&mut self,
		event: I::PointerMotionAbsoluteEvent,
	) -> anyhow::Result<()> {
		let serial = SERIAL_COUNTER.next_serial();

		let curr_workspace = self.workspaces.current();
		let output = curr_workspace.outputs().next().unwrap();
		let output_geo = curr_workspace.output_geometry(output).unwrap();
		let pos = event.position_transformed(output_geo.size) + output_geo.loc.to_f64();

		let location = self.workspaces.current().clamp_coords(pos);

		self.set_input_focus_auto();

		let under = self.surface_under();
		if let Some(ptr) = self.seat.get_pointer() {
			ptr.motion(
				self,
				under,
				&MotionEvent {
					location,
					serial,
					time: event.time_msec(),
				},
			);
		}

		Ok(())
	}

	pub fn pointer_button<I: InputBackend>(&mut self, event: I::PointerButtonEvent) -> anyhow::Result<()> {
		let serial = SERIAL_COUNTER.next_serial();

		let button = event.button_code();
		let button_state = event.state();
		self.set_input_focus_auto();
		if let Some(ptr) = self.seat.get_pointer() {
			ptr.button(
				self,
				&ButtonEvent {
					button,
					state: button_state,
					serial,
					time: event.time_msec(),
				},
			);
		}

		Ok(())
	}

	pub fn pointer_axis<I: InputBackend>(&mut self, event: I::PointerAxisEvent) -> anyhow::Result<()> {
		let horizontal_amount = event
			.amount(Axis::Horizontal)
			.unwrap_or_else(|| event.amount(Axis::Horizontal).unwrap_or(0.0) * 3.0);
		let vertical_amount = event
			.amount(Axis::Vertical)
			.unwrap_or_else(|| event.amount(Axis::Vertical).unwrap_or(0.0) * 3.0);

		let mut frame = AxisFrame::new(event.time_msec()).source(event.source());
		if horizontal_amount != 0.0 {
			frame = frame.value(Axis::Horizontal, horizontal_amount);
		} else if event.source() == AxisSource::Finger {
			frame = frame.stop(Axis::Horizontal);
		}
		if vertical_amount != 0.0 {
			frame = frame.value(Axis::Vertical, vertical_amount);
		} else if event.source() == AxisSource::Finger {
			frame = frame.stop(Axis::Vertical);
		}

		if let Some(ptr) = self.seat.get_pointer() {
			ptr.axis(self, frame);
		}

		Ok(())
	}
}
