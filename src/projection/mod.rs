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
//! > It's unfortunately (seemingly) not possible to specify this constraint directly on [`Projection<A, B>`] without losing meaningful object safety there.
//! >
//! > If you have any idea how to add this constraint without a lot of repeating boiler plate, then please let me know!
//! > I'll add it in a future version with a breaking Semver change in that case.
//!
//! ### Example
//!
//! ```
//! use ergo_pin::ergo_pin;
//! use percolate::projection::{IntoProjection, Projection};
//!
//! #[ergo_pin]
//! async fn project<A, B, X>(value: A, projection: impl IntoProjection<A, B, X>) -> B {
//!     pin!(
//!         projection.into_projection() // impl Projection<A, B>
//!     )                                // Pin<&mut impl Projection<A, B>>
//!         .into_ref()                  // Pin<&impl Projection<A, B>>
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
//! async fn project_heavy<A, B, X>(value: A, projection: impl IntoProjection<A, B, X>) -> B {
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
//! TODO: Provide an attribute macro that can heuristically perform this transformation, and also generate disambiguation args automatically.
//!
//! # `〚Fused〛`
//!
//! These projections generate [`FusedFuture`]s, which keep track of whether they are allowed to be [`.poll(…)`](Future::poll)ed again through their [`.is_terminated()`](`FusedFuture::is_terminated`) method.
//!
//! Note that some adapters, like [`AsyncMut`], are dependently fused.
//! If the underlying projection generates a [`FusedFuture`],
//! then so do they *when called through their respective `Fused…` trait's `…fused(…)` method*.
//!
//! > Practically speaking, it's the same underlying type, but **this is not guaranteed!**
//!
//! - casting: `Fused` -> ~~`Fused`~~
//!
//! # `〚Ref‖Mut〛`
//!
//! Types with `Ref` or `Mut` in their name in this position project from an "any lifetime" reference (`&'_ A`) or mutable reference (`&'_ mut A`) to their output type.
//! This reference stays borrowed for at least as long as the resulting [`PinHandleMut`]/[`Future`] exists.
//!
//! Without generic associated types or traits that can be implemented over the "any" lifetime,
//! it's unfortunately currently not possible to fully unify types over whether they accept their
//! parameter by value or by (mutable) reference with "any" lifetime (in a way that's nice to work with.
//! Some workarounds using [`fn`] as generic type parameter should work but would be less easy to use).
//!
//! The object-safe ~~`Into`~~ `Ref`/`Mut` traits like [`RefProjection<A, B>`] can already be expressed as trait aliases,
//! in this case for example over [`for<'a> Projection<&'a A, B>`](`Projection`), and are blanket-implemented as such.
//!
//! When implementing a custom projection, implement the underlying ~~`Ref`~~/~~`Mut`~~ trait for any lifetime.
//! The aliased shorthand then becomes available automatically.
//!
//! - casting: `Mut` -> `Ref` -> ~~`Mut`~~, `Ref` -> ~~`Ref`~~
//!
//! ## Trailing `〚Mut〛`
//!
//! The projection itself is mutably borrowed.
//!
//! Note that most simple projections still require this to store their parameter,
//! as object-safety within a no-std crate doesn't leave room for temporary allocations.
//!
//! - casting: ~~`Mut`~~ -> `Mut`
//!
//! ### Example
//!
//! ```
//! use ergo_pin::ergo_pin;
//! use percolate::projection::{AsyncMut, IntoProjectionMut, ProjectionMut};
//! use pollster::block_on;
//! use tap::Conv;
//!
//! #[ergo_pin]
//! async fn project<A, B, X>(value: A, projection: impl IntoProjectionMut<A, B, X>) -> B {
//!     pin!(
//!         projection.into_projection_mut() // impl ProjectionMut<A, B>
//!     )                                    // Pin<&mut impl ProjectionMut<A, B>>
//!         .project(value)                  // PinHandleMut<dyn Future<B>>
//!         .await                           // B
//! }
//!
//! assert_eq!(block_on(project(1, |x: u8| x + 1)), 2);
//! assert_eq!(
//!     block_on(project(
//!         1,
//!         // Type inference doesn't understand this on its own (yet), unfortunately.
//!         // We can instead pass the projection pre-converted.
//!         (|x| async move { x + 1 }).conv::<AsyncMut<_, _, _, _>>()),
//!     ),
//!     2,
//! );
//! ```

