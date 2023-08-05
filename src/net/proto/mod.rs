//! This module implements network protocols.

pub mod ip;
pub mod tcp;

use super::osi::TransmitPipeline;
use super::BuffList;
use super::SocketDesc;
use crate::errno::EResult;
use crate::util::boxed::Box;

/// Trait representing an object which builds a packet according to a specific protocol.
pub trait TransmitBuilder {
	/// Creates a new builder.
	///
	/// Arguments:
	/// - `desc` is the socket descriptor.
	/// - `sockaddr` is a buffer representing the socket address.
	///
	/// If the given parameters are invalid, the function returns an error.
	fn new(desc: &SocketDesc, sockaddr: &[u8]) -> EResult<Box<dyn TransmitBuilder>>
	where
		Self: Sized;

	/// Transmits data in the given buffer.
	///
	/// Arguments:
	/// - `buff` is the list of buffer which composes the packet being built.
	/// - `next` is the rest of the pipeline to which the build packet is passed.
	fn transmit(&self, buff: BuffList<'_>, next: &TransmitPipeline) -> EResult<()>;
}

/// Packet builder constructor.
pub type TransmitBuilderCtor = fn(&SocketDesc, &[u8]) -> EResult<Box<dyn TransmitBuilder>>;

/// The dummy layer is a simple network layer which does nothing and just passes data to the next
/// layer.
pub struct DummyBuilder {}

impl TransmitBuilder for DummyBuilder {
	fn new(_desc: &SocketDesc, _sockaddr: &[u8]) -> EResult<Box<dyn TransmitBuilder>> {
		Ok(Box::new(DummyBuilder {})? as _)
	}

	fn transmit<'chunk>(&self, buff: BuffList<'chunk>, next: &TransmitPipeline) -> EResult<()> {
		next.transmit(buff)
	}
}
