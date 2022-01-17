//! The APIC has an integrated timer whose frequency is base on the core crystal clock. It replaces
//! the PIT in multicore systems.

use core::cmp::min;
use crate::cpu::apic::APIC;
use crate::cpu;
use crate::time::timer::Timer;
use crate::util::boxed::Box;
use crate::util::math::rational::Rational;
use crate::util::ptr::WeakPtr;

/// The interrupt vector for the APIC timer.
pub const TIMER_VEC: u8 = 0x0;

/// Structure representing an APIC timer.
pub struct APICTimer {
	/// A pointer to the APIC associated with the timer.
	apic: WeakPtr<APIC>,

	/// The current timer frequency.
	freq: Rational,

	/// The number of ignored interrupts. This counter is reached when the limit given by the
	/// counter ratio is reached.
	interrupt_counter: u64,
	/// The callback being called `freq` times per second.
	callback: Option<Box<dyn FnMut()>>,
}

impl APICTimer {
	/// Creates a timer from the given `apic`.
	pub fn new(apic: WeakPtr<APIC>) -> Self {
		let s = Self {
			apic,

			freq: Rational::from(0),

			interrupt_counter: 0,
			callback: None,
		};

		// TODO Register interrupt handler

		s
	}

	/// The counter ratio is a fraction of the number of interrupts to ignore between each call to
	/// the callback and the value of the APIC timer counter.
	/// This value is computed according to the given frequency `freq`.
	fn get_counter_ratio(&self) -> Rational {
		self.get_max_frequency() / self.freq
	}
}

impl Timer for APICTimer {
	fn get_name(&self) -> &str {
		"APIC"
	}

	fn get_max_frequency(&self) -> Rational {
		// TODO Cache to avoid using CPUID everytimes?
		let (_, freq) = cpu::get_clock_ratios();
		// TODO Use ratio?
		Rational::from(freq as i64)
	}

	fn get_curr_frequency(&self) -> Rational {
		self.freq
	}

	fn set_curr_frequency(&mut self, frequency: Rational) {
		self.freq = frequency;

		let apic_mutex = self.apic.get();
		if apic_mutex.is_none() {
			return;
		}

		let apic_guard = apic_mutex.unwrap().lock();
		let apic = apic_guard.get();

		// If zero, disable the timer. Else, enable it and update its settings
		if self.freq == Rational::from(0) {
			let lvt_timer = apic.reg_read(super::REG_OFFSET_LVT_TIMER) | 0x400000;
			apic.reg_write(super::REG_OFFSET_LVT_TIMER, lvt_timer);
		} else {
			let counter_ratio = self.get_counter_ratio();

			// Computing the value of the counter and the divider
			let mut count = counter_ratio.get_denominator();
			let div_val = {
				let trailing_zeros = count.trailing_zeros();
				// Shifting the counter value by the amount of zeros to use the divider
				// The amount of shifts is limited to the maximum the divider can handle
				let shift = min(trailing_zeros, 7);
				count >>= shift;

				match shift {
					0 => 0b111,
					1 => 0b000,
					2 => 0b001,
					3 => 0b010,
					4 => 0b011,
					5 => 0b100,
					6 => 0b101,
					_ => 0b110,
				}
			};

			// Setting the divider
			let div = (apic.reg_read(super::REG_OFFSET_DCR) & 0xfffffff4)
				| ((div_val & 0b100) << 3) | (div_val & 0b11);
			apic.reg_write(super::REG_OFFSET_DCR, div);

			// Setting the counter
			apic.reg_write(super::REG_OFFSET_DCR, count as _);

			// Timer setup
			let lvt_timer = apic.reg_read(super::REG_OFFSET_LVT_TIMER) & 0xfff8ff00;
			apic.reg_write(super::REG_OFFSET_LVT_TIMER, lvt_timer | 0x20000 | (TIMER_VEC as u32));
		}
	}

	fn set_callback(&mut self, callback: Box<dyn FnMut()>) {
		self.callback = Some(callback);
	}
}
