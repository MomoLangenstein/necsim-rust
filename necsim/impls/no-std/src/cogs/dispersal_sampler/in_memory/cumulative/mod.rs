use core::marker::PhantomData;

use alloc::{boxed::Box, sync::Arc, vec};

use necsim_core::{
    cogs::{Habitat, MathsCore, RngCore},
    landscape::Location,
};
use necsim_core_bond::{ClosedUnitF64, NonNegativeF64};

use crate::{array2d::Array2D, cogs::dispersal_sampler::in_memory::InMemoryDispersalSampler};

use super::{contract::check_in_memory_dispersal_contract, InMemoryDispersalSamplerError};

mod contract;
mod dispersal;

#[derive(Debug)]
pub struct InMemoryCumulativeDispersalSampler<M: MathsCore, H: Habitat<M>, G: RngCore<M>> {
    cumulative_dispersal: Arc<[ClosedUnitF64]>,
    valid_dispersal_targets: Arc<[Option<usize>]>,
    marker: PhantomData<(M, H, G)>,
}

impl<M: MathsCore, H: Habitat<M>, G: RngCore<M>> InMemoryDispersalSampler<M, H, G>
    for InMemoryCumulativeDispersalSampler<M, H, G>
{
    #[debug_ensures(ret.as_ref().map_or(true, |ret| {
        ret.explicit_only_valid_targets_dispersal_contract(old(habitat))
    }), "valid_dispersal_targets only allows dispersal to habitat")]
    fn new(
        dispersal: &Array2D<NonNegativeF64>,
        habitat: &H,
    ) -> Result<Self, InMemoryDispersalSamplerError> {
        check_in_memory_dispersal_contract(dispersal, habitat)?;

        let habitat_extent = habitat.get_extent();

        let mut cumulative_dispersal =
            vec![NonNegativeF64::zero(); dispersal.num_elements()].into_boxed_slice();
        let mut valid_dispersal_targets = vec![None; dispersal.num_elements()].into_boxed_slice();

        for (row_index, row) in dispersal.rows_iter().enumerate() {
            let sum: NonNegativeF64 =
                row.enumerate()
                    .map(|(col_index, dispersal_probability)| {
                        #[expect(clippy::cast_possible_truncation)]
                        let location = Location::new(
                            habitat_extent.origin().x().wrapping_add(
                                (col_index % usize::from(habitat_extent.width())) as u32,
                            ),
                            habitat_extent.origin().y().wrapping_add(
                                (col_index / usize::from(habitat_extent.width())) as u32,
                            ),
                        );

                        // Multiply all dispersal probabilities by the habitat of their target
                        *dispersal_probability
                            * NonNegativeF64::from(habitat.get_habitat_at_location(&location))
                    })
                    .sum();

            if sum > 0.0_f64 {
                let mut acc = NonNegativeF64::zero();
                let mut last_valid_target: Option<usize> = None;

                for col_index in 0..dispersal.num_columns() {
                    #[expect(clippy::cast_possible_truncation)]
                    let location =
                        Location::new(
                            habitat_extent.origin().x().wrapping_add(
                                (col_index % usize::from(habitat_extent.width())) as u32,
                            ),
                            habitat_extent.origin().y().wrapping_add(
                                (col_index / usize::from(habitat_extent.width())) as u32,
                            ),
                        );

                    // Multiply all dispersal probabilities by the habitat of their target
                    let dispersal_probability = dispersal[(row_index, col_index)]
                        * NonNegativeF64::from(habitat.get_habitat_at_location(&location));

                    if dispersal_probability > 0.0_f64 {
                        acc += dispersal_probability;

                        last_valid_target = Some(col_index);
                    }

                    cumulative_dispersal[row_index * dispersal.row_len() + col_index] = acc / sum;

                    // Store the index of the last valid dispersal target
                    valid_dispersal_targets[row_index * dispersal.row_len() + col_index] =
                        last_valid_target;
                }
            }
        }

        // Safety: The dispersal weights are now probabilities in [0.0; 1.0]
        let cumulative_dispersal = unsafe {
            core::mem::transmute::<Box<[NonNegativeF64]>, Box<[ClosedUnitF64]>>(
                cumulative_dispersal,
            )
        };

        Ok(Self {
            cumulative_dispersal: Arc::from(cumulative_dispersal),
            valid_dispersal_targets: Arc::from(valid_dispersal_targets),
            marker: PhantomData::<(M, H, G)>,
        })
    }
}

impl<M: MathsCore, H: Habitat<M>, G: RngCore<M>> Clone
    for InMemoryCumulativeDispersalSampler<M, H, G>
{
    fn clone(&self) -> Self {
        Self {
            cumulative_dispersal: self.cumulative_dispersal.clone(),
            valid_dispersal_targets: self.valid_dispersal_targets.clone(),
            marker: PhantomData::<(M, H, G)>,
        }
    }
}
