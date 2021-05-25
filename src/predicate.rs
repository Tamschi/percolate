use crate::{
	handles::PinHandle,
	projection::{self, IntoRefProjectionMut, RefProjectionMut},
};
use core::{future::Future, pin::Pin};

pub trait PredicateMut<T: ?Sized>: RefProjectionMut<T, bool> {
	fn test<'a>(
		self: Pin<&'a mut Self>,
		value: &'a T,
	) -> PinHandle<'a, dyn 'a + Future<Output = bool>> {
		self.project(value)
	}
}
impl<P: ?Sized, T: ?Sized> PredicateMut<T> for P where P: RefProjectionMut<T, bool> {}

pub trait IntoPredicateMut<T: ?Sized>: IntoRefProjectionMut<T, bool> {
	type IntoPredicateMut: PredicateMut<T>;
	#[must_use]
	fn into_predicate_mut(self) -> Self::IntoPredicateMut;
}
impl<P, T: ?Sized> IntoPredicateMut<T> for P
where
	P: IntoRefProjectionMut<T, bool>,
{
	type IntoPredicateMut = Self::IntoRefProjectionMut;
	fn into_predicate_mut(self) -> Self::IntoPredicateMut {
		self.into_ref_projection_mut()
	}
}

#[must_use]
pub fn from_blocking<P, T: ?Sized>(predicate_mut: P) -> projection::RefBlockingMut<P, T, bool>
where
	P: FnMut(&T) -> bool,
{
	projection::from_ref_blocking_mut(predicate_mut)
}