use crate::handles::PinHandleMut;
use core::{future::Future, pin::Pin};
use futures_core::FusedFuture;

mod async_mut;
mod fused_blocking_mut;
mod fused_mut_blocking_mut;
mod fused_ref_blocking_mut;

pub use async_mut::{from_async_mut, AsyncMut};
pub use fused_blocking_mut::{from_blocking_mut, FusedBlockingMut};
pub use fused_mut_blocking_mut::{from_mut_blocking_mut, FusedMutBlockingMut};
pub use fused_ref_blocking_mut::{from_ref_blocking_mut, FusedRefBlockingMut};

pub trait Projection<A, B>: ProjectionMut<A, B> {
	fn project(self: Pin<&Self>, value: A) -> PinHandleMut<'_, dyn '_ + Future<Output = B>>;
}

pub trait FusedProjection<A, B>: Projection<A, B> + FusedProjectionMut<A, B> {
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

/// alias: [`for<'a> Projection<&'a A, B>`](`Projection`)
pub trait RefProjection<A: ?Sized, B>:
	for<'a> Projection<&'a A, B> + MutProjection<A, B> + RefProjectionMut<A, B>
{
	fn project_ref<'a>(
		self: Pin<&'a Self>,
		value: &'a A,
	) -> PinHandleMut<'a, dyn 'a + Future<Output = B>>;
}
impl<P, A: ?Sized, B> RefProjection<A, B> for P
where
	P: for<'a> Projection<&'a A, B> + for<'a> Projection<&'a mut A, B>,
{
	fn project_ref<'a>(
		self: Pin<&'a Self>,
		value: &'a A,
	) -> PinHandleMut<'a, dyn 'a + Future<Output = B>> {
		self.project(value)
	}
}

/// alias: [`for<'a> Projection<&'a mut A, B>`](`Projection`)
pub trait MutProjection<A: ?Sized, B>:
	for<'a> Projection<&'a mut A, B> + MutProjectionMut<A, B>
{
	fn project_mut<'a>(
		self: Pin<&'a Self>,
		value: &'a mut A,
	) -> PinHandleMut<'a, dyn 'a + Future<Output = B>>;
}
impl<P, A: ?Sized, B> MutProjection<A, B> for P
where
	P: for<'a> Projection<&'a mut A, B>,
{
	fn project_mut<'a>(
		self: Pin<&'a Self>,
		value: &'a mut A,
	) -> PinHandleMut<'a, dyn 'a + Future<Output = B>> {
		self.project(value)
	}
}

/// alias: [`for<'a> FusedProjection<&'a A, B>`](`FusedProjection`)
pub trait FusedRefProjection<A: ?Sized, B>:
	for<'a> FusedProjection<&'a A, B>
	+ FusedMutProjection<A, B>
	+ RefProjection<A, B>
	+ FusedRefProjectionMut<A, B>
{
	fn project_ref_fused<'a>(
		self: Pin<&'a Self>,
		value: &'a A,
	) -> PinHandleMut<'a, dyn 'a + FusedFuture<Output = B>>;
}
impl<P, A: ?Sized, B> FusedRefProjection<A, B> for P
where
	P: for<'a> FusedProjection<&'a A, B> + for<'a> FusedProjection<&'a mut A, B>,
{
	fn project_ref_fused<'a>(
		self: Pin<&'a Self>,
		value: &'a A,
	) -> PinHandleMut<'a, dyn 'a + FusedFuture<Output = B>> {
		self.project_fused(value)
	}
}

/// alias: [`for<'a> FusedProjection<&'a mut A, B>`](`FusedProjection`)
pub trait FusedMutProjection<A: ?Sized, B>:
	for<'a> FusedProjection<&'a mut A, B> + MutProjection<A, B> + FusedMutProjectionMut<A, B>
{
	fn project_mut_fused<'a>(
		self: Pin<&'a Self>,
		value: &'a mut A,
	) -> PinHandleMut<'a, dyn 'a + FusedFuture<Output = B>>;
}
impl<P, A: ?Sized, B> FusedMutProjection<A, B> for P
where
	P: for<'a> FusedProjection<&'a mut A, B>,
{
	fn project_mut_fused<'a>(
		self: Pin<&'a Self>,
		value: &'a mut A,
	) -> PinHandleMut<'a, dyn 'a + FusedFuture<Output = B>> {
		self.project_fused(value)
	}
}

