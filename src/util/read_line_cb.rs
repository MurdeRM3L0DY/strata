use std::{
	io::{
		BufRead,
		BufReader,
		Read,
	},
	os::fd::{AsFd, AsRawFd},
};

use piccolo::{
	self as lua,
};
use smithay::reexports::calloop::{
	self,
	generic::Generic,
	EventSource,
	PostAction,
};

pub struct ReadLineMeta {
	pub buf: Vec<u8>,
	pub cb: lua::StashedFunction,
}

pub struct ReadLineCb<S: Read + AsFd> {
	source: Generic<S>,
	meta: ReadLineMeta,
}

impl<S: Read + AsFd> ReadLineCb<S> {
	pub fn new(src: S, cb: lua::StashedFunction) -> anyhow::Result<Self> {
		let f = nix::fcntl::fcntl(src.as_fd().as_raw_fd(), nix::fcntl::F_GETFL)?;
		let mut oflags = nix::fcntl::OFlag::from_bits(f).expect("should be a valid oflag");
		oflags |= nix::fcntl::OFlag::O_NONBLOCK;
		nix::fcntl::fcntl(src.as_fd().as_raw_fd(), nix::fcntl::F_SETFL(oflags))?;


		let source = ReadLineCb {
			meta: ReadLineMeta {
				buf: Vec::with_capacity(32),
				cb,
			},
			source: calloop::generic::Generic::new(src, calloop::Interest::READ, calloop::Mode::Level),
		};

		Ok(source)
	}
}

impl<S: Read + AsFd> EventSource for ReadLineCb<S> {
	type Error = std::io::Error;
	type Event = ();
	type Metadata = ReadLineMeta;
	type Ret = std::io::Result<()>;

	fn process_events<F>(
		&mut self,
		readiness: smithay::reexports::calloop::Readiness,
		token: smithay::reexports::calloop::Token,
		mut callback: F,
	) -> Result<smithay::reexports::calloop::PostAction, Self::Error>
	where
		F: FnMut(Self::Event, &mut Self::Metadata) -> Self::Ret,
	{
		self.source.process_events(readiness, token, |_, src| {
			// Safety: src isn't moved or dropped
			let src = unsafe { src.get_mut() };

			let mut reader = BufReader::new(src);
			loop {
				let Ok(read) = reader.read_until(b'\n', &mut self.meta.buf) else {
					break;
				};

				if read == 0 {
					break;
				}

				callback((), &mut self.meta)?;
				self.meta.buf.clear();
			}

			Ok(PostAction::Continue)
		})
	}

	fn register(
		&mut self,
		poll: &mut smithay::reexports::calloop::Poll,
		token_factory: &mut smithay::reexports::calloop::TokenFactory,
	) -> smithay::reexports::calloop::Result<()> {
		self.source.register(poll, token_factory)?;

		Ok(())
	}

	fn reregister(
		&mut self,
		poll: &mut smithay::reexports::calloop::Poll,
		token_factory: &mut smithay::reexports::calloop::TokenFactory,
	) -> smithay::reexports::calloop::Result<()> {
		self.source.reregister(poll, token_factory)?;

		Ok(())
	}

	fn unregister(&mut self, poll: &mut smithay::reexports::calloop::Poll) -> smithay::reexports::calloop::Result<()> {
		self.source.unregister(poll)?;

		Ok(())
	}
}
