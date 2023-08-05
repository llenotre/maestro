//! The Transmission Control Protocol (TCP) is a protocol transmitting sequenced, reliable,
//! two-way, connection-based byte streams.

use super::TransmitBuilder;
use crate::errno::EResult;
use crate::net::osi::TransmitPipeline;
use crate::net::sockaddr::SockAddr;
use crate::net::BuffList;
use crate::net::SocketDesc;
use crate::util::boxed::Box;

/// The TCP segment header.
#[repr(C, packed)]
pub struct TCPHdr {
	/// Source port.
	src_port: u16,
	/// Destination port.
	dst_port: u16,

	/// Sequence number.
	seq_nbr: u32,

	/// TODO doc
	ack_nbr: u32,

	/// The size of the header in units of 4 bytes.
	///
	/// Since the first 4 bits are reserved, the value must be shifted by 4 bits.
	data_offset: u8,
	/// The segment's flags.
	flags: u8,
	/// TODO doc
	win_size: u16,

	/// TODO doc
	checksum: u16,
	/// TODO doc
	urg_ptr: u16,
}

/// A builder for TCP packets.
pub struct TCPBuilder {}

impl TransmitBuilder for TCPBuilder {
	fn new(_desc: &SocketDesc, _sockaddr: &dyn SockAddr) -> EResult<Box<dyn TransmitBuilder>> {
		// TODO
		todo!();
	}

	fn transmit<'chunk>(&self, _buff: BuffList<'chunk>, _next: &TransmitPipeline) -> EResult<()> {
		// TODO
		todo!();
	}
}
