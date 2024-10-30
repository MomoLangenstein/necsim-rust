use necsim_core::{
    cogs::{Habitat, MathsCore, PrimeableRng, TurnoverRate},
    landscape::IndexedLocation,
};
use necsim_core_bond::NonNegativeF64;

pub mod r#const;
pub mod exp;
pub mod fixed;
pub mod geometric;
pub mod poisson;

#[contract_trait]
pub trait EventTimeSampler<M: MathsCore, H: Habitat<M>, G: PrimeableRng<M>, T: TurnoverRate<M, H>>:
    Clone + core::fmt::Debug
{
    #[debug_requires(
        habitat.get_habitat_at_location(indexed_location.location()) > 0,
        "indexed_location must be habitable"
    )]
    #[debug_ensures(ret >= time, "the next event will happen weakly after time")]
    fn next_event_time_at_indexed_location_weakly_after(
        &self,
        indexed_location: &IndexedLocation,
        time: NonNegativeF64,
        habitat: &H,
        rng: &mut G,
        turnover_rate: &T,
    ) -> NonNegativeF64;
}
