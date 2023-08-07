//! Utilities for dynamically dispatched trait types.

use crate::errno::EResult;
use crate::util::boxed::Box;
use crate::util::TryClone;
use core::borrow::Borrow;

/// Trait allowing to clone from a `dyn Trait` type.
pub trait DynClone {
	/// Clones the object into a [`crate::util::boxed::Box`].
	fn dyn_clone_box(&self) -> EResult<Box<Self>>;
}

impl<T> DynClone for T
where
	T: TryClone,
{
	fn dyn_clone_box(&self) -> EResult<Box<T>> {
		Box::new(self.try_clone()?)
	}
}

/// An enumeration allowing to take several types of ownership on a `dyn` object.
pub enum DynOwnership<'a, T: ?Sized> {
	Borrowed(&'a T),
	Owned(Box<T>),
}

impl<'a, T: 'a + ?Sized> From<&'a T> for DynOwnership<'a, T> {
	fn from(val: &'a T) -> Self {
		Self::Borrowed(val)
	}
}

impl<T: 'static + ?Sized> From<Box<T>> for DynOwnership<'static, T> {
	fn from(val: Box<T>) -> Self {
		Self::Owned(val)
	}
}

impl<'a, T: 'a + DynClone + ?Sized> DynOwnership<'a, T> {
	fn to_owned(self) -> EResult<Self> {
		let val = match self {
			Self::Borrowed(t) => t.dyn_clone_box()?,
			Self::Owned(t) => t,
		};

		Ok(Self::Owned(val))
	}
}

impl<'a, T: 'a + ?Sized> Borrow<T> for DynOwnership<'a, T> {
	fn borrow(&self) -> &T {
		match self {
			Self::Borrowed(t) => t,
			Self::Owned(t) => &*t,
		}
	}
}

impl<'a, T: 'a + ?Sized> AsRef<T> for DynOwnership<'a, T> {
	fn as_ref(&self) -> &T {
		match self {
			Self::Borrowed(t) => t,
			Self::Owned(t) => &*t,
		}
	}
}
