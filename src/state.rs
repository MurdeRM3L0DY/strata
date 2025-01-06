// Copyright 2023 the Strata authors
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
	collections::HashMap,
	ffi::OsString,
	os::fd::AsRawFd,
	process::Command,
	rc::Rc,
	sync::Arc,
	time::Instant,
};

use piccolo::{
	self as lua,
};
use smithay::{
	backend::input::{
		Event,
		InputBackend,
		InputEvent,
		KeyState,
		KeyboardKeyEvent,
	},
	desktop::{
		layer_map_for_output,
		PopupManager,
		Space,
		Window,
	},
	input::{
		keyboard::{
			FilterResult,
			Keysym,
			ModifiersState,
			XkbConfig,
		},
		Seat,
		SeatState,
	},
	reexports::{
		calloop::{
			generic::{
				FdWrapper,
				Generic,
			},
			EventLoop,
			Interest,
			LoopHandle,
			LoopSignal,
			Mode,
			PostAction,
		},
		wayland_server::{
			backend::{
				ClientData,
				ClientId,
				DisconnectReason,
			},
			Display,
			DisplayHandle,
		},
	},
	utils::{
		Logical,
		Point,
		SERIAL_COUNTER,
	},
	wayland::{
		compositor::{
			CompositorClientState,
			CompositorState,
		},
		output::OutputManagerState,
		selection::{
			data_device::DataDeviceState,
			primary_selection::PrimarySelectionState,
		},
		shell::{
			wlr_layer::{
				Layer,
				WlrLayerShellState,
			},
			xdg::{
				decoration::XdgDecorationState,
				XdgShellState,
			},
		},
		shm::ShmState,
		socket::ListeningSocketSource,
	},
};

use crate::{
	backends::Backend,
	bindings,
	handlers::input::{
		KeyPattern,
		ModFlags,
		Mods,
	},
	workspaces::{
		FocusTarget,
		Workspaces,
	},
};

pub struct Strata {
	pub display: Display<Compositor>,
	pub lua: lua::Lua,
	pub comp: Compositor,
}

impl Strata {
	pub fn new(comp: Compositor, display: Display<Compositor>) -> Self {
		let mut lua = lua::Lua::full();

		std::env::set_var("WAYLAND_DISPLAY", comp.socket_name());

		let ex = lua
			.try_enter(|ctx| {
				let strata = lua::UserData::new_static(&ctx, comp.clone());
				strata.set_metatable(&ctx, Some(bindings::metatable(ctx)?));
				ctx.globals().set(ctx, "strata", strata)?;

				let main = lua::Closure::load(
					ctx,
					None,
					r#"
					local Key = strata.input.Key
					local Mod = strata.input.Mod

					strata.input.keybind({ Mod.Control_L, Mod.Alt_L }, Key.Return, function()
						strata.spawn('kitty')
					end)

					strata.input.keybind({ Mod.Control_L, Mod.Alt_L }, Key.Escape, function()
						strata.quit()
					end)
					"#
					.as_bytes(),
				)?;

				Ok(ctx.stash(lua::Executor::start(ctx, main.into(), ())))
			})
			.unwrap();

		if let Err(e) = lua.execute::<()>(&ex) {
			println!("{:#?}", e);
		}

		Strata {
			lua,
			comp,
			display,
		}
	}

