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
//! ### Example
//!
//! ```
//! use ergo_pin::ergo_pin;
//! use percolate::projection::{IntoProjection, Projection};
//!
//! #[ergo_pin]
//! async fn project<A, B>(value: A, projection: impl IntoProjection<A, B>) -> B {
//!     pin!(
//!         projection.into_projection() // impl Projection<A, B>
//!     )                                // Pin<&mut impl Projection<A, B>>
//!         .into_ref()                  // Pin<&impl Projection<A, B>>
//!         .project(value)              // Future<B>
//!         .await                       // B
//! }
//! ```
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
//! async fn project<A, B>(value: A, projection: impl IntoProjectionMut<A, B>) -> B {
//!     pin!(
//!         projection.into_projection_mut() // impl ProjectionMut<A, B>
//!     )                                    // Pin<&mut impl ProjectionMut<A, B>>
//!         .project(value)                  // Future<B>
//!         .await                           // B
//! }
//!
//! assert_eq!(block_on(project(1, |x| x + 1)), 2);
//! ```

use crate::handles::{PinHandle, RunOnce, Runnable};
use core::{
	cell::UnsafeCell,
	future::Future,
	mem::transmute,
	pin::Pin,
	ptr::NonNull,
	task::{Context, Poll},
};
use futures_core::FusedFuture;
use pin_project::pin_project;
use tap::Pipe;

pub trait Projection<A, B>: ProjectionMut<A, B> {
	fn project(self: Pin<&Self>, value: A) -> PinHandle<'_, dyn '_ + Future<Output = B>>;
}

pub trait FusedProjection<A, B>: Projection<A, B> {
	fn project_fused(self: Pin<&Self>, value: A)
		-> PinHandle<'_, dyn '_ + FusedFuture<Output = B>>;
}

pub trait ProjectionMut<A, B> {
	fn project(self: Pin<&mut Self>, value: A) -> PinHandle<'_, dyn '_ + Future<Output = B>>;
}

pub trait FusedProjectionMut<A, B>: ProjectionMut<A, B> {
	fn project_fused(
		self: Pin<&mut Self>,
		value: A,
	) -> PinHandle<'_, dyn '_ + FusedFuture<Output = B>>;
}

pub trait RefProjectionMut<A: ?Sized, B> {
	fn project_ref<'a>(
		self: Pin<&'a mut Self>,
		value: &'a A,
	) -> PinHandle<'a, dyn 'a + Future<Output = B>>;
}

pub trait FusedRefProjectionMut<A: ?Sized, B>: RefProjectionMut<A, B> {
	fn project_ref_fused<'a>(
		self: Pin<&'a mut Self>,
		value: &'a A,
	) -> PinHandle<'a, dyn 'a + FusedFuture<Output = B>>;
}

pub trait IntoProjection<A, B>: Sized + IntoProjectionMut<A, B> {
	type IntoProj: Projection<A, B> + IntoProjection<A, B>;
	#[must_use]
	fn into_projection(self) -> Self::IntoProj;
}

pub trait IntoFusedProjection<A, B>:
	Sized + IntoProjection<A, B> + IntoFusedProjectionMut<A, B>
{
	type IntoFusedProj: FusedProjection<A, B> + IntoFusedProjection<A, B>;
	#[must_use]
	fn into_fused_projection(self) -> Self::IntoProj;
}

pub trait IntoProjectionMut<A, B>: Sized {
	type IntoProjMut: ProjectionMut<A, B> + IntoProjectionMut<A, B>;
	#[must_use]
	fn into_projection_mut(self) -> Self::IntoProjMut;
}

pub trait IntoFusedProjectionMut<A, B>: Sized + IntoProjectionMut<A, B> {
	type IntoFusedProjMut: FusedProjectionMut<A, B> + IntoFusedProjectionMut<A, B>;
	#[must_use]
	fn into_fused_projection_mut(self) -> Self::IntoProjMut;
}

pub trait IntoRefProjectionMut<A: ?Sized, B>: Sized {
	type IntoRefProjMut: RefProjectionMut<A, B> + IntoRefProjectionMut<A, B>;
	#[must_use]
	fn into_ref_projection_mut(self) -> Self::IntoRefProjMut;
}

pub trait IntoFusedRefProjectionMut<A: ?Sized, B>: Sized + IntoRefProjectionMut<A, B> {
	type IntoFusedRefProjMut: FusedRefProjectionMut<A, B> + IntoFusedRefProjectionMut<A, B>;
	#[must_use]
	fn into_fused_ref_projection_mut(self) -> Self::IntoFusedRefProjMut;
}

