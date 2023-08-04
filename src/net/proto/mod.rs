//! This module implements layers for different protocols.

pub mod ip;
pub mod tcp;

use crate::errno::EResult;
use crate::net::BuffList;
use crate::net::SocketDesc;
use crate::util::boxed::Box;

/// An OSI layer.
///
/// A layer stack acts as a pipeline, passing data from one layer to the other.
pub trait Layer {
	// TODO receive

	/// Transmits data in the given buffer.
	///
	/// Arguments:
	/// - `buff` is the list of buffer which composes the packet being built.
	/// - `next` is the function called to pass the buffers list to the next layer.
	fn transmit<'c, F>(&self, buff: BuffList<'c>, next: F) -> EResult<()>
	where
		Self: Sized,
		F: Fn(BuffList<'c>) -> EResult<()>;
}

/// Function used to build a layer from a given sockaddr structure.
pub type LayerBuilder = fn(&SocketDesc, &[u8]) -> EResult<Box<dyn Layer>>;

/// The dummy layer is a simple network layer which does nothing and just passes data to the next
/// layer.
pub struct DummyLayer {}

impl Layer for DummyLayer {
	fn transmit<'c, F>(&self, buff: BuffList<'c>, next: F) -> EResult<()>
	where
		F: Fn(BuffList<'c>) -> EResult<()>,
	{
		next(buff)
	}
}

/// Builder for a dummy layer.
pub fn dummy_build(_desc: &SocketDesc, _sockaddr: &[u8]) -> EResult<Box<dyn Layer>> {
	Ok(Box::new(DummyLayer {})? as _)
}
