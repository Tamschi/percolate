use super::{IntoProjectionMut, ProjectionMut};
use crate::handles::{PinHandleMut, RunOnce, Runnable};
use core::{
	cell::UnsafeCell,
	marker::PhantomData,
	mem::transmute,
	pin::Pin,
	task::{Context, Poll},
};
use futures_core::Future;
use pin_project::pin_project;
use tap::Pipe;

/// [`From<`](`From`)[`P: FnMut(A) -> `](`FnMut`)[`F: Future<Output = B>`](`Future`)[`>`](`FnMut`)[`>`](`From`)` + `[`ProjectionMut<A, B>`]
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
unsafe impl<P, A, F, B> Send for AsyncMut<P, A, F, B>
where
	P: Send + FnMut(A) -> F,
	F: Send + Future<Output = B>,
{
}
/// [`&dyn AsyncMut`] is immutable and doesn't allow access to stored data.
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
	fn into_projection_mut(self) -> Self {
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
// endregion
// region: future
#[repr(transparent)]
#[pin_project]
struct AsyncMutFuture<P, A, F, B>(#[pin] UnsafeCell<AsyncMut<P, A, F, B>>)
where
	P: FnMut(A) -> F,
	F: Future<Output = B>;

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
// endregion
// region: clear
#[repr(transparent)]
#[pin_project]
struct ClearAsyncMut<P, A, F, B>(#[pin] AsyncMut<P, A, F, B>)
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
	fn into_projection_mut(self) -> AsyncMut<P, A, F, B> {
		self.into()
	}
}

/// [`FnMut(A) -> `](`FnMut`)[`Future<Output = B>`](`Future`) → [`ProjectionMut<A, B>`]
#[must_use]
pub fn from_async_mut<P, A, F, B>(projection: P) -> AsyncMut<P, A, F, B>
where
	P: FnMut(A) -> F,
	F: Future<Output = B>,
{
	projection.into()
}
// endregion
