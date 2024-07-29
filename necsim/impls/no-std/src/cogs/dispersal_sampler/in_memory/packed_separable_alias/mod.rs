use alloc::{sync::Arc, vec::Vec};
use core::marker::PhantomData;
use necsim_core_bond::{ClosedUnitF64, NonNegativeF64};

use necsim_core::{
    cogs::{Habitat, MathsCore, RngCore},
    landscape::Location,
};

use crate::{
    alias::packed::AliasMethodSamplerAtom,
    array2d::{ArcArray2D, Array2D, VecArray2D},
};

mod dispersal;

use super::{
    contract::check_in_memory_dispersal_contract, InMemoryDispersalSampler,
    InMemoryDispersalSamplerError,
};

#[derive(Clone, Debug, TypeLayout)]
#[doc(hidden)]
#[repr(C)]
pub struct AliasSamplerRange {
    start: usize,
    end: usize,
}

#[derive(Clone, Debug, TypeLayout)]
#[doc(hidden)]
#[repr(C)]
pub struct SeparableAliasSelfDispersal {
    // self-dispersal
    // 1-factor to multiply U(0,1)*|range| with to exclude self-dispersal
    self_dispersal: ClosedUnitF64,
    // non-self-dispersal event to sample in case rounding errors cause
    //  self-dispersal to be sampled in no-self-dispersal mode
    // if it is set to self-dispersal, then it marks self-dispersal not
    //  being part of the alias sampler
    non_self_dispersal_event: usize,
}

#[cfg_attr(feature = "cuda", derive(rust_cuda::lend::LendRustToCuda))]
#[cfg_attr(feature = "cuda", cuda(free = "M", free = "H", free = "G"))]
pub struct InMemoryPackedSeparableAliasDispersalSampler<M: MathsCore, H: Habitat<M>, G: RngCore<M>>
{
    #[cfg_attr(feature = "cuda", cuda(embed))]
    alias_dispersal_ranges: ArcArray2D<AliasSamplerRange>,
    #[cfg_attr(feature = "cuda", cuda(embed))]
    self_dispersal: ArcArray2D<SeparableAliasSelfDispersal>,
    #[cfg_attr(feature = "cuda", cuda(embed))]
    alias_dispersal_buffer: Arc<[AliasMethodSamplerAtom<usize>]>,
    marker: PhantomData<(M, H, G)>,
}

impl<M: MathsCore, H: Habitat<M>, G: RngCore<M>> InMemoryDispersalSampler<M, H, G>
    for InMemoryPackedSeparableAliasDispersalSampler<M, H, G>
{
    #[expect(clippy::too_many_lines)]
    fn new(
        dispersal: &Array2D<NonNegativeF64>,
        habitat: &H,
    ) -> Result<Self, InMemoryDispersalSamplerError> {
        check_in_memory_dispersal_contract(dispersal, habitat)?;

        let habitat_extent = habitat.get_extent();

        let mut event_weights: Vec<(usize, NonNegativeF64)> =
            Vec::with_capacity(dispersal.row_len());

        let mut alias_dispersal_buffer = Vec::new();

        let mut self_dispersal = VecArray2D::from_iter_row_major(
            (0..).map(|self_dispersal_event| SeparableAliasSelfDispersal {
                self_dispersal: ClosedUnitF64::zero(),
                // no self-dispersal
                non_self_dispersal_event: self_dispersal_event,
            }),
            usize::from(habitat_extent.height()),
            usize::from(habitat_extent.width()),
        )
        .unwrap();

        let alias_dispersal_ranges = Array2D::from_iter_row_major(
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
                        event_weights.push((col_index, weight));

                        // Separate self-dispersal from out-dispersal
                        if col_index == row_index {
                            self_dispersal_at_location = weight;
                        }
                    }
                }

                let range_from = alias_dispersal_buffer.len();

                if event_weights.is_empty() {
                    AliasSamplerRange {
                        start: range_from,
                        end: range_from,
                    }
                } else {
                    // total weight already contains self-dispersal
                    let total_weight = event_weights
                        .iter()
                        .map(|(_e, w)| *w)
                        .sum::<NonNegativeF64>();
                    // Safety: Normalisation limits the result to [0.0; 1.0]
                    let self_dispersal_probability = unsafe {
                        ClosedUnitF64::new_unchecked(
                            (self_dispersal_at_location / total_weight).get(),
                        )
                    };

                    let self_dispersal_u = self_dispersal_at_location
                        * NonNegativeF64::from(event_weights.len())
                        / total_weight;

                    let mut atoms;
                    let mut non_self_dispersal_event = row_index;

                    if self_dispersal_u < 1.0 {
                        atoms = AliasMethodSamplerAtom::create(&event_weights);

                        assert_eq!(atoms.len(), event_weights.len());

                        // if self-dispersal exists, since the self-dispersal u is underfull,
                        //  the alias method table construction guarantees that there will be
                        //  exactly one atom which contains self-dispersal - let's find it
                        if let Some((self_dispersal_index, self_dispersal_atom)) = atoms
                            .iter_mut()
                            .enumerate()
                            .find(|(_, atom)| (atom.e() == row_index) || (atom.k() == row_index))
                        {
                            // ensure that self-dispersal is in the last atom on the right
                            //  and in its right slot
                            if self_dispersal_atom.e() == row_index {
                                self_dispersal_atom.flip();
                            }
                            non_self_dispersal_event = self_dispersal_atom.e();
                            let last_atom_index = atoms.len() - 1;
                            atoms.swap(self_dispersal_index, last_atom_index);
                        };
                    } else {
                        // remove self-dispersal from the alias sampler as it is cheaper to handle
                        //  it separately
                        event_weights.retain(|(event, _)| *event != row_index);
                        atoms = AliasMethodSamplerAtom::create(&event_weights);
                    };

                    self_dispersal[(
                        row_index / usize::from(habitat_extent.width()),
                        row_index % usize::from(habitat_extent.width()),
                    )] = SeparableAliasSelfDispersal {
                        self_dispersal: self_dispersal_probability,
                        non_self_dispersal_event,
                    };

                    alias_dispersal_buffer.append(&mut atoms);

                    AliasSamplerRange {
                        start: range_from,
                        end: alias_dispersal_buffer.len(),
                    }
                }
            }),
            usize::from(habitat_extent.height()),
            usize::from(habitat_extent.width()),
        )
        .unwrap(); // infallible by PRE;

        Ok(Self {
            alias_dispersal_ranges,
            self_dispersal: self_dispersal.switch_backend(),
            alias_dispersal_buffer: Arc::from(alias_dispersal_buffer.into_boxed_slice()),
            marker: PhantomData::<(M, H, G)>,
        })
    }
}

impl<M: MathsCore, H: Habitat<M>, G: RngCore<M>> core::fmt::Debug
    for InMemoryPackedSeparableAliasDispersalSampler<M, H, G>
{
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct(stringify!(InMemoryPackedSeparableAliasDispersalSampler))
            .finish_non_exhaustive()
    }
}

impl<M: MathsCore, H: Habitat<M>, G: RngCore<M>> Clone
    for InMemoryPackedSeparableAliasDispersalSampler<M, H, G>
{
    fn clone(&self) -> Self {
        Self {
            alias_dispersal_ranges: self.alias_dispersal_ranges.clone(),
            self_dispersal: self.self_dispersal.clone(),
            alias_dispersal_buffer: self.alias_dispersal_buffer.clone(),
            marker: PhantomData::<(M, H, G)>,
        }
    }
}