	pub fn process_input_event<I: InputBackend>(&mut self, event: InputEvent<I>) -> anyhow::Result<()> {
		match event {
			InputEvent::Keyboard {
				event, ..
			} => self.keyboard::<I>(event)?,
			InputEvent::PointerMotion {
				event, ..
			} => self.comp.pointer_motion::<I>(event)?,
			InputEvent::PointerMotionAbsolute {
				event, ..
			} => self.comp.pointer_motion_absolute::<I>(event)?,
			InputEvent::PointerButton {
				event, ..
			} => self.comp.pointer_button::<I>(event)?,
			InputEvent::PointerAxis {
				event, ..
			} => self.comp.pointer_axis::<I>(event)?,
			InputEvent::DeviceAdded {
				device: _,
			} => {
				// todo
				println!("device added");
			}
			InputEvent::DeviceRemoved {
				device: _,
			} => todo!(),
			InputEvent::GestureSwipeBegin {
				event: _,
			} => todo!(),
			InputEvent::GestureSwipeUpdate {
				event: _,
			} => todo!(),
			InputEvent::GestureSwipeEnd {
				event: _,
			} => todo!(),
			InputEvent::GesturePinchBegin {
				event: _,
			} => todo!(),
			InputEvent::GesturePinchUpdate {
				event: _,
			} => todo!(),
			InputEvent::GesturePinchEnd {
				event: _,
			} => todo!(),
			InputEvent::GestureHoldBegin {
				event: _,
			} => todo!(),
			InputEvent::GestureHoldEnd {
				event: _,
			} => todo!(),
			InputEvent::TouchDown {
				event: _,
			} => todo!(),
			InputEvent::TouchMotion {
				event: _,
			} => todo!(),
			InputEvent::TouchUp {
				event: _,
			} => todo!(),
			InputEvent::TouchCancel {
				event: _,
			} => todo!(),
			InputEvent::TouchFrame {
				event: _,
			} => todo!(),
			InputEvent::TabletToolAxis {
				event: _,
			} => todo!(),
			InputEvent::TabletToolProximity {
				event: _,
			} => todo!(),
			InputEvent::TabletToolTip {
				event: _,
			} => todo!(),
			InputEvent::TabletToolButton {
				event: _,
			} => todo!(),
			InputEvent::Special(_) => todo!(),
			// _ => anyhow::bail!("unhandled winit event: {:#?}", &event),
		};

		Ok(())
	}

	pub fn keyboard<I: InputBackend>(&mut self, event: I::KeyboardKeyEvent) -> anyhow::Result<()> {
		let serial = SERIAL_COUNTER.next_serial();
		let time = Event::time_msec(&event);

		let keyboard = self
			.comp
			.seat
			.get_keyboard()
			.ok_or_else(|| anyhow::anyhow!("no keyboard attached to seat"))?;
		if let Some(k) = keyboard.input(
			&mut self.comp,
			event.key_code(),
			event.state(),
			serial,
			time,
			|comp, mods, keysym_h| {
				comp.handle_mods::<I>(mods, keysym_h.modified_sym(), &event);

				println!("{:#?}", comp.mods);
				println!("{:#?}({:#?})", event.state(), keysym_h.modified_sym());
				match event.state() {
					KeyState::Pressed => {
						let k = KeyPattern {
							modifier: comp.mods.flags,
							key: keysym_h.modified_sym().into(),
						};

						if comp.config.keybinds.contains_key(&k) {
							FilterResult::Intercept(k)
						} else {
							FilterResult::Forward
						}
					}
					KeyState::Released => FilterResult::Forward,
				}
			},
			})?;
		};

		Ok(())
	}
}

pub struct Compositor {
	pub backend: Backend,

	pub display_handle: DisplayHandle,
	pub loop_handle: LoopHandle<'static, Strata>,
	pub loop_signal: LoopSignal,

	pub clock: Instant,

	pub compositor_state: CompositorState,
	pub xdg_shell_state: XdgShellState,
	pub xdg_decoration_state: XdgDecorationState,
	pub shm_state: ShmState,
	pub output_manager_state: OutputManagerState,
	pub data_device_state: DataDeviceState,
	pub primary_selection_state: PrimarySelectionState,
	pub seat_state: SeatState<Compositor>,
	pub layer_shell_state: WlrLayerShellState,
	pub popup_manager: PopupManager,
	pub space: Space<Window>,
	pub seat: Seat<Compositor>,
	pub socket_name: OsString,
	pub workspaces: Workspaces,
	pub mods: Mods,
	pub config: StrataConfig,
}