/// alias: [`for<'a> ProjectionMut<&'a A, B>`](`ProjectionMut`)
pub trait RefProjectionMut<A: ?Sized, B>:
	for<'a> ProjectionMut<&'a A, B> + MutProjectionMut<A, B>
{
	fn project_ref<'a>(
		self: Pin<&'a mut Self>,
		value: &'a A,
	) -> PinHandleMut<'a, dyn 'a + Future<Output = B>>;
}
impl<P, A: ?Sized, B> RefProjectionMut<A, B> for P
where
	P: for<'a> ProjectionMut<&'a A, B> + for<'a> ProjectionMut<&'a mut A, B>,
{
	fn project_ref<'a>(
		self: Pin<&'a mut Self>,
		value: &'a A,
	) -> PinHandleMut<'a, dyn 'a + Future<Output = B>> {
		self.project(value)
	}
}

/// alias: [`for<'a> ProjectionMut<&'a mut A, B>`](`ProjectionMut`)
pub trait MutProjectionMut<A: ?Sized, B>: for<'a> ProjectionMut<&'a mut A, B> {
	fn project_mut<'a>(
		self: Pin<&'a mut Self>,
		value: &'a mut A,
	) -> PinHandleMut<'a, dyn 'a + Future<Output = B>>;
}
impl<P, A: ?Sized, B> MutProjectionMut<A, B> for P
where
	P: for<'a> ProjectionMut<&'a mut A, B>,
{
	fn project_mut<'a>(
		self: Pin<&'a mut Self>,
		value: &'a mut A,
	) -> PinHandleMut<'a, dyn 'a + Future<Output = B>> {
		self.project(value)
	}
}

/// alias: [`for<'a> FusedProjectionMut<&'a A, B>`](`FusedProjectionMut`)
pub trait FusedRefProjectionMut<A: ?Sized, B>:
	for<'a> FusedProjectionMut<&'a A, B> + FusedMutProjectionMut<A, B> + RefProjectionMut<A, B>
{
	fn project_ref_fused<'a>(
		self: Pin<&'a mut Self>,
		value: &'a A,
	) -> PinHandleMut<'a, dyn 'a + FusedFuture<Output = B>>;
}
impl<P, A: ?Sized, B> FusedRefProjectionMut<A, B> for P
where
	P: for<'a> FusedProjectionMut<&'a A, B> + for<'a> FusedProjectionMut<&'a mut A, B>,
{
	fn project_ref_fused<'a>(
		self: Pin<&'a mut Self>,
		value: &'a A,
	) -> PinHandleMut<'a, dyn 'a + FusedFuture<Output = B>> {
		self.project_fused(value)
	}
}

/// alias: [`for<'a> FusedProjectionMut<&'a mut A, B>`](`FusedProjectionMut`)
pub trait FusedMutProjectionMut<A: ?Sized, B>:
	for<'a> FusedProjectionMut<&'a mut A, B> + MutProjectionMut<A, B>
{
	fn project_mut_fused<'a>(
		self: Pin<&'a mut Self>,
		value: &'a mut A,
	) -> PinHandleMut<'a, dyn 'a + FusedFuture<Output = B>>;
}
impl<P, A: ?Sized, B> FusedMutProjectionMut<A, B> for P
where
	P: for<'a> FusedProjectionMut<&'a mut A, B>,
{
	fn project_mut_fused<'a>(
		self: Pin<&'a mut Self>,
		value: &'a mut A,
	) -> PinHandleMut<'a, dyn 'a + FusedFuture<Output = B>> {
		self.project_fused(value)
	}
}

pub trait IntoProjection<A, B, X>: Sized + IntoProjectionMut<A, B, X> {
	type IntoProj: Projection<A, B> + IntoProjection<A, B, X>;
	#[must_use]
	fn into_projection(self) -> Self::IntoProj;
}

pub trait IntoFusedProjection<A, B, X>:
	Sized + IntoProjection<A, B, X> + IntoFusedProjectionMut<A, B, X>
{
	type IntoFusedProj: FusedProjection<A, B> + IntoFusedProjection<A, B, X>;
	#[must_use]
	fn into_fused_projection(self) -> Self::IntoProj;
}

