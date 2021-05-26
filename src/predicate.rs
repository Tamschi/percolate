use futures_core::FusedFuture;

use crate::{
	handles::PinHandleMut,
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
	) -> PinHandleMut<'a, dyn 'a + Future<Output = bool>> {
		self.project_ref(value)
	}
}
impl<P: ?Sized, T: ?Sized> PredicateMut<T> for P where P: RefProjectionMut<T, bool> {}

pub trait FusedPredicateMut<T: ?Sized>: PredicateMut<T> + FusedRefProjectionMut<T, bool> {
	fn test<'a>(
		self: Pin<&'a mut Self>,
		value: &'a T,
	) -> PinHandleMut<'a, dyn 'a + FusedFuture<Output = bool>> {
		self.project_ref_fused(value)
	}
}
impl<P: ?Sized, T: ?Sized> FusedPredicateMut<T> for P where P: FusedRefProjectionMut<T, bool> {}

pub trait IntoPredicateMut<T: ?Sized, X>: IntoRefProjectionMut<T, bool, X> {
	type IntoPredMut: PredicateMut<T>;
	#[must_use]
	fn into_predicate_mut(self) -> Self::IntoPredMut;
}
impl<P, T: ?Sized, X> IntoPredicateMut<T, X> for P
where
	P: IntoRefProjectionMut<T, bool, X>,
{
	type IntoPredMut = Self::IntoRefProjMut;
	fn into_predicate_mut(self) -> Self::IntoPredMut {
		self.into_ref_projection_mut()
	}
}

pub trait IntoFusedPredicateMut<T: ?Sized, X>:
	IntoPredicateMut<T, X> + IntoFusedRefProjectionMut<T, bool, X>
{
	type IntoFusedPredMut: FusedPredicateMut<T>;
	#[must_use]
	fn into_fused_predicate_mut(self) -> Self::IntoFusedPredMut;
}
impl<P, T: ?Sized, X> IntoFusedPredicateMut<T, X> for P
where
	P: IntoFusedRefProjectionMut<T, bool, X>,
{
	type IntoFusedPredMut = Self::IntoFusedRefProjMut;
	fn into_fused_predicate_mut(self) -> Self::IntoFusedPredMut {
		self.into_fused_ref_projection_mut()
	}
}

#[must_use]
pub fn from_blocking_mut<P, T: ?Sized>(
	predicate_mut: P,
) -> projection::FusedRefBlockingMut<P, T, bool>
where
	P: FnMut(&T) -> bool,
{
	projection::from_ref_blocking_mut(predicate_mut)
}
