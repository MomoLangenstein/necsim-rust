use necsim_core::{
    cogs::{Habitat, HabitatPrimeableRng, MathsCore, PrimeableRng, RngSampler, TurnoverRate},
    landscape::IndexedLocation,
};
use necsim_core_bond::{NonNegativeF64, PositiveF64};

use super::EventTimeSampler;

// 2^64 / PHI
const INV_PHI: u64 = 0x9e37_79b9_7f4a_7c15_u64;

#[expect(clippy::module_name_repetitions)]
#[derive(Clone, Debug)]
#[cfg_attr(feature = "cuda", derive(rust_cuda::lend::LendRustToCuda))]
pub struct ExpEventTimeSampler {
    delta_t: PositiveF64,
}

impl ExpEventTimeSampler {
    #[must_use]
    pub fn new(delta_t: PositiveF64) -> Self {
        Self { delta_t }
    }
}

#[contract_trait]
impl<M: MathsCore, H: Habitat<M>, G: PrimeableRng<M>, T: TurnoverRate<M, H>>
    EventTimeSampler<M, H, G, T> for ExpEventTimeSampler
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
        // Safety: The turnover rate is >= 0.0 and must actually be > 0.0 because
        //  * `indexed_location` is habitable by this method's precondition
        //  * Therefore, `turnover_rate` must return a positive value by its
        //    postcondition
        let lambda = unsafe {
            PositiveF64::new_unchecked(
                turnover_rate
                    .get_turnover_rate_at_location(indexed_location.location(), habitat)
                    .get(),
            )
        };

        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let mut time_step = M::floor(time.get() / self.delta_t.get()) as u64;

        let mut event_time = NonNegativeF64::from(time_step) * self.delta_t;
        let mut time_slice_end = NonNegativeF64::from(time_step + 1) * self.delta_t;

        rng.prime_with_habitat(habitat, indexed_location, time_step);

        let mut sub_index: u64 = 0;

        loop {
            event_time += rng.sample_exponential(lambda);

            sub_index = sub_index.wrapping_add(INV_PHI);

            // The time slice is exclusive at time_slice_end
            if event_time >= time_slice_end {
                time_step += 1;
                sub_index = 0;

                event_time = time_slice_end;
                time_slice_end = NonNegativeF64::from(time_step + 1) * self.delta_t;

                rng.prime_with_habitat(habitat, indexed_location, time_step);
            } else if event_time > time {
                break;
            }
        }

        rng.prime_with_habitat(habitat, indexed_location, time_step.wrapping_add(sub_index));

        event_time
    }
}
