pub mod args;
pub mod decorations;
pub mod state;
pub mod workspaces;

pub struct CommsChannel<T> {
	pub sender: crossbeam_channel::Sender<T>,
	pub receiver: crossbeam_channel::Receiver<T>,
}
