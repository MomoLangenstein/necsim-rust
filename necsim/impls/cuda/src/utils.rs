use core::mem::MaybeUninit;

use rust_cuda::safety::StackOnly;

#[derive(TypeLayout)]
#[repr(transparent)]
#[doc(hidden)]
pub struct MaybeSome<T: StackOnly>(MaybeUninit<T>);

impl<T: StackOnly> MaybeSome<T> {
    #[cfg(not(target_os = "cuda"))]
    #[expect(non_upper_case_globals)] // FIXME: use expect
    pub(crate) const None: Self = Self(MaybeUninit::uninit());

    #[expect(non_snake_case)] // FIXME: use expect
    pub(crate) fn Some(value: T) -> Self {
        Self(MaybeUninit::new(value))
    }

    pub(crate) unsafe fn assume_some_read(&self) -> T {
        self.0.assume_init_read()
    }

    #[cfg(not(target_os = "cuda"))]
    pub(crate) unsafe fn assume_some_ref(&self) -> &T {
        self.0.assume_init_ref()
    }

    #[cfg(not(target_os = "cuda"))]
    pub(crate) unsafe fn assume_some_mut(&mut self) -> &mut T {
        self.0.assume_init_mut()
    }
}
