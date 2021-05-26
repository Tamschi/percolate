use super::{FusedProjectionMut, IntoFusedProjectionMut, IntoProjectionMut, ProjectionMut};
use crate::handles::{PinHandle, RunOnce, Runnable};
use core::{
	cell::UnsafeCell,
	intrinsics::transmute,
	pin::Pin,
	task::{Context, Poll},
};
use futures_core::{FusedFuture, Future};
use pin_project::pin_project;
use tap::Pipe;

#[pin_project]
pub struct FusedBlockingMut<P, A, B>
where
	P: FnMut(A) -> B,
{
	projection: UnsafeCell<P>,
	param: UnsafeCell<Option<A>>,
}
unsafe impl<P, A, B> Send for FusedBlockingMut<P, A, B>
where
	P: Send + FnMut(A) -> B,
	A: Sync,
{
}
/// [`&dyn FusedBlockingMut`] is immutable.
unsafe impl<P, A, B> Sync for FusedBlockingMut<P, A, B> where P: FnMut(A) -> B {}

impl<P, A, B> IntoProjectionMut<A, B> for FusedBlockingMut<P, A, B>
where
	P: FnMut(A) -> B,
{
	type IntoProjMut = Self;
	fn into_projection_mut(self) -> Self::IntoProjMut {
		self
	}
}

impl<P, A, B> IntoFusedProjectionMut<A, B> for FusedBlockingMut<P, A, B>
where
	P: FnMut(A) -> B,
{
	type IntoFusedProjMut = Self;
	fn into_fused_projection_mut(self) -> Self::IntoFusedProjMut {
		self
	}
}

impl<P, A, B> ProjectionMut<A, B> for FusedBlockingMut<P, A, B>
where
	P: FnMut(A) -> B,
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
	P: FnMut(A) -> B,
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
	P: FnMut(A) -> B;

impl<P, A, B> Future for FusedBlockingFuture<P, A, B>
where
	P: FnMut(A) -> B,
{
	type Output = B;
	fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
		let blocking = unsafe { Pin::new_unchecked(&*self.project().0.get()) };
		unsafe { &mut *blocking.param.get() }
			.take()
			.expect("`FusedBlockingFuture::poll` called twice")
			.pipe(|param| unsafe { &mut *blocking.projection.get() }(param))
			.pipe(Poll::Ready)
	}
}

impl<P, A, B> FusedFuture for FusedBlockingFuture<P, A, B>
where
	P: FnMut(A) -> B,
{
	fn is_terminated(&self) -> bool {
		unsafe { &*(*self.0.get()).param.get() }.is_none()
	}
}

#[repr(transparent)]
#[pin_project]
struct ClearFusedBlocking<P, A, B>(#[pin] FusedBlockingMut<P, A, B>)
where
	P: FnMut(A) -> B;
impl<P, A, B> Runnable<(), ()> for ClearFusedBlocking<P, A, B>
where
	P: FnMut(A) -> B,
{
	fn run(&self, _: ()) {
		unsafe { &mut *self.0.param.get() }.take().pipe(drop)
	}
}

#[must_use]
pub fn from_blocking_mut<P, A, B>(projection: P) -> FusedBlockingMut<P, A, B>
where
	P: FnMut(A) -> B,
{
	FusedBlockingMut {
		projection: projection.into(),
		param: None.into(),
	}
}

impl<P, A, B> IntoProjectionMut<A, B> for P
where
	P: FnMut(A) -> B,
{
	type IntoProjMut = FusedBlockingMut<P, A, B>;
	fn into_projection_mut(self) -> Self::IntoProjMut {
		from_blocking_mut(self)
	}
}

impl<P, A, B> IntoFusedProjectionMut<A, B> for P
where
	P: FnMut(A) -> B,
{
	type IntoFusedProjMut = FusedBlockingMut<P, A, B>;
	fn into_fused_projection_mut(self) -> Self::IntoFusedProjMut {
		from_blocking_mut(self)
	}
}
