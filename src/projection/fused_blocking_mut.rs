use super::{FusedProjectionMut, IntoFusedProjectionMut, IntoProjectionMut, ProjectionMut};
use crate::handles::{PinHandleMut, RunOnce, Runnable};
use core::{
	cell::UnsafeCell,
	mem::transmute,
	pin::Pin,
	task::{Context, Poll},
};
use futures_core::{FusedFuture, Future};
use pin_project::pin_project;
use tap::Pipe;

/// [`From<`](`From`)[`P: FnMut(A) -> B>`](`FnMut`)[`>`](`From`)` + `[`FusedProjectionMut<A, B>`]
#[pin_project]
pub struct FusedBlockingMut<P, A, B>
where
	P: FnMut(A) -> B,
{
	projection: UnsafeCell<P>,
	param: UnsafeCell<Option<A>>,
}

// region: threading
unsafe impl<P, A, B> Send for FusedBlockingMut<P, A, B>
where
	P: Send + FnMut(A) -> B,
	A: Send,
{
}
/// [`&dyn FusedBlockingMut`] is immutable and doesn't allow access to stored data.
unsafe impl<P, A, B> Sync for FusedBlockingMut<P, A, B> where P: FnMut(A) -> B {}
// endregion
// region: projection impls
impl<P, A, B> IntoProjectionMut<A, B, Self> for FusedBlockingMut<P, A, B>
where
	P: FnMut(A) -> B,
{
	fn into_projection_mut(self) -> Self {
		self
	}
}

impl<P, A, B> IntoFusedProjectionMut<A, B, Self> for FusedBlockingMut<P, A, B>
where
	P: FnMut(A) -> B,
{
	fn into_fused_projection_mut(self) -> Self {
		self
	}
}

impl<P, A, B> ProjectionMut<A, B> for FusedBlockingMut<P, A, B>
where
	P: FnMut(A) -> B,
{
	#[must_use]
	fn project(self: Pin<&mut Self>, value: A) -> PinHandleMut<'_, dyn '_ + Future<Output = B>> {
		unsafe { *self.param.get() = Some(value) };
		let this = self.into_ref();
		PinHandleMut::new(
			unsafe { transmute::<Pin<&Self>, Pin<&mut FusedBlockingMutFuture<P, A, B>>>(this) },
			Some(unsafe {
				RunOnce::new(transmute::<Pin<&Self>, &ClearFusedBlockingMut<P, A, B>>(
					this,
				))
			}),
		)
	}
}

impl<P, A, B> FusedProjectionMut<A, B> for FusedBlockingMut<P, A, B>
where
	P: FnMut(A) -> B,
{
	fn project_fused(
		self: Pin<&mut Self>,
		value: A,
	) -> PinHandleMut<'_, dyn '_ + FusedFuture<Output = B>> {
		unsafe { *self.param.get() = Some(value) };
		let this = self.into_ref();
		PinHandleMut::new(
			unsafe { transmute::<Pin<&Self>, Pin<&mut FusedBlockingMutFuture<P, A, B>>>(this) },
			Some(unsafe {
				RunOnce::new(transmute::<Pin<&Self>, &ClearFusedBlockingMut<P, A, B>>(
					this,
				))
			}),
		)
	}
}
// endregion
// region: future
#[repr(transparent)]
#[pin_project]
struct FusedBlockingMutFuture<P, A, B>(#[pin] UnsafeCell<FusedBlockingMut<P, A, B>>)
where
	P: FnMut(A) -> B;

impl<P, A, B> Future for FusedBlockingMutFuture<P, A, B>
where
	P: FnMut(A) -> B,
{
	type Output = B;
	fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
		let blocking = unsafe { Pin::new_unchecked(&*self.project().0.get()) };
		unsafe { &mut *blocking.param.get() }
			.take()
			.expect("`FusedBlockingMutFuture::poll` called twice")
			.pipe(|param| unsafe { &mut *blocking.projection.get() }(param))
			.pipe(Poll::Ready)
	}
}

impl<P, A, B> FusedFuture for FusedBlockingMutFuture<P, A, B>
where
	P: FnMut(A) -> B,
{
	fn is_terminated(&self) -> bool {
		unsafe { &*(*self.0.get()).param.get() }.is_none()
	}
}
// endregion
// region: clear
#[repr(transparent)]
#[pin_project]
struct ClearFusedBlockingMut<P, A, B>(#[pin] FusedBlockingMut<P, A, B>)
where
	P: FnMut(A) -> B;
impl<P, A, B> Runnable<(), ()> for ClearFusedBlockingMut<P, A, B>
where
	P: FnMut(A) -> B,
{
	fn run(&self, _: ()) {
		unsafe { &mut *self.0.param.get() }.take().pipe(drop)
	}
}
// endregion
// region: conversions
impl<P, A, B> From<P> for FusedBlockingMut<P, A, B>
where
	P: FnMut(A) -> B,
{
	fn from(projection: P) -> Self {
		Self {
			projection: projection.into(),
			param: None.into(),
		}
	}
}

impl<P, A, B> IntoProjectionMut<A, B, FusedBlockingMut<P, A, B>> for P
where
	P: FnMut(A) -> B,
{
	fn into_projection_mut(self) -> FusedBlockingMut<P, A, B> {
		self.into()
	}
}

impl<P, A, B> IntoFusedProjectionMut<A, B, FusedBlockingMut<P, A, B>> for P
where
	P: FnMut(A) -> B,
{
	fn into_fused_projection_mut(self) -> FusedBlockingMut<P, A, B> {
		self.into()
	}
}

/// [`FnMut(A) -> B`](`FnMut`) → [`FusedProjectionMut<A, B>`]
#[must_use]
pub fn from_blocking_mut<P, A, B>(projection: P) -> FusedBlockingMut<P, A, B>
where
	P: FnMut(A) -> B,
{
	projection.into()
}
// endregion
