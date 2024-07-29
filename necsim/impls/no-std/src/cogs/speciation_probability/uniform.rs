use necsim_core::{
    cogs::{Habitat, MathsCore, SpeciationProbability},
    landscape::Location,
};
use necsim_core_bond::ClosedUnitF64;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "cuda", derive(rust_cuda::lend::LendRustToCuda))]
#[expect(clippy::module_name_repetitions)]
pub struct UniformSpeciationProbability {
    speciation_probability: ClosedUnitF64,
}

impl UniformSpeciationProbability {
    #[must_use]
    pub fn new(speciation_probability: ClosedUnitF64) -> Self {
        Self {
            speciation_probability,
        }
    }
}

#[contract_trait]
impl<M: MathsCore, H: Habitat<M>> SpeciationProbability<M, H> for UniformSpeciationProbability {
    #[must_use]
    #[inline]
    fn get_speciation_probability_at_location(
        &self,
        _location: &Location,
        _habitat: &H,
    ) -> ClosedUnitF64 {
        self.speciation_probability
    }
}
