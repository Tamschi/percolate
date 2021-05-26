//! Projections asynchronously transform an input `A` into an output `B`.
//! During this process, a erence to `self` is held.

use crate::handles::PinHandle;
use core::{
	future::Future,
	mem::transmute,
	pin::Pin,
	task::{Context, Poll},
};
use futures_core::FusedFuture;
use pin_project::pin_project;
use tap::Pipe;

pub trait ProjectionMut<A, B> {
	fn project<'a>(self: Pin<&'a mut Self>, value: A) -> PinHandle<'a, dyn 'a + Future<Output = B>>
	where
		A: 'a;
}

pub trait FusedProjectionMut<A, B>: ProjectionMut<A, B> {
	fn project_fused<'a>(
		self: Pin<&'a mut Self>,
		value: A,
	) -> PinHandle<'a, dyn 'a + FusedFuture<Output = B>>
	where
		A: 'a;
}

#[allow(clippy::module_name_repetitions)]
pub trait IntoProjectionMut<A, B>: Sized {
	type IntoProjectionMut: ProjectionMut<A, B>;
	#[must_use]
	fn into_projection_mut(self) -> Self::IntoProjectionMut;
}

#[allow(clippy::module_name_repetitions)]
pub trait IntoFusedProjectionMut<A, B>: IntoProjectionMut<A, B> {
	type IntoFusedProjectionMut: FusedProjectionMut<A, B>;
	#[must_use]
	fn into_fused_projection_mut(self) -> Self::IntoFusedProjectionMut;
}

#[pin_project]
pub struct BlockingMut<P, A, B>
where
	P: FnMut(A) -> B,
{
	projection: P,
	param: Option<A>,
}
unsafe impl<P, A, B> Send for BlockingMut<P, A, B>
where
	P: Send + FnMut(A) -> B,
	A: Sync,
{
}
/// [`&BlockingMut`] is immutable.
unsafe impl<P, A, B> Sync for BlockingMut<P, A, B> where P: FnMut(A) -> B {}

impl<P, A, B> IntoProjectionMut<A, B> for BlockingMut<P, A, B>
where
	P: FnMut(A) -> B,
{
	type IntoProjectionMut = Self;
	fn into_projection_mut(self) -> Self::IntoProjectionMut {
		self
	}
}

impl<P, A, B> IntoFusedProjectionMut<A, B> for BlockingMut<P, A, B>
where
	P: FnMut(A) -> B,
{
	type IntoFusedProjectionMut = Self;
	fn into_fused_projection_mut(self) -> Self::IntoProjectionMut {
		self
	}
}

impl<P, A, B> ProjectionMut<A, B> for BlockingMut<P, A, B>
where
	P: FnMut(A) -> B,
{
	#[must_use]
	fn project<'a>(
		mut self: Pin<&'a mut Self>,
		value: A,
	) -> PinHandle<'a, dyn 'a + Future<Output = B>>
	where
		A: 'a,
	{
		self.param = Some(value);
		PinHandle::new(
			unsafe { transmute::<Pin<&mut Self>, Pin<&mut BlockingFuture<P, A, B>>>(self) },
			None,
		)
	}
}

impl<P, A, B> FusedProjectionMut<A, B> for BlockingMut<P, A, B>
where
	P: FnMut(A) -> B,
{
	#[must_use]
	fn project_fused<'a>(
		mut self: Pin<&'a mut Self>,
		value: A,
	) -> PinHandle<'a, dyn 'a + FusedFuture<Output = B>>
	where
		A: 'a,
	{
		self.param = Some(value);
		PinHandle::new(
			unsafe { transmute::<Pin<&mut Self>, Pin<&mut BlockingFuture<P, A, B>>>(self) },
			None,
		)
	}
}

#[repr(transparent)]
#[pin_project]
struct BlockingFuture<P, A, B>(#[pin] BlockingMut<P, A, B>)
where
	P: FnMut(A) -> B;

impl<P, A, B> Future for BlockingFuture<P, A, B>
where
	P: FnMut(A) -> B,
{
	type Output = B;
	fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
		let blocking = &mut self.project().0;
		blocking
			.param
			.take()
			.expect("`BlockingFuture::poll` called twice")
			.pipe(|param| (blocking.projection)(param))
			.pipe(Poll::Ready)
	}
}

impl<P, A, B> FusedFuture for BlockingFuture<P, A, B>
where
	P: FnMut(A) -> B,
{
	fn is_terminated(&self) -> bool {
		self.0.param.is_none()
	}
}

#[must_use]
pub fn from_blocking_mut<P, A, B>(projection: P) -> BlockingMut<P, A, B>
where
	P: FnMut(A) -> B,
{
	BlockingMut {
		projection,
		param: None,
	}
}

impl<P, A, B> IntoProjectionMut<A, B> for P
where
	P: FnMut(A) -> B,
{
	type IntoProjectionMut = BlockingMut<P, A, B>;
	fn into_projection_mut(self) -> Self::IntoProjectionMut {
		from_blocking_mut(self)
	}
}
