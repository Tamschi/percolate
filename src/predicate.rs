use futures_core::FusedFuture;

use crate::{
	handles::PinHandle,
	projection::{
		self, FusedProjectionMut, IntoFusedProjectionMut, IntoProjectionMut, ProjectionMut,
	},
};
use core::{future::Future, ops::Deref, pin::Pin};

pub trait PredicateMut<T: ?Sized>: for<'a> ProjectionMut<&'a dyn Deref<Target = T>, bool> {
	fn test(self: Pin<&mut Self>, value: &T) -> PinHandle<'_, dyn '_ + Future<Output = bool>> {
		self.project(value)
	}
}
impl<P: ?Sized, T: ?Sized> PredicateMut<T> for P where
	P: for<'a> ProjectionMut<&'a dyn Deref<Target = T>, bool>
{
}

pub trait FusedPredicateMut<T: ?Sized>:
	PredicateMut<T> + for<'a> FusedProjectionMut<&'a dyn Deref<Target = T>, bool>
{
	fn test(self: Pin<&mut Self>, value: &T) -> PinHandle<'_, dyn '_ + FusedFuture<Output = bool>> {
		self.project_fused(value)
	}
}
impl<P: ?Sized, T: ?Sized> FusedPredicateMut<T> for P where
	P: for<'a> FusedProjectionMut<&'a dyn Deref<Target = T>, bool>
{
}

pub trait IntoPredicateMut<T: ?Sized>:
	for<'a> IntoProjectionMut<&'a dyn Deref<Target = T>, bool>
{
	type IntoPredicateMut: PredicateMut<T>;
	#[must_use]
	fn into_predicate_mut(self) -> Self::IntoPredicateMut;
}
impl<'a, P, T: ?Sized> IntoPredicateMut<T> for P
where
	P: IntoProjectionMut<&'a dyn Deref<Target = T>, bool>,
{
	type IntoPredicateMut = Self::IntoProjectionMut;
	fn into_predicate_mut(self) -> Self::IntoPredicateMut {
		self.into_projection_mut()
	}
}

pub trait IntoFusedPredicateMut<T: ?Sized>:
	IntoPredicateMut<T> + for<'a> IntoFusedProjectionMut<&'a dyn Deref<Target = T>, bool>
{
	type IntoFusedPredicateMut: FusedPredicateMut<T>;
	#[must_use]
	fn into_fused_predicate_mut(self) -> Self::IntoFusedPredicateMut;
}
impl<'a, P, T: ?Sized> IntoFusedPredicateMut<T> for P
where
	P: IntoFusedProjectionMut<&'a dyn Deref<Target = T>, bool>,
{
	type IntoFusedPredicateMut = Self::IntoFusedProjectionMut;
	fn into_fused_predicate_mut(self) -> Self::IntoFusedPredicateMut {
		self.into_fused_projection_mut()
	}
}

#[must_use]
pub fn from_blocking<'a, P, T: ?Sized>(
	predicate_mut: P,
) -> projection::BlockingMut<P, &'a dyn Deref<Target = T>, bool>
where
	P: FnMut(&T) -> bool,
{
	projection::from_blocking_mut(predicate_mut)
}
