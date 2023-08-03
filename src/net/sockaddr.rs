//! This module defines sockaddr structures used by system calls to define connection informations
//! on sockets.

use core::ffi::c_short;

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
