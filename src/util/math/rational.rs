//! A rational number is a number which can be represented as the fraction of two integers: `a / b`

use core::cmp::Ordering;
use core::cmp::PartialEq;
use core::ops::Add;
use core::ops::AddAssign;
use core::ops::Div;
use core::ops::DivAssign;
use core::ops::Mul;
use core::ops::MulAssign;
use core::ops::Neg;
use core::ops::Sub;
use core::ops::SubAssign;

/// Structure implementing the representing a rational number.
#[derive(Copy, Clone)]
pub struct Rational {
	/// The numerator.
	a: i64,
	/// The denominator.
	b: i64,
}

impl Rational {
	/// Creates an instance from a given integer `n`.
	pub const fn from_integer(n: i64) -> Self {
		Self {
			a: n,
			b: 1,
		}
	}

	/// Creates an instance from two integers `a` and `b` such as the number equals `a / b`.
	/// If `b` is zero, the behaviour is undefined.
	pub const fn from_integers(a: i64, b: i64) -> Self {
		Self {
			a,
			b,
		}
	}

	/// Returns the numerator of the number.
	pub fn get_numerator(&self) -> i64 {
		self.a
	}

	/// Returns the denominator of the number.
	pub fn get_denominator(&self) -> i64 {
		self.b
	}

	/// Converts the value to the nearest integer value.
	pub fn as_integer(&self) -> i64 {
		self.a / self.b
	}
}

impl From<i64> for Rational {
	fn from(n: i64) -> Self {
		Self::from_integer(n)
	}
}

impl Neg for Rational {
	type Output = Self;

	fn neg(mut self) -> Self {
		self.a = -self.a;
		self
	}
}

impl Add for Rational {
	type Output = Self;

	fn add(self, other: Self) -> Self {
		// TODO Reduce
		Self {
			a: (self.a * other.b) + (other.a * self.b),
			b: self.b * other.b,
		}
	}
}

impl Add<i64> for Rational {
	type Output = Self;

	fn add(self, other: i64) -> Self {
		// TODO Reduce
		Self {
			a: self.a + (other * self.b),
			b: self.b,
		}
	}
}

impl Sub for Rational {
	type Output = Self;

	fn sub(self, other: Self) -> Self {
		// TODO Reduce
		Self {
			a: (self.a * other.b) - (other.a * self.b),
			b: self.b * other.b,
		}
	}
}

impl Sub<i64> for Rational {
	type Output = Self;

	fn sub(self, other: i64) -> Self {
		// TODO Reduce
		Self {
			a: self.a - (other * self.b),
			b: self.b,
		}
	}
}

impl Mul for Rational {
	type Output = Self;

	fn mul(self, other: Self) -> Self {
		// TODO Reduce
		Self {
			a: self.a * other.a,
			b: self.b * other.b,
		}
	}
}

impl Mul<i64> for Rational {
	type Output = Self;

	fn mul(self, other: i64) -> Self {
		// TODO Reduce
		Self {
			a: self.a * other,
			b: self.b,
		}
	}
}

impl Div for Rational {
	type Output = Self;

	fn div(self, other: Self) -> Self {
		// TODO Reduce
		Self {
			a: self.a * other.b,
			b: self.b * other.a,
		}
	}
}

impl Div<i64> for Rational {
	type Output = Self;

	fn div(self, other: i64) -> Self {
		// TODO Reduce
		Self {
			a: self.a,
			b: self.b * other,
		}
	}
}

impl AddAssign for Rational {
	fn add_assign(&mut self, other: Self) {
		*self = *self + other;
	}
}

impl SubAssign for Rational {
	fn sub_assign(&mut self, other: Self) {
		*self = *self - other;
	}
}

impl MulAssign for Rational {
	fn mul_assign(&mut self, other: Self) {
		*self = *self * other;
	}
}

impl DivAssign for Rational {
	fn div_assign(&mut self, other: Self) {
		*self = *self / other;
	}
}

impl PartialEq for Rational {
	fn eq(&self, other: &Self) -> bool {
		self.a == other.a && self.b == other.b
	}
}

impl PartialOrd for Rational {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some((self.a * other.b).cmp(&(other.a * self.b)))
	}
}
