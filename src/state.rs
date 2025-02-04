// Copyright 2023 the Strata authors
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
	ffi::OsString,
	os::fd::AsRawFd,
	sync::Arc,
	time::Instant,
};

use anyhow::Context as _;
use piccolo::{
	self as lua,
};
use piccolo_util::freeze::{
	Freeze,
	FreezeGuard,
	Frozen,
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
	api::{
		self,
	},
	backends::Backend,
	config::{
		StrataConfig,
		StrataXkbConfig,
	},
	handlers::input::{
		KeyPattern,
		Modifier,
		Mods,
	},
	workspaces::{
		FocusTarget,
		Workspaces,
	},
};

mod process_state;

pub use process_state::init_sigchld_handler;

pub type FrozenCompositor = Frozen<Freeze![&'freeze mut Compositor]>;

pub struct Strata {
	pub display: Display<Compositor>,
	pub comp: Compositor,

	lua: lua::Lua,
	ex: lua::StashedExecutor,
	fcomp: FrozenCompositor,
}

impl Strata {
	pub fn new(comp: Compositor, display: Display<Compositor>) -> anyhow::Result<Self> {
		let mut lua = lua::Lua::full();
		let ex = lua.enter(|ctx| ctx.stash(lua::Executor::new(ctx)));
		let mut strata = Strata {
			display,
			comp,
			lua,
			ex,
			fcomp: FrozenCompositor::new(),
		};

		std::env::set_var("WAYLAND_DISPLAY", &strata.comp.socket_name);

		strata.scope(|lua, ex, fcomp| {
			lua.enter(|ctx| {
				ctx.globals().set(ctx, "strata", api::create_global(ctx, fcomp)?)?;

				let main = lua::Closure::load(ctx, None, include_str!("../init.lua").as_bytes())?;

				ctx.fetch(ex).restart(ctx, main.into(), ());

				anyhow::Ok(())
			})?;

			if let Err(e) = lua.execute::<()>(ex) {
				println!("{:?}", e);
			}

			anyhow::Ok(())
		})?;

		Ok(strata)
	}

	pub fn scope<F, R>(&mut self, f: F) -> R
	where
		F: FnOnce(&mut lua::Lua, &lua::StashedExecutor, &FrozenCompositor) -> R,
	{
		FreezeGuard::new(&self.fcomp, &mut self.comp).scope(|| f(&mut self.lua, &self.ex, &self.fcomp))
	}

	pub fn enter<R>(&mut self, f: impl FnOnce(lua::Context, &lua::StashedExecutor, &mut Compositor) -> R) -> R {
		self.lua.enter(|ctx| f(ctx, &self.ex, &mut self.comp))
	}

	pub fn execute<R: for<'gc> lua::FromMultiValue<'gc>>(&mut self) -> anyhow::Result<R> {
		self.scope(|lua, ex, _| lua.execute::<R>(ex).context("lua closure error"))
	}

	pub fn execute_closure<R: for<'gc> lua::FromMultiValue<'gc>>(
		&mut self,
		f: impl for<'gc> FnOnce(lua::Context<'gc>, &lua::StashedExecutor, &mut Compositor),
	) -> anyhow::Result<R> {
		self.enter(f);
		self.execute::<R>()
	}

	pub fn process_input_event<I: InputBackend>(&mut self, event: InputEvent<I>) -> anyhow::Result<()> {
		match event {
			InputEvent::Keyboard {
				event, ..
			} => self.on_keyboard::<I>(event)?,
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

	pub fn on_keyboard<I: InputBackend>(&mut self, event: I::KeyboardKeyEvent) -> anyhow::Result<()> {
		let serial = SERIAL_COUNTER.next_serial();
		let time = Event::time_msec(&event);

		let keyboard = self.comp.seat.get_keyboard().context("no keyboard attached to seat")?;

		if let Some(k) = keyboard.input(
			&mut self.comp,
			event.key_code(),
			event.state(),
			serial,
			time,
			|comp, mods, keysym_h| {
				comp.handle_mods::<I>(mods, keysym_h.modified_sym(), &event);

				// println!("{:#?}", comp.mods);
				// println!("{:#?}({:#?})", event.state(), keysym_h.modified_sym());
				match event.state() {
					KeyState::Pressed => {
						let k = KeyPattern {
							modifier: comp.mods.flags,
							key: keysym_h.modified_sym().into(),
						};

						if comp.config.input_config.global_keybinds.contains_key(&k) {
							FilterResult::Intercept(k)
						} else {
							FilterResult::Forward
						}
					}
					KeyState::Released => FilterResult::Forward,
				}
			},
		) {
			if let Err(e) = self.execute_closure::<()>(|ctx, ex, comp| {
				let f = &comp.config.input_config.global_keybinds[&k];
				ctx.fetch(ex).restart(ctx, ctx.fetch(f), ());
			}) {
				println!("{:?}", e);
			};
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
	pub process_state: process_state::ProcessState,
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

		let config = StrataConfig::default();

		let repeat_info = &config.input_config.repeat_info;
		let xkbconfig = &config
			.input_config
			.xkbconfig
			.as_ref()
			.expect("StrataXkbConfig should have a default set");

		let mut seat_state = SeatState::new();
		let mut seat = seat_state.new_wl_seat(&display_handle, "strata-seat-0");
		let keyboard = seat
			.add_keyboard(
				XkbConfig {
					layout: &xkbconfig.layout,
					options: xkbconfig.options.clone(),
					rules: &xkbconfig.rules,
					model: &xkbconfig.model,
					variant: &xkbconfig.variant,
				},
				repeat_info.delay,
				repeat_info.rate,
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

		let process_state = process_state::ProcessState::new(&loop_handle)?;

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

			config,
			process_state,
		};

		Ok(comp)
	}

	pub fn update_xkbconfig(&mut self, cfg: &StrataXkbConfig) -> anyhow::Result<()> {
		let keyboard = self
			.seat
			.get_keyboard()
			.ok_or_else(|| anyhow::anyhow!("Unable to get keyboard handle"))?;
		keyboard
			.set_xkb_config(
				self,
				XkbConfig {
					layout: &cfg.layout,
					rules: &cfg.rules,
					model: &cfg.model,
					options: cfg.options.clone(),
					variant: &cfg.variant,
				},
			)
			.context(format!("Invalid layout: {:?}", &cfg.layout))?;
		self.mods.state = keyboard.modifier_state();

		Ok(())
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

	pub fn handle_mods<I: InputBackend>(
		&mut self,
		new_modstate: &ModifiersState,
		keysym: Keysym,
		event: &I::KeyboardKeyEvent,
	) {
		let old_modstate = self.mods.state;

		let modflag = match keysym {
			// equivalent to "Control_* + Shift_* + Alt_*" (on my keyboard *smile*)
			Keysym::Meta_L => Modifier::Alt_L,
			Keysym::Meta_R => Modifier::Alt_R,

			Keysym::Shift_L => Modifier::Shift_L,
			Keysym::Shift_R => Modifier::Shift_R,

			Keysym::Control_L => Modifier::Control_L,
			Keysym::Control_R => Modifier::Control_R,

			Keysym::Alt_L => Modifier::Alt_L,
			Keysym::Alt_R => Modifier::Alt_R,

			Keysym::Super_L => Modifier::Super_L,
			Keysym::Super_R => Modifier::Super_R,

			Keysym::ISO_Level3_Shift => Modifier::ISO_Level3_Shift,
			Keysym::ISO_Level5_Shift => Modifier::ISO_Level5_Shift,

			Keysym::Hyper_L => Modifier::Hyper_L,
			Keysym::Hyper_R => Modifier::Hyper_R,

			_ => Modifier::empty(),
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

#[derive(Default)]
pub struct ClientState {
	pub compositor_state: CompositorClientState,
}
impl ClientData for ClientState {
	fn initialized(&self, _client_id: ClientId) {}

	fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}
