use super::{FusedProjectionMut, IntoFusedProjectionMut, IntoProjectionMut, ProjectionMut};
use crate::handles::{PinHandleMut, RunOnce, Runnable};
use core::{
	cell::UnsafeCell,
	marker::PhantomData,
	mem::transmute,
	pin::Pin,
	task::{Context, Poll},
};
use futures_core::{FusedFuture, Future};
use pin_project::pin_project;
use tap::Pipe;

/// [`From<`](`From`)[`P: FnMut(A) -> `](`FnMut`)`F: `[`〚Fused〛`](`FusedFuture`)[`Future<Output = B>`](`Future`)[`>`](`FnMut`)[`>`](`From`)
/// and [`〚Fused〛`](`FusedProjectionMut`)[`ProjectionMut<A, B>`]
#[pin_project]
pub struct AsyncMut<P, A, F, B>
where
	P: FnMut(A) -> F,
	F: Future<Output = B>,
{
	projection: P,
	#[pin]
	future: UnsafeCell<Option<F>>,
	_phantom: PhantomData<A>,
}

// region: threading
/// Only `P` is persistent. Whenever `F` is instantiated, there is a [`PinHandleMut`]`: `[`!Send`](`Send`) that drops it before the mutable borrow is released.
unsafe impl<P, A, F, B> Send for AsyncMut<P, A, F, B>
where
	P: Send + FnMut(A) -> F,
	F: Future<Output = B>,
{
}
/// [`&AsyncMut`](`AsyncMut`) is immutable and doesn't (publicly) allow access to stored data.
unsafe impl<P, A, F, B> Sync for AsyncMut<P, A, F, B>
where
	P: FnMut(A) -> F,
	F: Future<Output = B>,
{
}
// endregion
// region: projection impls
impl<P, A, F, B> IntoProjectionMut<A, B, Self> for AsyncMut<P, A, F, B>
where
	P: FnMut(A) -> F,
	F: Future<Output = B>,
{
	type IntoProjMut = Self;
	fn into_projection_mut(self) -> Self::IntoProjMut {
		self
	}
}
impl<P, A, F, B> IntoFusedProjectionMut<A, B, Self> for AsyncMut<P, A, F, B>
where
	P: FnMut(A) -> F,
	F: FusedFuture<Output = B>,
{
	type IntoFusedProjMut = Self;
	fn into_fused_projection_mut(self) -> Self::IntoFusedProjMut {
		self
	}
}

impl<P, A, F, B> ProjectionMut<A, B> for AsyncMut<P, A, F, B>
where
	P: FnMut(A) -> F,
	F: Future<Output = B>,
{
	#[must_use]
	fn project(
		mut self: Pin<&mut Self>,
		value: A,
	) -> PinHandleMut<'_, dyn '_ + Future<Output = B>> {
		let this = self.as_mut().project();
		unsafe { *this.future.get() = Some((this.projection)(value)) };
		let this = self.into_ref();
		PinHandleMut::new(
			unsafe { transmute::<Pin<&Self>, Pin<&mut AsyncMutFuture<P, A, F, B>>>(this) },
			Some(unsafe {
				RunOnce::new(transmute::<Pin<&Self>, &ClearAsyncMut<P, A, F, B>>(this))
			}),
		)
	}
}
impl<P, A, F, B> FusedProjectionMut<A, B> for AsyncMut<P, A, F, B>
where
	P: FnMut(A) -> F,
	F: FusedFuture<Output = B>,
{
	fn project_fused(
		mut self: Pin<&mut Self>,
		value: A,
	) -> PinHandleMut<'_, dyn '_ + FusedFuture<Output = B>> {
		let this = self.as_mut().project();
		unsafe { *this.future.get() = Some((this.projection)(value)) };
		let this = self.into_ref();
		PinHandleMut::new(
			unsafe { transmute::<Pin<&Self>, Pin<&mut AsyncMutFuture<P, A, F, B>>>(this) },
			Some(unsafe {
				RunOnce::new(transmute::<Pin<&Self>, &ClearAsyncMut<P, A, F, B>>(this))
			}),
		)
	}
}
// endregion
// region: future
#[repr(transparent)]
#[pin_project]
struct AsyncMutFuture<P, A, F, B>(
	#[pin] UnsafeCell<AsyncMut<P, A, F, B>>,
	PhantomData<*const ()>,
)
where
	P: FnMut(A) -> F,
	F: Future<Output = B>;

