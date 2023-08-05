//! This module implements the IP protocol.

use super::TransmitBuilder;
use crate::crypto::checksum;
use crate::errno::EResult;
use crate::net::osi::TransmitPipeline;
use crate::net::BuffList;
use crate::net::SocketDesc;
use crate::util;
use crate::util::boxed::Box;
use core::ffi::c_short;
use core::mem::size_of;
use core::slice;

/// The default TTL value.
const DEFAULT_TTL: u8 = 128;

/// IPv4 flag: Do not fragment the packet
const FLAG_DF: u8 = 0b010;
/// IPv4 flag: More fragments are to come after this one
const FLAG_MF: u8 = 0b100;

/// Protocol: TCP
pub const PROTO_TCP: u8 = 0x06;
/// Protocol: UDP
pub const PROTO_UDP: u8 = 0x11;

/// The IPv4 header (RFC 791).
#[repr(C, packed)]
struct IPv4Header {
	/// The version of the header with the IHL (header length).
	version_ihl: u8,
	/// The type of service.
	type_of_service: u8,
	/// The total length of the datagram.
	total_length: u16,

	/// TODO doc
	identification: u16,
	/// TODO doc
	flags_fragment_offset: u16,

	/// Time-To-Live.
	ttl: u8,
	/// Protocol number.
	protocol: u8,
	/// The checksum of the header (RFC 1071).
	hdr_checksum: u16,

	/// Source address.
	src_addr: [u8; 4],
	/// Destination address.
	dst_addr: [u8; 4],
}

impl IPv4Header {
	/// Checks the checksum of the packet.
	///
	/// If correct, the function returns `true`.
	pub fn check_checksum(&self) -> bool {
		let slice =
			unsafe { slice::from_raw_parts(self as *const _ as *const u8, size_of::<Self>()) };

		checksum::compute_rfc1071(slice) == 0
	}

	/// Computes the checksum of the header and writes it into the appropriate field.
	pub fn compute_checksum(&mut self) {
		self.hdr_checksum = 0;

		let slice =
			unsafe { slice::from_raw_parts(self as *const _ as *const u8, size_of::<Self>()) };
		self.hdr_checksum = checksum::compute_rfc1071(slice);
	}
}

/// A builder for IPv4 packets.
pub struct IPv4Builder {
	/// The protocol ID.
	pub protocol: u8,
	/// The destination IPv4, in big-endian.
	pub dst_addr: [u8; 4],
}

impl TransmitBuilder for IPv4Builder {
	fn new(desc: &SocketDesc, sockaddr: &[u8]) -> EResult<Box<dyn TransmitBuilder>> {
		let sockaddr: &SockAddrIn =
			unsafe { util::reinterpret(sockaddr) }.ok_or_else(|| errno!(EINVAL))?;

		let protocol = (desc.protocol as u32)
			.try_into()
			.map_err(|_| errno!(EINVAL))?;
		Ok(Box::new(Self {
			protocol,
			dst_addr: sockaddr.sin_addr.to_be_bytes(),
		})? as _)
	}

	fn transmit<'chunk>(
		&self,
		mut buff: BuffList<'chunk>,
		next: &TransmitPipeline,
	) -> EResult<()> {
		let hdr_len = size_of::<IPv4Header>() as u16; // TODO add options support?

		let dscp = 0; // TODO
		let ecn = 0; // TODO

		// TODO check endianess
		let mut hdr = IPv4Header {
			version_ihl: 4 | (((hdr_len / 4) as u8) << 4),
			type_of_service: (dscp << 2) | ecn,
			total_length: hdr_len + buff.len() as u16,

			identification: 0,        // TODO
			flags_fragment_offset: 0, // TODO

			// TODO allow setting a different value
			ttl: DEFAULT_TTL,
			protocol: self.protocol,
			hdr_checksum: 0,

			src_addr: [0; 4], // IPADDR_ANY
			dst_addr: self.dst_addr,
		};
		hdr.compute_checksum();

		let hdr_buff = unsafe {
			slice::from_raw_parts::<u8>(&hdr as *const _ as *const _, size_of::<IPv4Header>())
		};

		buff.push_front(hdr_buff.into());
		next.transmit(buff)
	}
}

/// Structure providing connection informations for sockets with IPv4.
#[repr(C)]
#[derive(Clone)]
pub struct SockAddrIn {
	/// The family of the socket.
	sin_family: c_short,
	/// The port on which the connection is to be opened.
	sin_port: c_short,
	/// The destination address of the connection.
	sin_addr: u32,
	/// Padding.
	sin_zero: [u8; 8],
}

// TODO IPv6 builder

/// The IPv6 header (RFC 8200).
#[repr(C, packed)]
struct IPv6Header {
	/// The version, traffic class and flow label.
	version_traffic_class_flow_label: u32,

	/// The length of the payload.
	payload_length: u16,
	/// The type of the next header.
	next_header: u8,
	/// The number of hops remaining before discarding the packet.
	hop_limit: u8,

	/// Source address.
	src_addr: [u8; 16],
	/// Destination address.
	dst_addr: [u8; 16],
}

/// Structure representing an IPv6 address.
#[repr(C)]
#[derive(Clone, Copy)]
pub union In6Addr {
	__s6_addr: [u8; 16],
	__s6_addr16: [u16; 8],
	__s6_addr32: [u32; 4],
}

/// Structure providing connection informations for sockets with IPv6.
#[repr(C)]
#[derive(Clone)]
pub struct SockAddrIn6 {
	/// The family of the socket.
	sin6_family: c_short,
	/// The port on which the connection is to be opened.
	sin6_port: c_short,
	/// TODO doc
	sin6_flowinfo: u32,
	/// The destination address of the connection.
	sin6_addr: In6Addr,
	/// TODO doc
	sin6_scope_id: u32,
}
