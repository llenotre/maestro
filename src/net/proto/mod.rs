//! This module implements network protocols.

pub mod ip;
pub mod tcp;

use super::osi::TransmitPipeline;
use super::BuffList;
use super::SocketDesc;
use crate::errno::EResult;
use crate::net::sockaddr::SockAddr;
use crate::util::boxed::Box;

/// Trait representing an object which builds a packet according to a specific protocol.
pub trait TransmitBuilder {
	/// Creates a new builder.
	///
	/// Arguments:
	/// - `desc` is the socket descriptor
	/// - `sockaddr` is the socket address
	///
	/// If the given parameters are invalid, the function returns an error.
	fn new(desc: &SocketDesc, sockaddr: &dyn SockAddr) -> EResult<Box<dyn TransmitBuilder>>
	where
		Self: Sized;

	/// Transmits data in the given buffer.
	///
	/// Arguments:
	/// - `buff` is the list of buffer which composes the packet being built.
	/// - `next` is the rest of the pipeline to which the build packet is passed.
	fn transmit(&self, buff: BuffList<'_>, next: &TransmitPipeline) -> EResult<()>;
}

/// [Packet builder](TransmitBuilder) constructor.
///
/// This type is to be used when requiring a pointer to the `new` function of the trait
/// [`TransmitBuilder`].
pub type TransmitBuilderCtor = fn(&SocketDesc, &dyn SockAddr) -> EResult<Box<dyn TransmitBuilder>>;

/// The dummy layer is a simple network layer which does nothing and just passes data to the next
/// layer.
pub struct DummyBuilder {}

impl TransmitBuilder for DummyBuilder {
	fn new(_desc: &SocketDesc, _sockaddr: &dyn SockAddr) -> EResult<Box<dyn TransmitBuilder>> {
		Ok(Box::new(DummyBuilder {})? as _)
	}

	fn transmit<'chunk>(&self, buff: BuffList<'chunk>, next: &TransmitPipeline) -> EResult<()> {
		next.transmit(buff)
	}
}
