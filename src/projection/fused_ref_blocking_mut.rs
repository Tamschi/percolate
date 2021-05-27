use super::{
	FusedRefProjectionMut, IntoFusedRefProjectionMut, IntoRefProjectionMut, RefProjectionMut,
};
use crate::handles::PinHandleMut;
use core::{
	mem::transmute,
	pin::Pin,
	ptr::NonNull,
	task::{Context, Poll},
};
use futures_core::{FusedFuture, Future};
use pin_project::pin_project;
use tap::Pipe;

/// [`From<`](`From`)[`P: FnMut(&A) -> B>`](`FnMut`)[`>`](`From`)
/// and [`FusedRefProjectionMut<A, B>`]
#[pin_project]
pub struct FusedRefBlockingMut<P, A: ?Sized, B>
where
	P: FnMut(&A) -> B,
{
	projection: P,
	param: Option<NonNull<A>>,
}

// region: threading
unsafe impl<P, A: ?Sized, B> Send for FusedRefBlockingMut<P, A, B>
where
	P: Send + FnMut(&A) -> B,
	A: Sync,
{
}
/// [`&dyn RefBlockingMut`] is immutable.
unsafe impl<P, A: ?Sized, B> Sync for FusedRefBlockingMut<P, A, B> where P: FnMut(&A) -> B {}
// endregion
// region: projection impls
impl<P, A: ?Sized, B> IntoRefProjectionMut<A, B, Self> for FusedRefBlockingMut<P, A, B>
where
	P: FnMut(&A) -> B,
{
	type IntoRefProjMut = Self;
	fn into_ref_projection_mut(self) -> Self::IntoRefProjMut {
		self
	}
}

impl<P, A: ?Sized, B> IntoFusedRefProjectionMut<A, B, Self> for FusedRefBlockingMut<P, A, B>
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
	) -> PinHandleMut<'a, dyn 'a + Future<Output = B>> {
		self.param = Some(value.into());
		PinHandleMut::new(
			unsafe {
				transmute::<Pin<&mut Self>, Pin<&mut FusedRefBlockingFutureMut<P, A, B>>>(self)
			},
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
	) -> PinHandleMut<'a, dyn 'a + FusedFuture<Output = B>> {
		self.param = Some(value.into());
		PinHandleMut::new(
			unsafe {
				transmute::<Pin<&mut Self>, Pin<&mut FusedRefBlockingFutureMut<P, A, B>>>(self)
			},
			None,
		)
	}
}
// endregion
// region: future
#[repr(transparent)]
#[pin_project]
struct FusedRefBlockingFutureMut<P, A: ?Sized, B>(#[pin] FusedRefBlockingMut<P, A, B>)
where
	P: FnMut(&A) -> B;

impl<P, A: ?Sized, B> Future for FusedRefBlockingFutureMut<P, A, B>
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

impl<P, A: ?Sized, B> FusedFuture for FusedRefBlockingFutureMut<P, A, B>
where
	P: FnMut(&A) -> B,
{
	fn is_terminated(&self) -> bool {
		self.0.param.is_none()
	}
}
// endregion
// region: conversions
impl<P, A: ?Sized, B> From<P> for FusedRefBlockingMut<P, A, B>
where
	P: FnMut(&A) -> B,
{
	fn from(projection: P) -> Self {
		Self {
			projection,
			param: None,
		}
	}
}

impl<P, A: ?Sized, B> IntoRefProjectionMut<A, B, FusedRefBlockingMut<P, A, B>> for P
where
	P: FnMut(&A) -> B,
{
	type IntoRefProjMut = FusedRefBlockingMut<P, A, B>;
	fn into_ref_projection_mut(self) -> Self::IntoRefProjMut {
		self.into()
	}
}

impl<P, A: ?Sized, B> IntoFusedRefProjectionMut<A, B, FusedRefBlockingMut<P, A, B>> for P
where
	P: FnMut(&A) -> B,
{
	type IntoFusedRefProjMut = FusedRefBlockingMut<P, A, B>;
	fn into_fused_ref_projection_mut(self) -> Self::IntoFusedRefProjMut {
		self.into()
	}
}

/// [`FnMut(&A) -> B`](`FnMut`) → [`FusedRefProjectionMut<A, B>`]
#[must_use]
pub fn from_ref_blocking_mut<P, A: ?Sized, B>(projection: P) -> FusedRefBlockingMut<P, A, B>
where
	P: FnMut(&A) -> B,
{
	projection.into()
}
// endregion
