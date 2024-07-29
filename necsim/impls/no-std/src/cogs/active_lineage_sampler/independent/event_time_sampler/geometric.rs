use necsim_core::{
    cogs::{Habitat, HabitatPrimeableRng, MathsCore, PrimeableRng, RngSampler, TurnoverRate},
    landscape::IndexedLocation,
};
use necsim_core_bond::{NonNegativeF64, PositiveF64};

use super::EventTimeSampler;

#[expect(clippy::module_name_repetitions)]
#[derive(Clone, Debug)]
#[cfg_attr(feature = "cuda", derive(rust_cuda::lend::LendRustToCuda))]
pub struct GeometricEventTimeSampler {
    delta_t: PositiveF64,
}

impl GeometricEventTimeSampler {
    #[must_use]
    pub fn new(delta_t: PositiveF64) -> Self {
        Self { delta_t }
    }
}

#[contract_trait]
impl<M: MathsCore, H: Habitat<M>, G: PrimeableRng<M>, T: TurnoverRate<M, H>>
    EventTimeSampler<M, H, G, T> for GeometricEventTimeSampler
{
    #[inline]
    fn next_event_time_at_indexed_location_weakly_after(
        &self,
        indexed_location: &IndexedLocation,
        time: NonNegativeF64,
        habitat: &H,
        rng: &mut G,
        turnover_rate: &T,
    ) -> NonNegativeF64 {
        let event_probability_per_step = (turnover_rate
            .get_turnover_rate_at_location(indexed_location.location(), habitat)
            * self.delta_t)
            .neg_exp::<M>()
            .one_minus();

        #[expect(clippy::cast_possible_truncation)]
        #[expect(clippy::cast_sign_loss)]
        let mut time_step = M::floor(time.get() / self.delta_t.get()) as u64 + 1;

        loop {
            rng.prime_with_habitat(habitat, indexed_location, time_step);

            if rng.sample_event(event_probability_per_step) {
                break;
            }

            time_step += 1;
        }

        NonNegativeF64::from(time_step) * self.delta_t
    }
}
