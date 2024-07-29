use core::marker::PhantomData;

use necsim_core::{
    cogs::{Backup, Habitat, LineageStore, MathsCore},
    lineage::{GlobalLineageReference, Lineage},
};

#[expect(clippy::module_name_repetitions)]
#[derive(Debug)]
#[cfg_attr(feature = "cuda", derive(rust_cuda::lend::LendRustToCuda))]
#[cfg_attr(feature = "cuda", cuda(free = "M", free = "H"))]
pub struct IndependentLineageStore<M: MathsCore, H: Habitat<M>> {
    marker: PhantomData<(M, H)>,
}

impl<M: MathsCore, H: Habitat<M>> Default for IndependentLineageStore<M, H> {
    fn default() -> Self {
        Self {
            marker: PhantomData::<(M, H)>,
        }
    }
}

#[contract_trait]
impl<M: MathsCore, H: Habitat<M>> Backup for IndependentLineageStore<M, H> {
    unsafe fn backup_unchecked(&self) -> Self {
        Self {
            marker: PhantomData::<(M, H)>,
        }
    }
}

#[contract_trait]
impl<M: MathsCore, H: Habitat<M>> LineageStore<M, H> for IndependentLineageStore<M, H> {
    type LocalLineageReference = GlobalLineageReference;

    fn with_capacity(_habitat: &H, _capacity: usize) -> Self {
        Self::default()
    }

    #[must_use]
    fn get_lineage_for_local_reference(
        &self,
        _reference: &GlobalLineageReference,
    ) -> Option<&Lineage> {
        None
    }
}