#[pin_project]
pub struct FusedBlockingMut<P, A, B>
where
	P: Fn(A) -> B,
{
	projection: P,
	param: UnsafeCell<Option<A>>,
}
unsafe impl<P, A, B> Send for FusedBlockingMut<P, A, B>
where
	P: Send + Fn(A) -> B,
	A: Sync,
{
}
/// [`&dyn FusedBlockingMut`] is immutable.
unsafe impl<P, A, B> Sync for FusedBlockingMut<P, A, B> where P: Fn(A) -> B {}

impl<P, A, B> IntoProjectionMut<A, B> for FusedBlockingMut<P, A, B>
where
	P: Fn(A) -> B,
{
	type IntoProjMut = Self;
	fn into_projection_mut(self) -> Self::IntoProjMut {
		self
	}
}

impl<P, A, B> IntoFusedProjectionMut<A, B> for FusedBlockingMut<P, A, B>
where
	P: Fn(A) -> B,
{
	type IntoFusedProjMut = Self;
	fn into_fused_projection_mut(self) -> Self::IntoFusedProjMut {
		self
	}
}

impl<P, A, B> ProjectionMut<A, B> for FusedBlockingMut<P, A, B>
where
	P: Fn(A) -> B,
{
	#[must_use]
	fn project(self: Pin<&mut Self>, value: A) -> PinHandle<'_, dyn '_ + Future<Output = B>> {
		unsafe { *self.param.get() = Some(value) };
		let this = self.into_ref();
		PinHandle::new(
			unsafe { transmute::<Pin<&Self>, Pin<&mut FusedBlockingFuture<P, A, B>>>(this) },
			Some(unsafe {
				RunOnce::new(transmute::<Pin<&Self>, &ClearFusedBlocking<P, A, B>>(this))
			}),
		)
	}
}

impl<P, A, B> FusedProjectionMut<A, B> for FusedBlockingMut<P, A, B>
where
	P: Fn(A) -> B,
{
	fn project_fused(
		self: Pin<&mut Self>,
		value: A,
	) -> PinHandle<'_, dyn '_ + FusedFuture<Output = B>> {
		unsafe { *self.param.get() = Some(value) };
		let this = self.into_ref();
		PinHandle::new(
			unsafe { transmute::<Pin<&Self>, Pin<&mut FusedBlockingFuture<P, A, B>>>(this) },
			Some(unsafe {
				RunOnce::new(transmute::<Pin<&Self>, &ClearFusedBlocking<P, A, B>>(this))
			}),
		)
	}
}

#[repr(transparent)]
#[pin_project]
struct FusedBlockingFuture<P, A, B>(#[pin] UnsafeCell<FusedBlockingMut<P, A, B>>)
where
	P: Fn(A) -> B;

impl<P, A, B> Future for FusedBlockingFuture<P, A, B>
where
	P: Fn(A) -> B,
{
	type Output = B;
	fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
		let blocking = unsafe { Pin::new_unchecked(&*self.project().0.get()) };
		unsafe { &mut *blocking.param.get() }
			.take()
			.expect("`RefBlockingFuture::poll` called twice")
			.pipe(|param| (blocking.projection)(param))
			.pipe(Poll::Ready)
	}
}

impl<P, A, B> FusedFuture for FusedBlockingFuture<P, A, B>
where
	P: Fn(A) -> B,
{
	fn is_terminated(&self) -> bool {
		unsafe { &*(*self.0.get()).param.get() }.is_none()
	}
}

