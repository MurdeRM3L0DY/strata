// Copyright 2023 the Strata authors
// SPDX-License-Identifier: GPL-3.0-or-later

use log::error;
use smithay::{
	output::Output,
	reexports::{
		calloop::LoopHandle,
		wayland_server::DisplayHandle,
	},
};

use crate::{
	backends::{
		// udev::UdevData,
		winit::WinitData,
	},
	state::{
		Compositor,
		Strata,
	},
};

pub mod cursor;
mod drawing;
pub mod udev;
pub mod winit;

pub enum Backend {
	Winit(WinitData),
	// Udev(UdevData),
	Unset,
}

impl Backend {
	pub fn winit(&self) -> &WinitData {
		match self {
			Backend::Winit(data) => data,
			_ => unreachable!("Tried to retrieve Winit backend when not initialized with it."),
		}
	}

	pub fn winit_mut(&mut self) -> &mut WinitData {
		match self {
			Backend::Winit(data) => data,
			_ => unreachable!("Tried to retrieve Winit backend when not initialized with it."),
		}
	}

	// pub fn udev(&mut self) -> &mut UdevData {
	// 	match self {
	// 		Backend::Udev(data) => data,
	// 		_ => unreachable!("Tried to retrieve Udev backend when not initialized with
	// it."), 	}
	// }

	pub fn from_str(
		backend: &str,
		comp: &mut Compositor,
	) -> anyhow::Result<Self> {
		Ok(match backend {
			"winit" => WinitData::new(comp)?,
			"udev" => {
				todo!()
			}
			unknown => {
				anyhow::bail!("Unknown backend provided: {}", unknown)
			}
		})
	}
}
