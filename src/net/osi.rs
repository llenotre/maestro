//! The Open Systems Interconnection (OSI) model defines the architecure of a network stack.
//!
//! This module implements the concept of network stack with protocol layers.

use super::proto::ip::IPv4Builder;
use super::proto::DummyBuilder;
use super::proto::TransmitBuilder;
use super::proto::TransmitBuilderCtor;
use super::BuffList;
use super::SocketDesc;
use super::SocketDomain;
use super::SocketType;
use crate::errno::EResult;
use crate::util::boxed::Box;
use crate::util::container::hashmap::HashMap;
use crate::util::lock::Mutex;

/// Container of OSI layers 3 (network)
static DOMAINS: Mutex<HashMap<u32, TransmitBuilderCtor>> = Mutex::new(HashMap::new());
/// Container of OSI layers 4 (transport)
static PROTOCOLS: Mutex<HashMap<u32, TransmitBuilderCtor>> = Mutex::new(HashMap::new());

/// Container of default protocols ID for domain/type pairs.
///
/// If this container doesn't contain a pair, it is considered invalid.
static DEFAULT_PROTOCOLS: Mutex<HashMap<(u32, SocketType), u32>> = Mutex::new(HashMap::new());

/// A pipeline used to build a packet.
///
/// This enumeration acts as a linked list of protocol layers which are called one after the other.
pub enum TransmitPipeline {
	/// Wraps data from the previous layer into a packet according to the protocol associated with
	/// the current layer, then passes the result to the next layer.
	Wrap {
		/// The current layer.
		curr: Box<dyn TransmitBuilder>,
		/// The next layer.
		next: Box<TransmitPipeline>,
	},
	/// Transmits the packet.
	Flush {
		// TODO
	},
}

impl TransmitPipeline {
	pub fn transmit(&self, buff: BuffList<'_>) -> EResult<()> {
		match self {
			Self::Wrap {
				curr,
				next,
			} => curr.transmit(buff, next),

			Self::Flush {} => {
				// TODO
				todo!()
			}
		}
	}
}

impl TransmitPipeline {
	/// Creates a new transmit pipeline.
	///
	/// Arguments:
	/// - `desc` is the descriptor of the socket.
	/// - `sockaddr` is a buffer representing the socket address structure.
	///
	/// If the descriptor is invalid or if the stack cannot be created, the function returns an
	/// error.
	pub fn new(desc: &SocketDesc, sockaddr: &[u8]) -> EResult<Self> {
		let domain = {
			let guard = DOMAINS.lock();
			let ctor = guard
				.get(&desc.domain.get_id())
				.ok_or_else(|| errno!(EINVAL))?;
			ctor(desc, sockaddr)?
		};

		let protocol: u32 = if desc.protocol != 0 {
			desc.protocol as _
		} else {
			DEFAULT_PROTOCOLS
				.lock()
				.get(&(desc.domain.get_id(), desc.type_))
				.ok_or_else(|| errno!(EINVAL))?
				.clone()
		};
		let protocol = {
			let guard = PROTOCOLS.lock();
			let ctor = guard.get(&protocol).ok_or_else(|| errno!(EINVAL))?;
			ctor(desc, sockaddr)?
		};

		Ok(Self::Wrap {
			curr: protocol,
			next: Box::new(Self::Wrap {
				curr: domain,
				next: Box::new(Self::Flush {})?,
			})?,
		})
	}
}

/// Registers default domains/types/protocols.
pub fn init() -> EResult<()> {
	let domains = HashMap::try_from([
		// TODO unix
		(
			SocketDomain::AfInet.get_id(),
			IPv4Builder::new as TransmitBuilderCtor,
		),
		// TODO inet6
		// TODO netlink
		// TODO packet
	])?;
	let protocols = HashMap::try_from([
		// ICMP
		(1, DummyBuilder::new as TransmitBuilderCtor),
		// TODO tcp
		// TODO udp
	])?;
	let default_protocols = HashMap::try_from([
		// TODO unix

		// ((SocketDomain::AfInet.get_id(), SocketType::SockStream.get_id()), /* TODO: ipv4/tcp */),
		// ((SocketDomain::AfInet.get_id(), SocketType::SockDgram.get_id()), /* TODO: ipv4/udp */),

		// ((SocketDomain::AfInet6.get_id(), SocketType::SockStream.get_id()), /* TODO: ipv6/tcp */),
		// ((SocketDomain::AfInet6.get_id(), SocketType::SockDgram.get_id()), /* TODO: ipv6/udp */),

		// TODO netlink
		// TODO packet
	])?;

	*DOMAINS.lock() = domains;
	*PROTOCOLS.lock() = protocols;
	*DEFAULT_PROTOCOLS.lock() = default_protocols;

	Ok(())
}
