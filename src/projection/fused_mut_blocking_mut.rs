use super::{
	FusedProjectionMut, IntoFusedMutProjectionMut, IntoFusedProjectionMut, IntoMutProjectionMut,
	IntoProjectionMut, ProjectionMut,
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

/// [`From<`](`From`)[`P: FnMut(&mut A) -> B>`](`FnMut`)[`>`](`From`)
/// and [`FusedMutProjectionMut<A, B>`](`super::FusedMutProjectionMut`)
#[pin_project]
pub struct FusedMutBlockingMut<P, A: ?Sized, B>
where
	P: FnMut(&mut A) -> B,
{
	projection: P,
	param: Option<NonNull<A>>,
}

// region: threading
unsafe impl<P, A: ?Sized, B> Send for FusedMutBlockingMut<P, A, B>
where
	P: Send + FnMut(&mut A) -> B,
	A: Sync,
{
}
/// [`&dyn MutBlockingMut`] is immutable.
unsafe impl<P, A: ?Sized, B> Sync for FusedMutBlockingMut<P, A, B> where P: FnMut(&mut A) -> B {}
// endregion
// region: projection impls
impl<P, A: ?Sized, B> IntoMutProjectionMut<A, B, Self> for FusedMutBlockingMut<P, A, B>
where
	P: FnMut(&mut A) -> B,
{
	type IntoMutProjMut = Self;
	fn into_mut_projection_mut(self) -> Self::IntoMutProjMut {
		self
	}
}

impl<P, A: ?Sized, B> IntoFusedMutProjectionMut<A, B, Self> for FusedMutBlockingMut<P, A, B>
where
	P: FnMut(&mut A) -> B,
{
	type IntoFusedMutProjMut = Self;
	fn into_fused_mut_projection_mut(self) -> Self::IntoFusedMutProjMut {
		self
	}
}

impl<'a, P, A: ?Sized, B> IntoProjectionMut<&'a mut A, B, Self> for FusedMutBlockingMut<P, A, B>
where
	P: FnMut(&mut A) -> B,
{
	type IntoProjMut = Self;
	fn into_projection_mut(self) -> Self::IntoProjMut {
		self
	}
}

impl<'a, P, A: ?Sized, B> IntoFusedProjectionMut<&'a mut A, B, Self>
	for FusedMutBlockingMut<P, A, B>
where
	P: FnMut(&mut A) -> B,
{
	type IntoFusedProjMut = Self;
	fn into_fused_projection_mut(self) -> Self::IntoFusedProjMut {
		self
	}
}

impl<'a, P, A: ?Sized, B> ProjectionMut<&'a mut A, B> for FusedMutBlockingMut<P, A, B>
where
	P: FnMut(&mut A) -> B,
{
	#[must_use]
	fn project(
		mut self: Pin<&mut Self>,
		value: &mut A,
	) -> PinHandleMut<'_, dyn '_ + Future<Output = B>> {
		self.param = Some(value.into());
		PinHandleMut::new(
			unsafe {
				transmute::<Pin<&mut Self>, Pin<&mut FusedMutBlockingFutureMut<P, A, B>>>(self)
			},
			None,
		)
	}
}

impl<'a, P, A: ?Sized, B> FusedProjectionMut<&'a mut A, B> for FusedMutBlockingMut<P, A, B>
where
	P: FnMut(&mut A) -> B,
{
	#[must_use]
	fn project_fused(
		mut self: Pin<&mut Self>,
		value: &mut A,
	) -> PinHandleMut<'_, dyn '_ + FusedFuture<Output = B>> {
		self.param = Some(value.into());
		PinHandleMut::new(
			unsafe {
				transmute::<Pin<&mut Self>, Pin<&mut FusedMutBlockingFutureMut<P, A, B>>>(self)
			},
			None,
		)
	}
}
// endregion
// region: future
#[repr(transparent)]
#[pin_project]
struct FusedMutBlockingFutureMut<P, A: ?Sized, B>(#[pin] FusedMutBlockingMut<P, A, B>)
where
	P: FnMut(&mut A) -> B;

impl<P, A: ?Sized, B> Future for FusedMutBlockingFutureMut<P, A, B>
where
	P: FnMut(&mut A) -> B,
{
	type Output = B;
	fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
		let blocking = &mut self.project().0;
		blocking
			.param
			.take()
			.expect("`MutBlockingFutureMut::poll` called twice")
			.pipe(|mut param_ptr| (blocking.projection)(unsafe { param_ptr.as_mut() }))
			.pipe(Poll::Ready)
	}
}

impl<P, A: ?Sized, B> FusedFuture for FusedMutBlockingFutureMut<P, A, B>
where
	P: FnMut(&mut A) -> B,
{
	fn is_terminated(&self) -> bool {
		self.0.param.is_none()
	}
}
// endregion
// region: conversions
impl<P, A: ?Sized, B> From<P> for FusedMutBlockingMut<P, A, B>
where
	P: FnMut(&mut A) -> B,
{
	fn from(projection: P) -> Self {
		Self {
			projection,
			param: None,
		}
	}
}

impl<P, A: ?Sized, B> IntoMutProjectionMut<A, B, FusedMutBlockingMut<P, A, B>> for P
where
	P: FnMut(&mut A) -> B,
{
	type IntoMutProjMut = FusedMutBlockingMut<P, A, B>;
	fn into_mut_projection_mut(self) -> Self::IntoMutProjMut {
		self.into()
	}
}

impl<P, A: ?Sized, B> IntoFusedMutProjectionMut<A, B, FusedMutBlockingMut<P, A, B>> for P
where
	P: FnMut(&mut A) -> B,
{
	type IntoFusedMutProjMut = FusedMutBlockingMut<P, A, B>;
	fn into_fused_mut_projection_mut(self) -> Self::IntoFusedMutProjMut {
		self.into()
	}
}

impl<'a, P, A: ?Sized, B> IntoProjectionMut<&'a mut A, B, FusedMutBlockingMut<P, A, B>> for P
where
	P: FnMut(&mut A) -> B,
{
	type IntoProjMut = FusedMutBlockingMut<P, A, B>;
	fn into_projection_mut(self) -> Self::IntoProjMut {
		self.into()
	}
}

impl<'a, P, A: ?Sized, B> IntoFusedProjectionMut<&'a mut A, B, FusedMutBlockingMut<P, A, B>> for P
where
	P: FnMut(&mut A) -> B,
{
	type IntoFusedProjMut = FusedMutBlockingMut<P, A, B>;
	fn into_fused_projection_mut(self) -> Self::IntoFusedProjMut {
		self.into()
	}
}

/// [`FnMut(&mut A) -> B`](`FnMut`) → [`FusedMutProjectionMut<A, B>`](`super::FusedMutProjectionMut`)
#[must_use]
pub fn from_mut_blocking_mut<P, A: ?Sized, B>(projection: P) -> FusedMutBlockingMut<P, A, B>
where
	P: FnMut(&mut A) -> B,
{
	projection.into()
}
// endregion
