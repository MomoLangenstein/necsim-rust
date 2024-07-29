use core::marker::PhantomData;

use alloc::vec::Vec;

use necsim_core::{
    cogs::{Habitat, MathsCore, RngCore},
    landscape::Location,
};
use necsim_core_bond::{ClosedUnitF64, NonNegativeF64};

use crate::{
    alias::AliasMethodSampler,
    array2d::{ArcArray2D, Array2D, VecArray2D},
    cogs::dispersal_sampler::in_memory::InMemoryDispersalSampler,
};

use super::{contract::check_in_memory_dispersal_contract, InMemoryDispersalSamplerError};

mod dispersal;

#[derive(Debug)]
pub struct InMemorySeparableAliasDispersalSampler<M: MathsCore, H: Habitat<M>, G: RngCore<M>> {
    alias_dispersal: ArcArray2D<Option<AliasMethodSampler<usize>>>,
    self_dispersal: ArcArray2D<ClosedUnitF64>,
    _marker: PhantomData<(M, H, G)>,
}

impl<M: MathsCore, H: Habitat<M>, G: RngCore<M>> InMemoryDispersalSampler<M, H, G>
    for InMemorySeparableAliasDispersalSampler<M, H, G>
{
    fn new(
        dispersal: &Array2D<NonNegativeF64>,
        habitat: &H,
    ) -> Result<Self, InMemoryDispersalSamplerError> {
        check_in_memory_dispersal_contract(dispersal, habitat)?;

        let habitat_extent = habitat.get_extent();

        let mut event_weights: Vec<(usize, NonNegativeF64)> =
            Vec::with_capacity(dispersal.row_len());

        let mut self_dispersal = VecArray2D::filled_with(
            ClosedUnitF64::zero(),
            usize::from(habitat_extent.height()),
            usize::from(habitat_extent.width()),
        );

        let alias_dispersal = Array2D::from_iter_row_major(
            dispersal.rows_iter().enumerate().map(|(row_index, row)| {
                event_weights.clear();

                let mut self_dispersal_at_location = NonNegativeF64::zero();

                for (col_index, dispersal_probability) in row.enumerate() {
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
                    let weight = *dispersal_probability
                        * NonNegativeF64::from(habitat.get_habitat_at_location(&location));

                    if weight > 0.0_f64 {
                        // Separate self-dispersal from out-dispersal
                        if col_index == row_index {
                            self_dispersal_at_location = weight;
                        } else {
                            event_weights.push((col_index, weight));
                        }
                    }
                }

                // total weight needs to extra include the self-dispersal,
                //  since it was excluded from event_weights earlier
                let total_weight = event_weights
                    .iter()
                    .map(|(_e, w)| *w)
                    .sum::<NonNegativeF64>()
                    + self_dispersal_at_location;

                if total_weight > 0.0_f64 {
                    // Safety: Normalisation limits the result to [0.0; 1.0]
                    let self_dispersal_probability = unsafe {
                        ClosedUnitF64::new_unchecked(
                            (self_dispersal_at_location / total_weight).get(),
                        )
                    };

                    self_dispersal[(
                        row_index / usize::from(habitat_extent.width()),
                        row_index % usize::from(habitat_extent.width()),
                    )] = self_dispersal_probability;
                }

                if event_weights.is_empty() {
                    None
                } else {
                    Some(AliasMethodSampler::new(&event_weights))
                }
            }),
            usize::from(habitat_extent.height()),
            usize::from(habitat_extent.width()),
        )
        .unwrap(); // infallible by PRE

        Ok(Self {
            alias_dispersal,
            self_dispersal: self_dispersal.switch_backend(),
            _marker: PhantomData::<(M, H, G)>,
        })
    }
}

impl<M: MathsCore, H: Habitat<M>, G: RngCore<M>> Clone
    for InMemorySeparableAliasDispersalSampler<M, H, G>
{
    fn clone(&self) -> Self {
        Self {
            alias_dispersal: self.alias_dispersal.clone(),
            self_dispersal: self.self_dispersal.clone(),
            _marker: PhantomData::<(M, H, G)>,
        }
    }
}
