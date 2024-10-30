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
pub struct PoissonEventTimeSampler {
    delta_t: PositiveF64,
}

impl PoissonEventTimeSampler {
    #[must_use]
    pub fn new(delta_t: PositiveF64) -> Self {
        Self { delta_t }
    }
}

#[contract_trait]
impl<M: MathsCore, H: Habitat<M>, G: PrimeableRng<M>, T: TurnoverRate<M, H>>
    EventTimeSampler<M, H, G, T> for PoissonEventTimeSampler
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
        let lambda =
            turnover_rate.get_turnover_rate_at_location(indexed_location.location(), habitat);
        let lambda_per_step = lambda * self.delta_t;
        let no_event_probability_per_step = M::exp(-lambda_per_step.get());

        #[expect(clippy::cast_possible_truncation)]
        #[expect(clippy::cast_sign_loss)]
        let mut time_step = M::floor(time.get() / self.delta_t.get()) as u64;

        let (event_time, event_index) = loop {
            rng.prime_with_habitat(habitat, indexed_location, time_step);

            let number_events_at_time_steps = if no_event_probability_per_step > 0.0_f64 {
                // https://en.wikipedia.org/wiki/Poisson_distribution#cite_ref-Devroye1986_54-0
                let mut poisson = 0_u32;
                let mut prod = no_event_probability_per_step;
                let mut acc = no_event_probability_per_step;

                let u = rng.sample_uniform_closed_open();

                while u > acc && prod > 0.0_f64 {
                    poisson += 1;
                    prod *= lambda_per_step.get() / f64::from(poisson);
                    acc += prod;
                }

                poisson
            } else {
                // Fallback in case no_event_probability_per_step underflows
                #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let normal_as_poisson = rng
                    .sample_2d_normal(lambda_per_step.get(), lambda_per_step.sqrt::<M>())
                    .0
                    .max(0.0_f64) as u32;

                normal_as_poisson
            };

            let mut next_event = None;

            for event_index in 0..number_events_at_time_steps {
                let event_time = (NonNegativeF64::from(time_step)
                    + NonNegativeF64::from(rng.sample_uniform_closed_open()))
                    * self.delta_t;

                if event_time > time {
                    next_event = match next_event {
                        Some((later_event_time, _)) if later_event_time > event_time => {
                            Some((event_time, event_index))
                        },
                        Some(next_event) => Some(next_event),
                        None => Some((event_time, event_index)),
                    };
                }
            }

            match next_event {
                Some(next_event) => break next_event,
                None => time_step += 1,
            }
        };

        rng.prime_with_habitat(
            habitat,
            indexed_location,
            time_step + INV_PHI.wrapping_mul(u64::from(event_index + 1)),
        );

        event_time
    }
}