impl Compositor {
	pub fn new(event_loop: &EventLoop<'static, Strata>, display: &mut Display<Self>) -> anyhow::Result<Self> {
		let loop_handle = event_loop.handle();
		let display_handle = display.handle();

		let listening_socket = ListeningSocketSource::new_auto().unwrap();
		let socket_name = listening_socket.socket_name().to_os_string();
		loop_handle
			.insert_source(listening_socket, move |client_stream, _, state| {
				// You may also associate some data with the client when inserting the client.
				state
					.display
					.handle()
					.insert_client(client_stream, Arc::new(ClientState::default()))
					.unwrap();
			})
			.expect("Failed to init the wayland event source.");

		loop_handle
			.insert_source(
				Generic::new(
					unsafe { FdWrapper::new(display.backend().poll_fd().as_raw_fd()) },
					Interest::READ,
					Mode::Level,
				),
				|_, _, state| {
					state.display.dispatch_clients(&mut state.comp)?;
					Ok(PostAction::Continue)
				},
			)
			.unwrap();

		let mut seat_state = SeatState::new();
		let mut seat = seat_state.new_wl_seat(&display_handle, "Strata");
		let keyboard = seat
			.add_keyboard(
				XkbConfig {
					layout: "it",
					options: Some("caps:swapescape".to_string()),
					..Default::default()
				},
				160,
				40,
			)
			.expect("Couldn't parse XKB config");
		seat.add_pointer();

		let config_workspace: u8 = 5;
		let workspaces = Workspaces::new(config_workspace);
		let mods_state = keyboard.modifier_state();

		let compositor_state = CompositorState::new::<Compositor>(&display_handle);
		let xdg_shell_state = XdgShellState::new::<Compositor>(&display_handle);
		let xdg_decoration_state = XdgDecorationState::new::<Compositor>(&display_handle);
		let shm_state = ShmState::new::<Compositor>(&display_handle, vec![]);
		let output_manager_state = OutputManagerState::new_with_xdg_output::<Compositor>(&display_handle);
		let data_device_state = DataDeviceState::new::<Compositor>(&display_handle);
		let primary_selection_state = PrimarySelectionState::new::<Compositor>(&display_handle);
		let layer_shell_state = WlrLayerShellState::new::<Compositor>(&display_handle);

		let comp = Compositor {
			backend: Backend::Unset,
			display_handle,
			loop_handle,
			loop_signal: event_loop.get_signal(),

			clock: Instant::now(),

			compositor_state,
			xdg_shell_state,
			xdg_decoration_state,
			shm_state,
			output_manager_state,
			data_device_state,
			primary_selection_state,
			seat_state,
			layer_shell_state,
			popup_manager: PopupManager::default(),
			space: Space::<Window>::default(),
			seat,
			socket_name,
			workspaces,
			mods: Mods {
				flags: Modifier::empty(),
				state: mods_state,
			},
			config: StrataConfig {
				keybinds: HashMap::new(),
			},
		};

		Ok(comp)
	}

	pub fn surface_under(&self) -> Option<(FocusTarget, Point<i32, Logical>)> {
		let pos = self.seat.get_pointer().unwrap().current_location();
		let output = self.workspaces.current().outputs().find(|o| {
			let geometry = self.workspaces.current().output_geometry(o).unwrap();
			geometry.contains(pos.to_i32_round())
		})?;
		let output_geo = self.workspaces.current().output_geometry(output).unwrap();
		let layers = layer_map_for_output(output);

		let mut under = None;
		if let Some(layer) = layers
			.layer_under(Layer::Overlay, pos)
			.or_else(|| layers.layer_under(Layer::Top, pos))
		{
			let layer_loc = layers.layer_geometry(layer).unwrap().loc;
			under = Some((layer.clone().into(), output_geo.loc + layer_loc))
		} else if let Some((window, location)) = self.workspaces.current().window_under(pos) {
			under = Some((window.clone().into(), location));
		} else if let Some(layer) = layers
			.layer_under(Layer::Bottom, pos)
			.or_else(|| layers.layer_under(Layer::Background, pos))
		{
			let layer_loc = layers.layer_geometry(layer).unwrap().loc;
			under = Some((layer.clone().into(), output_geo.loc + layer_loc));
		};
		under
	}

