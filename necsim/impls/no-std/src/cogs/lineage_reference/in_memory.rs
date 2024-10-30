use core::hash::Hash;

use necsim_core::cogs::{Backup, Habitat, LineageReference, MathsCore};

#[derive(PartialEq, Eq, Hash, Debug, TypeLayout)]
#[expect(clippy::module_name_repetitions)]
#[repr(transparent)]
pub struct InMemoryLineageReference(usize);

#[contract_trait]
impl Backup for InMemoryLineageReference {
    unsafe fn backup_unchecked(&self) -> Self {
        Self(self.0)
    }
}

impl<M: MathsCore, H: Habitat<M>> LineageReference<M, H> for InMemoryLineageReference {}

impl InMemoryLineageReference {
    /// # Safety
    ///
    /// A new [`InMemoryLineageReference`] must only be issued from a [`usize`]
    /// to create a new and unique reference. It is not allowed to issue
    /// multiple [`InMemoryLineageReference`]s from the same [`usize`].
    ///
    /// In case where a temporary and unexposed clone of the
    /// [`InMemoryLineageReference`] is required, use
    /// [`Backup::backup_unchecked`].
    #[must_use]
    pub unsafe fn issue(reference: usize) -> Self {
        Self(reference)
    }
}

impl From<InMemoryLineageReference> for usize {
    fn from(reference: InMemoryLineageReference) -> Self {
        reference.0
    }
}

impl<'a> From<&'a InMemoryLineageReference> for usize {
    fn from(reference: &'a InMemoryLineageReference) -> Self {
        reference.0
    }
}
