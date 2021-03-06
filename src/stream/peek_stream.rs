use crate::predicate::{IntoMutPredicateMut, IntoPredicateMut, MutPredicateMut, PredicateMut};
use core::{
	convert::TryFrom,
	mem::MaybeUninit,
	num::NonZeroUsize,
	ops::{Add, AddAssign, Sub},
	pin::Pin,
	task::{Context, Poll},
};
use ergo_pin::ergo_pin;
use futures_core::{FusedStream, Stream};
use futures_util::StreamExt as _;
use pin_project::pin_project;
use tap::{Conv as _, Pipe as _};

// A neat generic implementation isn't yet possible because types of const generic parameters can't depend on other type parameters yet.
// TODO: Check maths terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Modular<const MODULE: usize>(pub usize);
impl<const MODULE: usize> From<Modular<MODULE>> for usize {
	fn from(modular: Modular<MODULE>) -> Self {
		modular.0
	}
}
impl<const MODULE: usize> From<&mut Modular<MODULE>> for usize {
	fn from(modular: &mut Modular<MODULE>) -> Self {
		modular.0
	}
}
impl<const MODULE: usize> TryFrom<usize> for Modular<MODULE> {
	type Error = ();

	fn try_from(linear: usize) -> Result<Self, ()> {
		(linear < MODULE).then(|| Modular(linear)).ok_or(())
	}
}
impl<const MODULE: usize> Sub for Modular<MODULE> {
	type Output = usize;

	fn sub(self, rhs: Self) -> Self::Output {
		if self.0 >= rhs.0 {
			self.0 - rhs.0
		} else {
			MODULE - rhs.0 + self.0
		}
	}
}
impl<Rhs, const MODULE: usize> Sub<&Rhs> for Modular<MODULE>
where
	Self: Sub<Rhs>,
	Rhs: Copy,
{
	type Output = <Self as Sub<Rhs>>::Output;

	fn sub(self, rhs: &Rhs) -> Self::Output {
		self - *rhs
	}
}
impl<Rhs, const MODULE: usize> Sub<Rhs> for &Modular<MODULE>
where
	Modular<MODULE>: Sub<Rhs>,
{
	type Output = <Modular<MODULE> as Sub<Rhs>>::Output;

	fn sub(self, rhs: Rhs) -> Self::Output {
		*self - rhs
	}
}
impl<const MODULE: usize> Add<usize> for Modular<MODULE> {
	type Output = Self;

	fn add(self, rhs: usize) -> Self::Output {
		Modular(
			self.0
				.checked_add(rhs % MODULE)
				.expect("`Module` overflow in `add_assign`")
				% MODULE,
		)
	}
}
impl<const MODULE: usize> Add<usize> for &Modular<MODULE> {
	type Output = Modular<MODULE>;

	fn add(self, rhs: usize) -> Self::Output {
		*self + rhs
	}
}
impl<const MODULE: usize> AddAssign<usize> for Modular<MODULE> {
	fn add_assign(&mut self, rhs: usize) {
		*self = *self + rhs;
	}
}

/// A fixed-size-buffered lookahead [`Stream`] adapter.
#[pin_project]
pub struct PeekStream<Input: FusedStream, const CAPACITY: usize> {
	#[pin]
	input: Input,
	buffer: [MaybeUninit<Input::Item>; CAPACITY],
	start: Modular<CAPACITY>,
	len: usize,
}
impl<Input: FusedStream, const CAPACITY: usize> Stream for PeekStream<Input, CAPACITY> {
	type Item = Input::Item;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.project();
		if *this.len > 0 {
			let i: usize = this.start.into();
			*this.start += 1;
			*this.len -= 1;
			unsafe { this.buffer[i].as_ptr().read() }
				.pipe(Some)
				.pipe(Poll::Ready)
		} else {
			this.input.poll_next(cx)
		}
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let (start, end) = self.input.size_hint();
		(
			start + self.len,
			end.and_then(|end| end.checked_add(self.len)),
		)
	}
}
impl<Input: FusedStream, const CAPACITY: usize> FusedStream for PeekStream<Input, CAPACITY> {
	fn is_terminated(&self) -> bool {
		self.len == 0 && self.input.is_terminated()
	}
}
impl<Input: FusedStream, const CAPACITY: usize> PeekStream<Input, CAPACITY> {
	pub async fn peek_1(self: Pin<&mut Self>) -> Option<&Input::Item> {
		self.peek_n(NonZeroUsize::new(1).expect("unreachable"))
			.await
	}
	pub async fn peek_1_mut(self: Pin<&mut Self>) -> Option<&mut Input::Item> {
		self.peek_n_mut(NonZeroUsize::new(1).expect("unreachable"))
			.await
	}

	/// Peeks `depth` items ahead in `self`.
	///
	/// # Panics
	///
	/// Iff `depth` exceeds `CAPACITY`.
	pub async fn peek_n(self: Pin<&mut Self>, depth: NonZeroUsize) -> Option<&Input::Item> {
		self.peek_n_mut(depth).await.map(|item| &*item)
	}

	/// Peeks `depth` items ahead in `self`,
	/// allowing the caller to mutate the peeked item if available.
	///
	/// # Panics
	///
	/// Iff `depth` exceeds `CAPACITY`.
	pub async fn peek_n_mut(self: Pin<&mut Self>, depth: NonZeroUsize) -> Option<&mut Input::Item> {
		assert!(
			depth.get() <= CAPACITY,
			"`depth` out of range `0..CAPACITY`"
		);
		let mut this = self.project();
		while *this.len < depth.get() {
			if this.input.is_terminated() {
				return None;
			} else {
				this.buffer[(*this.start + *this.len).conv::<usize>()] =
					this.input.next().await?.pipe(MaybeUninit::new);
				*this.len += 1;
			}
		}
		unsafe {
			// Safety: Assuredly written to directly above or earlier than that.
			&mut *this.buffer[(*this.start + depth.get()).conv::<usize>()].as_mut_ptr()
		}
		.pipe(Some)
	}

	/// Retrieves the next item only if it satisfies `predicate`.
	///
	/// * The conversion of `predicate` happens immediately.
	/// * Buffers the next item, if available.
	#[ergo_pin]
	pub async fn next_if<X>(
		mut self: Pin<&mut Self>,
		predicate: impl IntoPredicateMut<Input::Item, X>,
	) -> Option<Input::Item> {
		if pin!(predicate.into_predicate_mut())
			.test(self.as_mut().peek_1().await?)
			.await
		{
			self.next().await
		} else {
			None
		}
	}

	/// Retrieves the next item only if it satisfies `predicate`,
	/// optionally mutating it during the check.
	///
	/// * The conversion of `predicate` happens immediately.
	/// * Buffers the next item, if available.
	#[ergo_pin]
	pub async fn next_if_mut<X>(
		mut self: Pin<&mut Self>,
		predicate: impl IntoMutPredicateMut<Input::Item, X>,
	) -> Option<Input::Item> {
		if pin!(predicate.into_mut_predicate_mut())
			.test_mut(self.as_mut().peek_1_mut().await?)
			.await
		{
			self.next().await
		} else {
			None
		}
	}
}