	pub fn close_window(&mut self) {
		let ptr = self.seat.get_pointer().unwrap();
		if let Some((window, _)) = self.workspaces.current().window_under(ptr.current_location()) {
			window.toplevel().send_close()
		}
	}

	pub fn switch_to_workspace(&mut self, id: u8) {
		self.workspaces.activate(id);
		self.set_input_focus_auto();
	}

	pub fn move_window_to_workspace(&mut self, id: u8) {
		let pos = self.seat.get_pointer().unwrap().current_location();
		let window = self.workspaces.current().window_under(pos).map(|d| d.0.clone());

		if let Some(window) = window {
			self.workspaces.move_window_to_workspace(&window, id);
		}
	}

	pub fn follow_window_move(&mut self, id: u8) {
		self.move_window_to_workspace(id);
		self.switch_to_workspace(id);
	}

	pub fn quit(&self) {
		self.loop_signal.stop();
	}

	pub fn spawn(&mut self, command: &str) {
		Command::new("/bin/sh")
			.arg("-c")
			.arg(command)
			.spawn()
			.expect("Failed to spawn command");
	}

	pub fn handle_mods<I: InputBackend>(
		&mut self,
		new_modstate: &ModifiersState,
		keysym: Keysym,
		event: &I::KeyboardKeyEvent,
	) {
		let old_modstate = self.mods.state;

		let modflag = match keysym {
			// equivalent to "Control_* + Shift_* + Alt_*" (on my keyboard *smile*)
			Keysym::Meta_L => ModFlags::Alt_L,
			Keysym::Meta_R => ModFlags::Alt_R,

			Keysym::Shift_L => ModFlags::Shift_L,
			Keysym::Shift_R => ModFlags::Shift_R,

			Keysym::Control_L => ModFlags::Control_L,
			Keysym::Control_R => ModFlags::Control_R,

			Keysym::Alt_L => ModFlags::Alt_L,
			Keysym::Alt_R => ModFlags::Alt_R,

			Keysym::Super_L => ModFlags::Super_L,
			Keysym::Super_R => ModFlags::Super_R,

			Keysym::ISO_Level3_Shift => ModFlags::ISO_Level3_Shift,
			Keysym::ISO_Level5_Shift => ModFlags::ISO_Level5_Shift,

			_ => ModFlags::empty(),
		};

		match event.state() {
			KeyState::Pressed => {
				let depressed = if new_modstate == &old_modstate {
					// ignore previous modstate
					true
				} else {
					// "lock" key modifier or "normal" key modifier
					new_modstate.serialized.depressed > old_modstate.serialized.depressed
				};

				// "lock" key modifiers (Caps Lock, Num Lock, etc...) => `depressed` == `locked`
				// "normal" key modifiers (Control_*, Shift_*, etc...) => `depressed` > 0
				// "normal" keys (a, s, d, f) => `depressed` == 0
				let is_modifier =
					new_modstate.serialized.depressed > new_modstate.serialized.locked - old_modstate.serialized.locked;

				if is_modifier && depressed {
					self.mods.flags ^= modflag;
				}
			}
			KeyState::Released => {
				self.mods.flags ^= modflag;
			}
		};

		self.mods.state = *new_modstate;
	}
}

pub trait WithState {
	type State;

	fn with_state<F, T>(&self, f: F)
	where
		F: FnOnce(&Self::State) -> T;
	fn with_state_mut<F, T>(&self, f: F)
	where
		F: FnOnce(&mut Self::State) -> T;
}

pub struct StrataConfig {
	pub keybinds: HashMap<KeyPattern, lua::StashedFunction>,
}

#[derive(Default)]
pub struct ClientState {
	pub compositor_state: CompositorClientState,
}
impl ClientData for ClientState {
	fn initialized(&self, _client_id: ClientId) {}

	fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}
