// Copyright 2023 the Strata authors
// SPDX-License-Identifier: GPL-3.0-or-later

use std::time::Duration;

use smithay::{
	backend::{
		renderer::{
			damage::OutputDamageTracker,
			glow::GlowRenderer,
		},
		winit::{
			self,
			WinitEvent,
			WinitEventLoop,
			WinitGraphicsBackend,
		},
	},
	desktop::space::SpaceElement,
	output::{
		Mode,
		Output,
		PhysicalProperties,
		Subpixel,
	},
	reexports::{
		calloop::timer::{
			TimeoutAction,
			Timer,
		},
		winit::platform::pump_events::PumpStatus,
	},
	utils::{
		Rectangle,
		Transform,
	},
};

use crate::{
	backends::Backend,
	decorations::BorderShader,
	state::{
		Compositor,
		Strata,
	},
};

pub struct WinitData {
	pub backend: WinitGraphicsBackend<GlowRenderer>,
	pub damage_tracker: OutputDamageTracker,
}

impl Strata {
	pub fn winit_dispatch(&mut self, winit_loop: &mut WinitEventLoop, output: &Output) {
		let res = winit_loop.dispatch_new_events(|event| {
			match event {
				WinitEvent::Resized {
					size, ..
				} => {
					output.change_current_state(
						Some(Mode {
							size,
							refresh: 60_000,
						}),
						None,
						None,
						None,
					);
				}
				WinitEvent::Input(event) => {
					if let Err(e) = self.process_input_event(event) {
						println!("{:#?}", e);
					}
				}
				_ => (),
			}
		});

		if let PumpStatus::Exit(_) = res {
			self.comp.loop_signal().stop();
		} else {
			self.winit_update();
		}
	}

	fn winit_update(&mut self) {
		let render_elements = self
			.comp
			.workspaces()
			.current()
			.render_elements(self.comp.backend().winit().backend.renderer());

		self.comp
			.backend()
			.winit()
			.damage_tracker
			.render_output(
				self.comp.backend().winit().backend.renderer(),
				0,
				&render_elements,
				[0.1, 0.1, 0.1, 1.0],
			)
			.unwrap();

		self.comp.set_input_focus_auto();

		// damage tracking
		let size = self.comp.backend().winit().backend.window_size();
		let damage = Rectangle::from_loc_and_size((0, 0), size);
		self.comp.backend().winit().backend.bind().unwrap();
		self.comp.backend().winit().backend.submit(Some(&[damage])).unwrap();

		// sync and cleanups
		let output = self.comp.workspaces().current().outputs().next().unwrap();
		self.comp.workspaces().current().windows().for_each(|window| {
			window.send_frame(output, self.comp.clock().elapsed(), Some(Duration::ZERO), |_, _| {
				Some(output.clone())
			});

			window.refresh();
		});
		self.comp.popup_manager().cleanup();
		BorderShader::cleanup(self.comp.backend().winit().backend.renderer());
	}
}

impl WinitData {
	pub fn new(comp: &mut Compositor) -> anyhow::Result<Backend> {
		let (mut backend, mut winit_loop) = winit::init().unwrap();
		let mode = Mode {
			size: backend.window_size(),
			refresh: 60_000,
		};
		let output = Output::new(
			"winit".to_string(),
			PhysicalProperties {
				size: (0, 0).into(),
				subpixel: Subpixel::Unknown,
				make: "Strata".into(),
				model: "Winit".into(),
			},
		);
		let _global = output.create_global::<Compositor>(comp.display_handle());
		output.change_current_state(Some(mode), Some(Transform::Flipped180), None, Some((0, 0).into()));
		output.set_preferred(mode);

		let damage_tracker = OutputDamageTracker::from_output(&output);

		BorderShader::init(backend.renderer());
		for workspace in comp.workspaces().iter() {
			workspace.add_output(output.clone());
		}

		comp.loop_handle()
			.insert_source(Timer::immediate(), move |_, _, data| {
				data.winit_dispatch(&mut winit_loop, &output);
				TimeoutAction::ToDuration(Duration::from_millis(16))
			})
			.map_err(|_| anyhow::anyhow!("unable to insert winit timer source"))?;

		Ok(Backend::Winit(WinitData {
			backend,
			damage_tracker,
		}))
	}
}
