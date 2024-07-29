use necsim_core::{
    cogs::{Habitat, MathsCore, TurnoverRate},
    landscape::Location,
};
use necsim_core_bond::{NonNegativeF64, PositiveF64};

#[derive(Clone, Debug)]
#[cfg_attr(feature = "cuda", derive(rust_cuda::lend::LendRustToCuda))]
#[expect(clippy::module_name_repetitions)]
pub struct UniformTurnoverRate {
    turnover_rate: PositiveF64,
}

impl Default for UniformTurnoverRate {
    fn default() -> Self {
        Self {
            turnover_rate: unsafe { PositiveF64::new_unchecked(0.5_f64) },
        }
    }
}

#[contract_trait]
impl<M: MathsCore, H: Habitat<M>> TurnoverRate<M, H> for UniformTurnoverRate {
    #[must_use]
    #[inline]
    fn get_turnover_rate_at_location(&self, _location: &Location, _habitat: &H) -> NonNegativeF64 {
        self.turnover_rate.into()
    }
}

impl UniformTurnoverRate {
    #[must_use]
    pub fn new(turnover_rate: PositiveF64) -> Self {
        Self { turnover_rate }
    }

    #[must_use]
    #[inline]
    pub fn get_uniform_turnover_rate(&self) -> PositiveF64 {
        self.turnover_rate
    }
}
