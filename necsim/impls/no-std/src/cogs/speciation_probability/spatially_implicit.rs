use necsim_core::{
    cogs::{Habitat, MathsCore, SpeciationProbability},
    landscape::Location,
};
use necsim_core_bond::{ClosedUnitF64, OpenClosedUnitF64 as PositiveUnitF64};

use crate::cogs::habitat::spatially_implicit::SpatiallyImplicitHabitat;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "cuda", derive(rust_cuda::lend::LendRustToCuda))]
#[expect(clippy::module_name_repetitions)]
pub struct SpatiallyImplicitSpeciationProbability {
    meta_speciation_probability: PositiveUnitF64,
}

impl SpatiallyImplicitSpeciationProbability {
    #[must_use]
    pub fn new(meta_speciation_probability: PositiveUnitF64) -> Self {
        Self {
            meta_speciation_probability,
        }
    }
}

#[contract_trait]
impl<M: MathsCore> SpeciationProbability<M, SpatiallyImplicitHabitat<M>>
    for SpatiallyImplicitSpeciationProbability
{
    #[must_use]
    #[inline]
    fn get_speciation_probability_at_location(
        &self,
        location: &Location,
        habitat: &SpatiallyImplicitHabitat<M>,
    ) -> ClosedUnitF64 {
        // By PRE, location must be habitable, i.e. either in the local or the meta
        //  habitat
        if habitat.local().get_extent().contains(location) {
            ClosedUnitF64::zero()
        } else {
            self.meta_speciation_probability.into()
        }
    }
}
