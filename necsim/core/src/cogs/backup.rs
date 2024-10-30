use core::ops::Deref;

#[contract_trait]
pub trait Backup: Sized {
    #[must_use]
    unsafe fn backup_unchecked(&self) -> Self;

    fn backup(&self) -> BackedUp<Self> {
        BackedUp(unsafe { self.backup_unchecked() })
    }
}

#[derive(Debug, TypeLayout)]
#[repr(transparent)]
pub struct BackedUp<T: Backup>(pub(crate) T);

impl<T: Backup> Deref for BackedUp<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Clone> Backup for T {
    #[inline]
    default unsafe fn backup_unchecked(&self) -> Self {
        self.__contracts_impl_backup_unchecked()
    }

    #[inline]
    default unsafe fn __contracts_impl_backup_unchecked(&self) -> Self {
        self.clone()
    }
}

#[contract_trait]
impl<T: Copy> Backup for T {
    #[inline]
    unsafe fn backup_unchecked(&self) -> Self {
        *self
    }
}
