//! For Berkley sockets, a socket address allows to specify an address on which a socket is bound,
//! or a destination address.

use super::Address;
use crate::errno::EResult;
use crate::net::osi::DOMAINS;
use core::ffi::c_short;
use core::mem::size_of;

/// A socket address, represented by `sockaddr_*` structures in userspace.
pub trait SockAddr {
	/// Returns the socket address associated with the given bytes representation.
	fn from_bytes<'b>(buf: &'b [u8]) -> EResult<&'b dyn SockAddr>
	where
		Self: Sized;

	/// Returns the address in native endianness, if applicable.
	fn get_address(&self) -> Option<Address>;
	/// Returns the port in native endianness, if applicable.
	fn get_port(&self) -> Option<u16>;
}

/// [Socket address](SockAddr) constructor.
///
/// This type is to be used when requiring a pointer to the `new` function of the trait
/// [`SockAddr`].
pub type SockAddrCtor = for<'b> fn(&'b [u8]) -> EResult<&'b dyn SockAddr>;

/// Extracts the `sin_family` field of a `sockaddr_*` structure (see [`SockAddr`]) structure from
/// the given buffer.
///
/// If the buffer is not large enough, the function returns `None`.
pub fn extract_family(buf: &[u8]) -> Option<c_short> {
	let arr = buf[..size_of::<c_short>()].try_into().ok()?;
	Some(c_short::from_ne_bytes(arr))
}

/// Returns the socket address associated with the given bytes representation.
pub fn from_bytes<'b>(buf: &'b [u8]) -> EResult<&'b dyn SockAddr> {
	let family = extract_family(buf).ok_or_else(|| errno!(EINVAL))?;

	let guard = DOMAINS.lock();
	let (_, ctor) = guard.get(&(family as _)).ok_or_else(|| errno!(EINVAL))?;

	ctor(buf)
}
