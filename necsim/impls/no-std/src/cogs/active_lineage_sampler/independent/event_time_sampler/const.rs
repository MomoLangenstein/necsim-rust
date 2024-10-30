use necsim_core::{
    cogs::{Habitat, MathsCore, PrimeableRng, TurnoverRate},
    landscape::IndexedLocation,
};
use necsim_core_bond::{NonNegativeF64, PositiveF64};

use super::EventTimeSampler;

#[expect(clippy::module_name_repetitions)]
#[derive(Clone, Debug)]
#[cfg_attr(feature = "cuda", derive(rust_cuda::lend::LendRustToCuda))]
pub struct ConstEventTimeSampler {
    event_time: PositiveF64,
}

impl ConstEventTimeSampler {
    #[must_use]
    pub fn new(event_time: PositiveF64) -> Self {
        Self { event_time }
    }
}

#[contract_trait]
impl<M: MathsCore, H: Habitat<M>, G: PrimeableRng<M>, T: TurnoverRate<M, H>>
    EventTimeSampler<M, H, G, T> for ConstEventTimeSampler
{
    #[inline]
    fn next_event_time_at_indexed_location_weakly_after(
        &self,
        _indexed_location: &IndexedLocation,
        _time: NonNegativeF64,
        _habitat: &H,
        _rng: &mut G,
        _turnover_rate: &T,
    ) -> NonNegativeF64 {
        self.event_time.into()
    }
}
