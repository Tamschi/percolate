//! Projections asynchronously transform an input `A` into an output `B`.
//!
//! During this process, a reference to `self` is held.
//!
//! # Naming Scheme
//!
//! The traits in this module have names of the form `〚Into〛〚Fused〛〚Ref‖Mut〛Projection〚Mut〛`.
//!
//! ## `〚Into〛`
//!
//! This is analogous to the `Into` in [`IntoIterator`].
//!
//! Use traits with this fragment in their name with `impl` or as generic type constraints to accept certain closures directly.
//!
//! Each type that implements a [`Projection<A, B>`] trait **should** also implement the matching [`IntoProjection<A, B, IntoProj = Self>`](`IntoProjection`) trait as identity transformation.
//!
//! > It's unfortunately not possible to specify this constraint directly on [`Projection<A, B>`] without losing meaningful object safety there.
//!
//! ### Example
//!
//! ```
//! use ergo_pin::ergo_pin;
//! use percolate::projection::{IntoProjection, Projection};
//!
//! #[ergo_pin]
//! async fn project<A, B, P>(value: A, projection: impl IntoProjection<A, B, P>) -> B {
//!     pin!(
//!         projection.into_projection() // P: Projection<A, B>
//!     )                                // Pin<&mut P: Projection<A, B>>
//!         .into_ref()                  // Pin<&P: Projection<A, B>>
//!         .project(value)              // PinHandleMut<dyn Future<B>>
//!         .await                       // B
//! }
//! ```
//!
//! ### `.into_…()` Proxy
//!
//! As the ~~`Into`~~`Projection` traits in this module are object-safe,
//! it makes sense to use a proxy for the initial conversion:
//!
//! ```
//! use core::pin::Pin;
//! use ergo_pin::ergo_pin;
//! use percolate::projection::{IntoProjection, Projection};
//!
//! #[ergo_pin]
//! async fn project_heavy<A, B, P>(value: A, projection: impl IntoProjection<A, B, P>) -> B {
//!     return project_heavy_dyn(
//!         value,
//!         pin!(projection.into_projection()),
//!     ).await;
//!
//!     async fn project_heavy_dyn<A, B>(value: A, projection: Pin<&mut dyn Projection<A, B>>) -> B {
//!         // Do significant work in this function.
//!         projection.project(value).await
//!     }
//! }
//! ```
//!
//! The inner function is then monomorphic over the type of `projection`,
//! which can significantly reduce the generated executable size.
//!
//! TODO: Provide an attribute macro that can heuristically perform this transformation.
//!
//! ## Trailing `〚Mut〛`
//!
//! The projection itself is mutably borrowed.
//!
//! Note that most simple projections still require this to store their parameter,
//! as object-safety within a no-std crate doesn't leave room for temporary allocations.
//!
//! ### Example
//!
//! ```
//! use ergo_pin::ergo_pin;
//! use percolate::projection::{IntoProjectionMut, ProjectionMut};
//! use pollster::block_on;
//!
//! #[ergo_pin]
//! async fn project<A, B, P>(value: A, projection: impl IntoProjectionMut<A, B, P>) -> B {
//!     pin!(
//!         projection.into_projection_mut() // P: ProjectionMut<A, B>
//!     )                                    // Pin<&mut P: ProjectionMut<A, B>>
//!         .project(value)                  // PinHandleMut<dyn Future<B>>
//!         .await                           // B
//! }
//!
//! assert_eq!(block_on(project(1, |x| x + 1)), 2);
//! assert_eq!(block_on(project(1, |x| async { x + 1 })), 2);
//! ```

use crate::handles::PinHandleMut;
use core::{future::Future, pin::Pin};
use futures_core::FusedFuture;

mod async_mut;
mod fused_blocking_mut;
mod fused_ref_blocking_mut;

pub use async_mut::{from_async_mut, AsyncMut};
pub use fused_blocking_mut::{from_blocking_mut, FusedBlockingMut};
pub use fused_ref_blocking_mut::{from_ref_blocking_mut, FusedRefBlockingMut};

pub trait Projection<A, B>: ProjectionMut<A, B> {
	fn project(self: Pin<&Self>, value: A) -> PinHandleMut<'_, dyn '_ + Future<Output = B>>;
}

pub trait FusedProjection<A, B>: Projection<A, B> {
	fn project_fused(
		self: Pin<&Self>,
		value: A,
	) -> PinHandleMut<'_, dyn '_ + FusedFuture<Output = B>>;
}

pub trait ProjectionMut<A, B> {
	fn project(self: Pin<&mut Self>, value: A) -> PinHandleMut<'_, dyn '_ + Future<Output = B>>;
}

pub trait FusedProjectionMut<A, B>: ProjectionMut<A, B> {
	fn project_fused(
		self: Pin<&mut Self>,
		value: A,
	) -> PinHandleMut<'_, dyn '_ + FusedFuture<Output = B>>;
}

pub trait RefProjectionMut<A: ?Sized, B> {
	fn project_ref<'a>(
		self: Pin<&'a mut Self>,
		value: &'a A,
	) -> PinHandleMut<'a, dyn 'a + Future<Output = B>>;
}

pub trait FusedRefProjectionMut<A: ?Sized, B>: RefProjectionMut<A, B> {
	fn project_ref_fused<'a>(
		self: Pin<&'a mut Self>,
		value: &'a A,
	) -> PinHandleMut<'a, dyn 'a + FusedFuture<Output = B>>;
}

pub trait IntoProjection<A, B, P>: Sized + IntoProjectionMut<A, B, P>
where
	P: Projection<A, B> + IntoProjection<A, B, P>,
{
	#[must_use]
	fn into_projection(self) -> P;
}

pub trait IntoFusedProjection<A, B, P>:
	Sized + IntoProjection<A, B, P> + IntoFusedProjectionMut<A, B, P>
where
	P: FusedProjection<A, B> + IntoFusedProjection<A, B, P>,
{
	#[must_use]
	fn into_fused_projection(self) -> P;
}

pub trait IntoProjectionMut<A, B, P>: Sized
where
	P: ProjectionMut<A, B> + IntoProjectionMut<A, B, P>,
{
	#[must_use]
	fn into_projection_mut(self) -> P;
}

pub trait IntoFusedProjectionMut<A, B, P>: Sized + IntoProjectionMut<A, B, P>
where
	P: FusedProjectionMut<A, B> + IntoFusedProjectionMut<A, B, P>,
{
	#[must_use]
	fn into_fused_projection_mut(self) -> P;
}

pub trait IntoRefProjectionMut<A: ?Sized, B, P>: Sized
where
	P: RefProjectionMut<A, B> + IntoRefProjectionMut<A, B, P>,
{
	#[must_use]
	fn into_ref_projection_mut(self) -> P;
}

pub trait IntoFusedRefProjectionMut<A: ?Sized, B, P>:
	Sized + IntoRefProjectionMut<A, B, P>
where
	P: FusedRefProjectionMut<A, B> + IntoFusedRefProjectionMut<A, B, P>,
{
	#[must_use]
	fn into_fused_ref_projection_mut(self) -> P;
}
