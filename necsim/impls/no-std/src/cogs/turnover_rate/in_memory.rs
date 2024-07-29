#![allow(non_local_definitions)] // FIXME: displaydoc

use alloc::sync::Arc;

use necsim_core::{
    cogs::{Habitat, MathsCore, TurnoverRate},
    landscape::Location,
};
use necsim_core_bond::NonNegativeF64;

use crate::{array2d::Array2D, cogs::habitat::in_memory::InMemoryHabitat};

#[expect(clippy::module_name_repetitions)]
#[derive(Clone, Debug)]
#[cfg_attr(feature = "cuda", derive(rust_cuda::lend::LendRustToCuda))]
pub struct InMemoryTurnoverRate {
    #[cfg_attr(feature = "cuda", cuda(embed))]
    turnover_rate: Arc<[NonNegativeF64]>,
}

#[contract_trait]
impl<M: MathsCore> TurnoverRate<M, InMemoryHabitat<M>> for InMemoryTurnoverRate {
    #[must_use]
    #[inline]
    fn get_turnover_rate_at_location(
        &self,
        location: &Location,
        habitat: &InMemoryHabitat<M>,
    ) -> NonNegativeF64 {
        let extent = habitat.get_extent();

        self.turnover_rate
            .get(
                (location.y().wrapping_sub(extent.origin().y()) as usize)
                    * usize::from(extent.width())
                    + (location.x().wrapping_sub(extent.origin().x()) as usize),
            )
            .copied()
            .unwrap_or_else(NonNegativeF64::zero)
    }
}

#[expect(clippy::module_name_repetitions)]
#[derive(displaydoc::Display, Debug)]
pub enum InMemoryTurnoverRateError {
    /// There is some location with zero turnover and non-zero habitat.
    ZeroTurnoverHabitat,
}

impl InMemoryTurnoverRate {
    /// # Errors
    ///
    /// Returns `InMemoryTurnoverRateError::ZeroTurnoverHabitat` iff there is
    ///  zero turnover at any location with non-zero habitat.
    pub fn new<M: MathsCore>(
        turnover_rate: Array2D<NonNegativeF64>,
        habitat: &InMemoryHabitat<M>,
    ) -> Result<Self, InMemoryTurnoverRateError> {
        if habitat
            .get_extent()
            .iter()
            .zip(turnover_rate.elements_row_major_iter())
            .all(|(location, turnover)| {
                (*turnover != 0.0_f64) || (habitat.get_habitat_at_location(&location) == 0)
            })
        {
            Ok(Self {
                turnover_rate: Arc::from(turnover_rate.into_row_major().into_boxed_slice()),
            })
        } else {
            Err(InMemoryTurnoverRateError::ZeroTurnoverHabitat)
        }
    }
}
