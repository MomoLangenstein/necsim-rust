use necsim_core_bond::ClosedUnitF64;

use crate::{
    cogs::{Habitat, MathsCore},
    landscape::Location,
};

#[contract_trait]
pub trait SpeciationProbability<M: MathsCore, H: Habitat<M>>:
    crate::cogs::Backup + core::fmt::Debug
{
    #[must_use]
    #[debug_requires(
        habitat.is_location_habitable(location),
        "location is habitable"
    )]
    fn get_speciation_probability_at_location(
        &self,
        location: &Location,
        habitat: &H,
    ) -> ClosedUnitF64;
}
