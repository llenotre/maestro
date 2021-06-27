//! This module handles the PIT (Programmable Interrupt Timer) which allows to trigger
//! interruptions at a fixed interval.

use crate::idt;
use crate::io;
use crate::time::timer::Timer;
use crate::util::math::rational::Rational;

// TODO Recheck flags

/// PIT channel number 0.
const CHANNEL_0: u16 = 0x40;
/// PIT channel number 1.
const CHANNEL_1: u16 = 0x41;
/// PIT channel number 2.
const CHANNEL_2: u16 = 0x42;
/// The port to send a command to the PIT.
const PIT_COMMAND: u16 = 0x43;

/// The command to enable the PC speaker.
const BEEPER_ENABLE_COMMAND: u8 = 0x61;

/// Select PIT channel 0.
const SELECT_CHANNEL_0: u8 = 0x0;
/// Select PIT channel 1.
const SELECT_CHANNEL_1: u8 = 0x40;
/// Select PIT channel 2.
const SELECT_CHANNEL_2: u8 = 0x80;
/// TODO doc
const READ_BACK_COMMAND: u8 = 0xc0;

/// TODO doc
const ACCESS_LATCH_COUNT_VALUE: u8 = 0x0;
/// TODO doc
const ACCESS_LOBYTE: u8 = 0x10;
/// TODO doc
const ACCESS_HIBYTE: u8 = 0x20;
/// TODO doc
const ACCESS_LOBYTE_HIBYTE: u8 = 0x30;

/// Interrupt on terminal count.
const MODE_0: u8 = 0x0;
/// Hardware re-triggerable one-shot.
const MODE_1: u8 = 0x1;
/// Rate generator.
const MODE_2: u8 = 0x2;
/// Square wave generator.
const MODE_3: u8 = 0x3;
/// Software triggered strobe.
const MODE_4: u8 = 0x4;
/// Hardware triggered strobe.
const MODE_5: u8 = 0x5;

/// Tells whether the BCD mode is enabled.
const BCD_MODE: u8 = 0x1;

/// The base frequency of the PIT.
const BASE_FREQUENCY: Rational = Rational::from_integer(1193180);

/// Structure representing a timer using the PIT.
pub struct PITTimer {
	/// The channel ID.
	channel: u8,

	/// The current frequency of the PIT.
	current_frequency: Rational,
}

impl PITTimer {
	/// Creates a new instance.
	/// `channel` is the channel number for the timer.
	/// If the timer cannot be created (for example, if already bound to the specified channel, or
	/// if the channel doesn't exist), the function returns `None`.
	pub fn new(channel: u8) -> Option<Self> {
		// TODO check whether the channel is already bound
		let c = match channel {
			0 => Some(SELECT_CHANNEL_0),
			2 => Some(SELECT_CHANNEL_2),
			_ => None,
		}?;

		idt::wrap_disable_interrupts(|| {
			unsafe {
				io::outb(PIT_COMMAND, c | ACCESS_LOBYTE_HIBYTE | MODE_4);
			}
		});

		Some(Self {
			channel,

			current_frequency: Rational::from_integer(0),
		})
	}

	/// Sets the PIT divider value to `count`.
	fn set_value(&self, count: u16) {
		idt::wrap_disable_interrupts(|| {
			let c = match self.channel {
				0 => CHANNEL_0,
				2 => CHANNEL_2,

				_ => CHANNEL_0,
			};

			unsafe {
				io::outb(c, (count & 0xff) as u8);
				io::outb(c, ((count >> 8) & 0xff) as u8);
			}
		});
	}
}

impl Timer for PITTimer {
	fn get_name(&self) -> &str {
		"PIT"
	}

	fn get_max_frequency(&self) -> Rational {
		BASE_FREQUENCY
	}

	fn get_curr_frequency(&self) -> Rational {
		self.current_frequency
	}

	fn set_curr_frequency(&mut self, frequency: Rational) {
		let mut c = {
			if frequency != From::from(0) {
				(BASE_FREQUENCY / frequency).as_integer()
			} else {
				0
			}
		};
		c &= 0xffff;
		if c & !0xffff != 0 {
			c = 0;
		}

		self.current_frequency = frequency;
		self.set_value(c as u16);
	}
}

// TODO Disable when hitting Drop
