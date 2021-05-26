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

pub trait IntoPredicateMut<T: ?Sized, P>: IntoRefProjectionMut<T, bool, P>
where
	P: PredicateMut<T> + IntoPredicateMut<T, P>,
{
	#[must_use]
	fn into_predicate_mut(self) -> P;
}
impl<X, T: ?Sized, P> IntoPredicateMut<T, P> for X
where
	X: IntoRefProjectionMut<T, bool, P>,
	P: PredicateMut<T> + IntoPredicateMut<T, P>,
{
	fn into_predicate_mut(self) -> P {
		self.into_ref_projection_mut()
	}
}

pub trait IntoFusedPredicateMut<T: ?Sized, P>:
	IntoPredicateMut<T, P> + IntoFusedRefProjectionMut<T, bool, P>
where
	P: FusedPredicateMut<T> + IntoFusedPredicateMut<T, P>,
{
	#[must_use]
	fn into_fused_predicate_mut(self) -> P;
}
impl<X, T: ?Sized, P> IntoFusedPredicateMut<T, P> for X
where
	X: IntoFusedRefProjectionMut<T, bool, P>,
	P: FusedPredicateMut<T> + IntoFusedPredicateMut<T, P>,
{
	fn into_fused_predicate_mut(self) -> P {
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