pub trait IntoProjectionMut<A, B, X>: Sized {
	type IntoProjMut: ProjectionMut<A, B> + IntoProjectionMut<A, B, X>;
	#[must_use]
	fn into_projection_mut(self) -> Self::IntoProjMut;
}

pub trait IntoFusedProjectionMut<A, B, X>: Sized + IntoProjectionMut<A, B, X> {
	type IntoFusedProjMut: FusedProjectionMut<A, B> + IntoFusedProjectionMut<A, B, X>;
	#[must_use]
	fn into_fused_projection_mut(self) -> Self::IntoProjMut;
}

pub trait IntoRefProjection<A: ?Sized, B, X>:
	Sized
	+ for<'a> IntoProjection<&'a A, B, X>
	+ IntoMutProjection<A, B, X>
	+ IntoRefProjectionMut<A, B, X>
{
	type IntoRefProj: RefProjection<A, B> + IntoRefProjection<A, B, X>;
	#[must_use]
	fn into_ref_projection(self) -> Self::IntoRefProj;
}

pub trait IntoMutProjection<A: ?Sized, B, X>:
	Sized + for<'a> IntoProjection<&'a mut A, B, X> + IntoMutProjectionMut<A, B, X>
{
	type IntoMutProj: MutProjection<A, B> + IntoMutProjection<A, B, X>;
	#[must_use]
	fn into_mut_projection(self) -> Self::IntoMutProj;
}

pub trait IntoRefProjectionMut<A: ?Sized, B, X>:
	Sized + for<'a> IntoProjectionMut<&'a A, B, X> + IntoMutProjectionMut<A, B, X>
{
	type IntoRefProjMut: RefProjectionMut<A, B> + IntoRefProjectionMut<A, B, X>;
	#[must_use]
	fn into_ref_projection_mut(self) -> Self::IntoRefProjMut;
}

pub trait IntoMutProjectionMut<A: ?Sized, B, X>:
	Sized + for<'a> IntoProjectionMut<&'a mut A, B, X>
{
	type IntoMutProjMut: MutProjectionMut<A, B> + IntoMutProjectionMut<A, B, X>;
	#[must_use]
	fn into_mut_projection_mut(self) -> Self::IntoMutProjMut;
}

pub trait IntoFusedRefProjection<A: ?Sized, B, X>:
	Sized
	+ for<'a> IntoFusedProjection<&'a A, B, X>
	+ IntoFusedMutProjection<A, B, X>
	+ IntoRefProjection<A, B, X>
	+ IntoFusedRefProjectionMut<A, B, X>
{
	type IntoFusedRefProj: FusedRefProjection<A, B> + IntoFusedRefProjection<A, B, X>;
	#[must_use]
	fn into_fused_ref_projection(self) -> Self::IntoFusedRefProj;
}

pub trait IntoFusedMutProjection<A: ?Sized, B, X>:
	Sized
	+ for<'a> IntoFusedProjection<&'a mut A, B, X>
	+ IntoMutProjection<A, B, X>
	+ IntoFusedMutProjectionMut<A, B, X>
{
	type IntoFusedMutProj: FusedMutProjection<A, B> + IntoFusedMutProjection<A, B, X>;
	#[must_use]
	fn into_fused_mut_projection(self) -> Self::IntoFusedMutProj;
}

pub trait IntoFusedRefProjectionMut<A: ?Sized, B, X>:
	Sized
	+ for<'a> IntoFusedProjectionMut<&'a A, B, X>
	+ IntoFusedMutProjectionMut<A, B, X>
	+ IntoRefProjectionMut<A, B, X>
{
	type IntoFusedRefProjMut: FusedRefProjectionMut<A, B> + IntoFusedRefProjectionMut<A, B, X>;
	#[must_use]
	fn into_fused_ref_projection_mut(self) -> Self::IntoFusedRefProjMut;
}

pub trait IntoFusedMutProjectionMut<A: ?Sized, B, X>:
	Sized + for<'a> IntoFusedProjectionMut<&'a mut A, B, X> + IntoMutProjectionMut<A, B, X>
{
	type IntoFusedMutProjMut: FusedMutProjectionMut<A, B> + IntoFusedMutProjectionMut<A, B, X>;
	#[must_use]
	fn into_fused_mut_projection_mut(self) -> Self::IntoFusedMutProjMut;
}
