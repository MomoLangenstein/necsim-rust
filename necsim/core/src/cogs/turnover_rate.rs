use necsim_core_bond::NonNegativeF64;

use crate::{
    cogs::{Habitat, MathsCore},
    landscape::Location,
};

#[contract_trait]
pub trait TurnoverRate<M: MathsCore, H: Habitat<M>>:
    crate::cogs::Backup + core::fmt::Debug
{
    #[must_use]
    #[debug_requires(
        habitat.is_location_habitable(location),
        "location is habitable"
    )]
    #[debug_ensures(
        ret != 0.0_f64 || habitat.get_habitat_at_location(location) == 0_u32,
        "only returns zero if the location is inhabitable"
    )]
    fn get_turnover_rate_at_location(&self, location: &Location, habitat: &H) -> NonNegativeF64;
}