/// `F` may exist now, but `P` isn't accessed.
unsafe impl<P, A, F, B> Send for AsyncMutFuture<P, A, F, B>
where
	P: FnMut(A) -> F,
	F: Send + Future<Output = B>,
{
}
/// [`&AsyncMutFuture`] allows access to `F: `[`FusedFuture`], but `P` isn't accessed.
unsafe impl<P, A, F, B> Sync for AsyncMutFuture<P, A, F, B>
where
	P: FnMut(A) -> F,
	F: Sync + Future<Output = B>,
{
}

impl<P, A, F, B> Future for AsyncMutFuture<P, A, F, B>
where
	P: FnMut(A) -> F,
	F: Future<Output = B>,
{
	type Output = B;
	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let blocking = unsafe { Pin::new_unchecked(&*self.project().0.get()) };
		unsafe { &mut *blocking.future.get() }
			.as_mut()
			.expect("unreachable")
			.pipe(|x| unsafe { Pin::new_unchecked(x) })
			.poll(cx)
	}
}
impl<P, A, F, B> FusedFuture for AsyncMutFuture<P, A, F, B>
where
	P: FnMut(A) -> F,
	F: FusedFuture<Output = B>,
{
	fn is_terminated(&self) -> bool {
		let blocking = unsafe { &*self.0.get() };
		unsafe { &*blocking.future.get() }
			.as_ref()
			.expect("unreachable")
			.is_terminated()
	}
}
// endregion
// region: clear
#[repr(transparent)]
#[pin_project]
struct ClearAsyncMut<P, A, F, B>(#[pin] AsyncMut<P, A, F, B>, PhantomData<*mut ()>)
where
	P: FnMut(A) -> F,
	F: Future<Output = B>;
impl<P, A, F, B> Runnable<(), ()> for ClearAsyncMut<P, A, F, B>
where
	P: FnMut(A) -> F,
	F: Future<Output = B>,
{
	fn run(&self, _: ()) {
		unsafe { Pin::new_unchecked(&mut *self.0.future.get()) }.set(None)
	}
}
/// `F` may exist now, but P isn't accessed.
unsafe impl<P, A, F, B> Send for ClearAsyncMut<P, A, F, B>
where
	P: FnMut(A) -> F,
	F: Send + Future<Output = B>,
{
}
/// [`&ClearAsyncMut`] is immutable and doesn't (publicly) allow access to stored data.
unsafe impl<P, A, F, B> Sync for ClearAsyncMut<P, A, F, B>
where
	P: FnMut(A) -> F,
	F: Future<Output = B>,
{
}
// endregion
// region: conversions
impl<P, A, F, B> From<P> for AsyncMut<P, A, F, B>
where
	P: FnMut(A) -> F,
	F: Future<Output = B>,
{
	fn from(projection: P) -> Self {
		Self {
			projection,
			future: None.into(),
			_phantom: PhantomData,
		}
	}
}

impl<P, A, F, B> IntoProjectionMut<A, B, AsyncMut<P, A, F, B>> for P
where
	P: FnMut(A) -> F,
	F: Future<Output = B>,
{
	type IntoProjMut = AsyncMut<P, A, F, B>;
	fn into_projection_mut(self) -> Self::IntoProjMut {
		self.into()
	}
}

impl<P, A, F, B> IntoFusedProjectionMut<A, B, AsyncMut<P, A, F, B>> for P
where
	P: FnMut(A) -> F,
	F: FusedFuture<Output = B>,
{
	type IntoFusedProjMut = AsyncMut<P, A, F, B>;
	fn into_fused_projection_mut(self) -> Self::IntoProjMut {
		self.into()
	}
}

/// [`FnMut(A) -> `](`FnMut`)[`〚Fused〛`](`FusedFuture`)[`Future<Output = B>`](`Future`) → [`〚Fused〛`](`FusedProjectionMut`)[`ProjectionMut<A, B>`]
#[must_use]
pub fn from_async_mut<P, A, F, B>(projection: P) -> AsyncMut<P, A, F, B>
where
	P: FnMut(A) -> F,
	F: Future<Output = B>,
{
	projection.into()
}
// endregion
