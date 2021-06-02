//! <code><s>Mut</s>Predicate</code>s are `RefProjection`s towards [`bool`].  
//! `MutPredicate`s are `MutProjection`s towards [`bool`].
//!
//! Prefer using `Into` `Predicate` types over `Projection` types in your function signatures to make them more intuitively readable.
//!
//! # Example
//!
//! `Predicate`s are useful for all sorts of async combinators, like filtering a Future:
//!
//! ```
//! use core::{future::Future, pin::Pin};
//! use ergo_pin::ergo_pin;
//! use percolate::predicate::{IntoPredicateMut, PredicateMut};
//!
//! #[ergo_pin]
//! async fn filter_future<T, X>(
//!     source: impl Future<Output = T>,
//!     predicate: impl IntoPredicateMut<T, X>,
//! ) -> Option<T> {
//!     let item = source.await;
//!     pin!(predicate.into_predicate_mut())
//!         .test(&item)
//!         .await
//!         .then(move || item)
//! }
//! ```

use crate::{
	handles::PinHandleMut,
	projection::{
		self, FusedMutProjectionMut, FusedRefProjectionMut, IntoFusedMutProjectionMut,
		IntoFusedRefProjectionMut, IntoMutProjectionMut, IntoRefProjectionMut, MutProjectionMut,
		RefProjectionMut,
	},
};
use core::{future::Future, pin::Pin};
use futures_core::FusedFuture;

/// alias: [`RefProjectionMut<T, bool>`]
pub trait PredicateMut<T: ?Sized>: RefProjectionMut<T, bool> + MutPredicateMut<T> {
	fn test<'a>(
		self: Pin<&'a mut Self>,
		value: &'a T,
	) -> PinHandleMut<'a, dyn 'a + Future<Output = bool>> {
		self.project_ref(value)
	}
}
impl<P: ?Sized, T: ?Sized> PredicateMut<T> for P where P: RefProjectionMut<T, bool> {}

/// alias: [`MutProjectionMut<T, bool>`]
pub trait MutPredicateMut<T: ?Sized>: MutProjectionMut<T, bool> {
	fn test_mut<'a>(
		self: Pin<&'a mut Self>,
		value: &'a mut T,
	) -> PinHandleMut<'a, dyn 'a + Future<Output = bool>> {
		self.project_mut(value)
	}
}
impl<P: ?Sized, T: ?Sized> MutPredicateMut<T> for P where P: MutProjectionMut<T, bool> {}

/// alias: [`FusedRefProjectionMut<T, bool>`]
pub trait FusedPredicateMut<T: ?Sized>:
	FusedRefProjectionMut<T, bool> + PredicateMut<T> + FusedMutPredicateMut<T>
{
	fn test<'a>(
		self: Pin<&'a mut Self>,
		value: &'a T,
	) -> PinHandleMut<'a, dyn 'a + FusedFuture<Output = bool>> {
		self.project_ref_fused(value)
	}
}
impl<P: ?Sized, T: ?Sized> FusedPredicateMut<T> for P where P: FusedRefProjectionMut<T, bool> {}

/// alias: [`FusedMutProjectionMut<T, bool>`]
pub trait FusedMutPredicateMut<T: ?Sized>:
	FusedMutProjectionMut<T, bool> + MutPredicateMut<T>
{
	fn test<'a>(
		self: Pin<&'a mut Self>,
		value: &'a mut T,
	) -> PinHandleMut<'a, dyn 'a + FusedFuture<Output = bool>> {
		self.project_mut_fused(value)
	}
}
impl<P: ?Sized, T: ?Sized> FusedMutPredicateMut<T> for P where P: FusedMutProjectionMut<T, bool> {}

/// alias: [`IntoRefProjectionMut<T, bool, X>`]
pub trait IntoPredicateMut<T: ?Sized, X>:
	IntoRefProjectionMut<T, bool, X> + IntoMutPredicateMut<T, X>
{
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

/// alias: [`IntoMutProjectionMut<T, bool, X>`]
pub trait IntoMutPredicateMut<T: ?Sized, X>: IntoMutProjectionMut<T, bool, X> {
	type IntoMutPredMut: MutPredicateMut<T>;
	#[must_use]
	fn into_mut_predicate_mut(self) -> Self::IntoMutPredMut;
}
impl<P, T: ?Sized, X> IntoMutPredicateMut<T, X> for P
where
	P: IntoMutProjectionMut<T, bool, X>,
{
	type IntoMutPredMut = Self::IntoMutProjMut;
	fn into_mut_predicate_mut(self) -> Self::IntoMutPredMut {
		self.into_mut_projection_mut()
	}
}

/// alias: [`IntoFusedRefProjectionMut<T, bool, X>`]
pub trait IntoFusedPredicateMut<T: ?Sized, X>:
	IntoFusedRefProjectionMut<T, bool, X> + IntoPredicateMut<T, X> + IntoFusedMutPredicateMut<T, X>
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

/// alias: [`IntoFusedMutProjectionMut<T, bool, X>`]
pub trait IntoFusedMutPredicateMut<T: ?Sized, X>:
	IntoFusedMutProjectionMut<T, bool, X> + IntoMutPredicateMut<T, X>
{
	type IntoFusedMutPredMut: FusedMutPredicateMut<T>;
	#[must_use]
	fn into_fused_mut_predicate_mut(self) -> Self::IntoFusedMutPredMut;
}
impl<P, T: ?Sized, X> IntoFusedMutPredicateMut<T, X> for P
where
	P: IntoFusedMutProjectionMut<T, bool, X>,
{
	type IntoFusedMutPredMut = Self::IntoFusedMutProjMut;
	fn into_fused_mut_predicate_mut(self) -> Self::IntoFusedMutPredMut {
		self.into_fused_mut_projection_mut()
	}
}

/// alias: [`projection::from_ref_blocking_mut(…)`](`projection::from_ref_blocking_mut`)
#[must_use]
pub fn from_blocking_mut<P, T: ?Sized>(predicate_mut: P) -> FusedBlockingMut<P, T>
where
	P: FnMut(&T) -> bool,
{
	projection::from_ref_blocking_mut(predicate_mut)
}

/// alias: [`projection::FusedRefBlockingMut`]
pub type FusedBlockingMut<P, T> = projection::FusedRefBlockingMut<P, T, bool>;