#[repr(transparent)]
#[pin_project]
struct ClearFusedBlocking<P, A, B>(#[pin] FusedBlockingMut<P, A, B>)
where
	P: Fn(A) -> B;
impl<P, A, B> Runnable<(), ()> for ClearFusedBlocking<P, A, B>
where
	P: Fn(A) -> B,
{
	fn run(&self, _: ()) {
		unsafe { &mut *self.0.param.get() }.take().pipe(drop)
	}
}

//////

#[pin_project]
pub struct FusedRefBlockingMut<P, A: ?Sized, B>
where
	P: FnMut(&A) -> B,
{
	projection: P,
	param: Option<NonNull<A>>,
}
unsafe impl<P, A: ?Sized, B> Send for FusedRefBlockingMut<P, A, B>
where
	P: Send + FnMut(&A) -> B,
	A: Sync,
{
}
/// [`&dyn RefBlockingMut`] is immutable.
unsafe impl<P, A: ?Sized, B> Sync for FusedRefBlockingMut<P, A, B> where P: FnMut(&A) -> B {}

impl<P, A: ?Sized, B> IntoRefProjectionMut<A, B> for FusedRefBlockingMut<P, A, B>
where
	P: FnMut(&A) -> B,
{
	type IntoRefProjMut = Self;
	fn into_ref_projection_mut(self) -> Self::IntoRefProjMut {
		self
	}
}

impl<P, A: ?Sized, B> IntoFusedRefProjectionMut<A, B> for FusedRefBlockingMut<P, A, B>
where
	P: FnMut(&A) -> B,
{
	type IntoFusedRefProjMut = Self;
	fn into_fused_ref_projection_mut(self) -> Self::IntoFusedRefProjMut {
		self
	}
}

impl<P, A: ?Sized, B> RefProjectionMut<A, B> for FusedRefBlockingMut<P, A, B>
where
	P: FnMut(&A) -> B,
{
	#[must_use]
	fn project_ref<'a>(
		mut self: Pin<&'a mut Self>,
		value: &'a A,
	) -> PinHandle<'a, dyn 'a + Future<Output = B>> {
		self.param = Some(value.into());
		PinHandle::new(
			unsafe { transmute::<Pin<&mut Self>, Pin<&mut RefBlockingFutureMut<P, A, B>>>(self) },
			None,
		)
	}
}

impl<P, A: ?Sized, B> FusedRefProjectionMut<A, B> for FusedRefBlockingMut<P, A, B>
where
	P: FnMut(&A) -> B,
{
	#[must_use]
	fn project_ref_fused<'a>(
		mut self: Pin<&'a mut Self>,
		value: &'a A,
	) -> PinHandle<'a, dyn 'a + FusedFuture<Output = B>> {
		self.param = Some(value.into());
		PinHandle::new(
			unsafe { transmute::<Pin<&mut Self>, Pin<&mut RefBlockingFutureMut<P, A, B>>>(self) },
			None,
		)
	}
}

#[repr(transparent)]
#[pin_project]
struct RefBlockingFutureMut<P, A: ?Sized, B>(#[pin] FusedRefBlockingMut<P, A, B>)
where
	P: FnMut(&A) -> B;

impl<P, A: ?Sized, B> Future for RefBlockingFutureMut<P, A, B>
where
	P: FnMut(&A) -> B,
{
	type Output = B;
	fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
		let blocking = &mut self.project().0;
		blocking
			.param
			.take()
			.expect("`RefBlockingFutureMut::poll` called twice")
			.pipe(|param_ptr| (blocking.projection)(unsafe { param_ptr.as_ref() }))
			.pipe(Poll::Ready)
	}
}

impl<P, A: ?Sized, B> FusedFuture for RefBlockingFutureMut<P, A, B>
where
	P: FnMut(&A) -> B,
{
	fn is_terminated(&self) -> bool {
		self.0.param.is_none()
	}
}

#[must_use]
pub fn from_blocking<P, A, B>(projection: P) -> FusedBlockingMut<P, A, B>
where
	P: Fn(A) -> B,
{
	FusedBlockingMut {
		projection,
		param: None.into(),
	}
}

impl<P, A, B> IntoProjectionMut<A, B> for P
where
	P: Fn(A) -> B,
{
	type IntoProjMut = FusedBlockingMut<P, A, B>;
	fn into_projection_mut(self) -> Self::IntoProjMut {
		from_blocking(self)
	}
}

impl<P, A, B> IntoFusedProjectionMut<A, B> for P
where
	P: Fn(A) -> B,
{
	type IntoFusedProjMut = FusedBlockingMut<P, A, B>;
	fn into_fused_projection_mut(self) -> Self::IntoFusedProjMut {
		from_blocking(self)
	}
}

#[must_use]
pub fn from_ref_blocking_mut<P, A: ?Sized, B>(projection: P) -> FusedRefBlockingMut<P, A, B>
where
	P: FnMut(&A) -> B,
{
	FusedRefBlockingMut {
		projection,
		param: None,
	}
}

impl<P, A: ?Sized, B> IntoRefProjectionMut<A, B> for P
where
	P: FnMut(&A) -> B,
{
	type IntoRefProjMut = FusedRefBlockingMut<P, A, B>;
	fn into_ref_projection_mut(self) -> Self::IntoRefProjMut {
		from_ref_blocking_mut(self)
	}
}

impl<P, A: ?Sized, B> IntoFusedRefProjectionMut<A, B> for P
where
	P: FnMut(&A) -> B,
{
	type IntoFusedRefProjMut = FusedRefBlockingMut<P, A, B>;
	fn into_fused_ref_projection_mut(self) -> Self::IntoFusedRefProjMut {
		from_ref_blocking_mut(self)
	}
}
