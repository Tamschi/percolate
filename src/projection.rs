//! Projections asynchronously transform an input `A` into an output `B`.
//! During this process, a reference to `self` is held.

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

#[allow(clippy::module_name_repetitions)]
pub trait IntoRefProjectionMut<A: ?Sized, B>: Sized {
	type IntoRefProjectionMut: RefProjectionMut<A, B>;
	#[must_use]
	fn into_ref_projection_mut(self) -> Self::IntoRefProjectionMut;
}

#[allow(clippy::module_name_repetitions)]
pub trait IntoFusedRefProjectionMut<A: ?Sized, B>: IntoRefProjectionMut<A, B> {
	type IntoFusedRefProjectionMut: FusedRefProjectionMut<A, B>;
	#[must_use]
	fn into_fused_ref_projection_mut(self) -> Self::IntoFusedRefProjectionMut;
}

#[pin_project]
pub struct RefBlockingMut<P, A: ?Sized, B>
where
	P: FnMut(&A) -> B,
{
	projection: P,
	param: Option<NonNull<A>>,
}
unsafe impl<P, A: ?Sized, B> Send for RefBlockingMut<P, A, B>
where
	P: Send + FnMut(&A) -> B,
	A: Sync,
{
}
/// [`&RefBlockingMut`] is immutable.
unsafe impl<P, A: ?Sized, B> Sync for RefBlockingMut<P, A, B> where P: FnMut(&A) -> B {}

impl<P, A: ?Sized, B> IntoRefProjectionMut<A, B> for RefBlockingMut<P, A, B>
where
	P: FnMut(&A) -> B,
{
	type IntoRefProjectionMut = Self;
	fn into_ref_projection_mut(self) -> Self::IntoRefProjectionMut {
		self
	}
}

impl<P, A: ?Sized, B> IntoFusedRefProjectionMut<A, B> for RefBlockingMut<P, A, B>
where
	P: FnMut(&A) -> B,
{
	type IntoFusedRefProjectionMut = Self;
	fn into_fused_ref_projection_mut(self) -> Self::IntoRefProjectionMut {
		self
	}
}

impl<P, A: ?Sized, B> RefProjectionMut<A, B> for RefBlockingMut<P, A, B>
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

impl<P, A: ?Sized, B> FusedRefProjectionMut<A, B> for RefBlockingMut<P, A, B>
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
struct RefBlockingFuture<P, A: ?Sized, B>(#[pin] RefBlockingMut<P, A, B>)
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
pub fn from_ref_blocking_mut<P, A: ?Sized, B>(projection: P) -> RefBlockingMut<P, A, B>
where
	P: FnMut(&A) -> B,
{
	RefBlockingMut {
		projection,
		param: None,
	}
}

impl<P, A: ?Sized, B> IntoRefProjectionMut<A, B> for P
where
	P: FnMut(&A) -> B,
{
	type IntoRefProjectionMut = RefBlockingMut<P, A, B>;
	fn into_ref_projection_mut(self) -> Self::IntoRefProjectionMut {
		from_ref_blocking_mut(self)
	}
}
