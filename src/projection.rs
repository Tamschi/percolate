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
//! use core::{ops::Deref, pin::Pin};
//! use ergo_pin::ergo_pin;
//! use percolate::projection::{IntoProjectionMut, ProjectionMut};
//!
//! #[ergo_pin]
//! //TODO: Make this work with a plain `IntoProjection`.
//! async fn project<A, B>(value: A, projection: impl IntoProjectionMut<A,B>) -> B {
//!     pin!(projection.into_projection_mut()).project(value).await
//! }
//! ```

use crate::handles::PinHandle;
use core::{
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

pub trait ProjectionMut<A, B> {
	fn project(self: Pin<&mut Self>, value: A) -> PinHandle<'_, dyn '_ + Future<Output = B>>;
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
	type IntoProj: Projection<A, B>;
	#[must_use]
	fn into_projection(self) -> Self::IntoProj;
}

pub trait IntoProjectionMut<A, B>: Sized {
	type IntoProjMut: ProjectionMut<A, B>;
	#[must_use]
	fn into_projection_mut(self) -> Self::IntoProjMut;
}

pub trait IntoRefProjectionMut<A: ?Sized, B>: Sized {
	type IntoRefProjMut: RefProjectionMut<A, B>;
	#[must_use]
	fn into_ref_projection_mut(self) -> Self::IntoRefProjMut;
}

pub trait IntoFusedRefProjectionMut<A: ?Sized, B>: Sized + IntoRefProjectionMut<A, B> {
	type IntoFusedRefProjMut: FusedRefProjectionMut<A, B>;
	#[must_use]
	fn into_fused_ref_projection_mut(self) -> Self::IntoFusedRefProjMut;
}

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
/// [`&RefBlockingMut`] is immutable.
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
			unsafe { transmute::<Pin<&mut Self>, Pin<&mut RefBlockingFuture<P, A, B>>>(self) },
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
			unsafe { transmute::<Pin<&mut Self>, Pin<&mut RefBlockingFuture<P, A, B>>>(self) },
			None,
		)
	}
}

#[repr(transparent)]
#[pin_project]
struct RefBlockingFuture<P, A: ?Sized, B>(#[pin] FusedRefBlockingMut<P, A, B>)
where
	P: FnMut(&A) -> B;

impl<P, A: ?Sized, B> Future for RefBlockingFuture<P, A, B>
where
	P: FnMut(&A) -> B,
{
	type Output = B;
	fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
		let blocking = &mut self.project().0;
		blocking
			.param
			.take()
			.expect("`RefBlockingFuture::poll` called twice")
			.pipe(|param_ptr| (blocking.projection)(unsafe { param_ptr.as_ref() }))
			.pipe(Poll::Ready)
	}
}

impl<P, A: ?Sized, B> FusedFuture for RefBlockingFuture<P, A, B>
where
	P: FnMut(&A) -> B,
{
	fn is_terminated(&self) -> bool {
		self.0.param.is_none()
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
