//! A few types needed to implement custom delegate logic on named types on stable.

use core::{
	future::Future,
	ops::{Deref, DerefMut},
	pin::Pin,
	task::{Context, Poll},
};
use futures_core::{FusedFuture, FusedStream, Stream};

/// Essentially [`Fn`].
pub trait Runnable<Args, R> {
	fn run(&self, args: Args) -> R;
}

/// Essentially [`FnOnce`].
pub struct RunOnce<'a, F: 'a + ?Sized>(&'a F);
impl<'a, F: ?Sized> RunOnce<'a, F> {
	pub fn new(f: &'a F) -> Self {
		Self(f)
	}
}
impl<'a> RunOnce<'a, dyn Runnable<(), ()>> {
	pub fn run(self) {
		self.0.run(())
	}
}

/// Dereferences to [`Pin<&'a mut T>`](`Pin`) and optionally runs custom drop logic via stored [`RunOnce<dyn Runnable>`]`.
pub struct PinHandleMut<'a, T: ?Sized> {
	pin: Pin<&'a mut T>,
	on_drop: Option<RunOnce<'a, dyn 'a + Runnable<(), ()>>>,
}

impl<'a, T: ?Sized> PinHandleMut<'a, T> {
	#[must_use]
	pub fn new(
		pin: Pin<&'a mut T>,
		on_drop: Option<RunOnce<'a, dyn 'a + Runnable<(), ()>>>,
	) -> Self {
		Self { pin, on_drop }
	}
}

impl<'a, T: ?Sized> Deref for PinHandleMut<'a, T> {
	type Target = Pin<&'a mut T>;
	fn deref(&self) -> &Self::Target {
		&self.pin
	}
}
impl<'a, T: ?Sized> DerefMut for PinHandleMut<'a, T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.pin
	}
}

impl<'a, T: ?Sized> Drop for PinHandleMut<'a, T> {
	fn drop(&mut self) {
		self.on_drop.take().map(RunOnce::run).unwrap_or_default()
	}
}

impl<'a, T: ?Sized> Future for PinHandleMut<'a, T>
where
	T: Future,
{
	type Output = T::Output;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		self.pin.as_mut().poll(cx)
	}
}

impl<'a, T: ?Sized> FusedFuture for PinHandleMut<'a, T>
where
	T: FusedFuture,
{
	fn is_terminated(&self) -> bool {
		self.pin.is_terminated()
	}
}

impl<'a, T: ?Sized> Stream for PinHandleMut<'a, T>
where
	T: Stream,
{
	type Item = T::Item;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		self.pin.as_mut().poll_next(cx)
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		self.pin.size_hint()
	}
}

impl<'a, T: ?Sized> FusedStream for PinHandleMut<'a, T>
where
	T: FusedStream,
{
	fn is_terminated(&self) -> bool {
		self.pin.is_terminated()
	}
}
