use crate::handles::PinHandle;
use core::{
	future::Future,
	mem::transmute,
	pin::Pin,
	ptr::NonNull,
	task::{Context, Poll},
};
use pin_project::pin_project;
use tap::Pipe;

pub trait RefProjectionMut<A: ?Sized, B> {
	fn project<'a>(
		self: Pin<&'a mut Self>,
		value: &'a A,
	) -> PinHandle<'a, dyn 'a + Future<Output = B>>;
}

#[allow(clippy::module_name_repetitions)]
pub trait IntoRefProjectionMut<A: ?Sized, B>: Sized {
	type IntoRefProjectionMut: RefProjectionMut<A, B>;
	#[must_use]
	fn into_ref_projection_mut(self) -> Self::IntoRefProjectionMut;
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
/// [`&RefBlockingMut`] is non-interactive.
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

impl<P, A: ?Sized, B> RefProjectionMut<A, B> for RefBlockingMut<P, A, B>
where
	P: FnMut(&A) -> B,
{
	#[must_use]
	fn project<'a>(
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
