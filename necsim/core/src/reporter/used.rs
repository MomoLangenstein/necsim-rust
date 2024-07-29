use core::marker::PhantomData;

use crate::reporter::boolean::{Boolean, False, True};

#[expect(clippy::module_name_repetitions)]
#[repr(transparent)]
pub struct MaybeUsed<T, B: Boolean> {
    inner: T,
    _used: PhantomData<B>,
}

pub type Used<T> = MaybeUsed<T, True>;
pub type Ignored<T> = MaybeUsed<T, False>;

impl<'a, T, B: Boolean> From<&'a T> for &'a MaybeUsed<T, B> {
    fn from(inner: &'a T) -> Self {
        unsafe { &*core::ptr::from_ref(inner).cast() }
    }
}

impl<T, B: Boolean> From<T> for MaybeUsed<T, B> {
    fn from(inner: T) -> Self {
        Self {
            inner,
            _used: PhantomData::<B>,
        }
    }
}

impl<T> core::ops::Deref for Used<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T, B: Boolean> MaybeUsed<T, B> {
    pub fn maybe_use_in<F: FnOnce(&T)>(&self, func: F) {
        if B::VALUE {
            func(&self.inner);
        }
    }
}
