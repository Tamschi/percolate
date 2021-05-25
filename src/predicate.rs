use futures_core::FusedFuture;

use crate::{
	handles::PinHandle,
	projection::{
		self, FusedRefProjectionMut, IntoFusedRefProjectionMut, IntoRefProjectionMut,
		RefProjectionMut,
	},
};
use core::{future::Future, pin::Pin};

pub trait PredicateMut<T: ?Sized>: RefProjectionMut<T, bool> {
	fn test<'a>(
		self: Pin<&'a mut Self>,
		value: &'a T,
	) -> PinHandle<'a, dyn 'a + Future<Output = bool>> {
		self.project_ref(value)
	}
}
impl<P: ?Sized, T: ?Sized> PredicateMut<T> for P where P: RefProjectionMut<T, bool> {}

pub trait FusedPredicateMut<T: ?Sized>: PredicateMut<T> + FusedRefProjectionMut<T, bool> {
	fn test<'a>(
		self: Pin<&'a mut Self>,
		value: &'a T,
	) -> PinHandle<'a, dyn 'a + FusedFuture<Output = bool>> {
		self.project_ref_fused(value)
	}
}
impl<P: ?Sized, T: ?Sized> FusedPredicateMut<T> for P where P: FusedRefProjectionMut<T, bool> {}

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

pub trait IntoFusedPredicateMut<T: ?Sized>:
	IntoPredicateMut<T> + IntoFusedRefProjectionMut<T, bool>
{
	type IntoFusedPredicateMut: FusedPredicateMut<T>;
	#[must_use]
	fn into_fused_predicate_mut(self) -> Self::IntoFusedPredicateMut;
}
impl<P, T: ?Sized> IntoFusedPredicateMut<T> for P
where
	P: IntoFusedRefProjectionMut<T, bool>,
{
	type IntoFusedPredicateMut = Self::IntoFusedRefProjectionMut;
	fn into_fused_predicate_mut(self) -> Self::IntoFusedPredicateMut {
		self.into_fused_ref_projection_mut()
	}
}

#[must_use]
pub fn from_blocking<P, T: ?Sized>(predicate_mut: P) -> projection::RefBlockingMut<P, T, bool>
where
	P: FnMut(&T) -> bool,
{
	projection::from_ref_blocking_mut(predicate_mut)
}
