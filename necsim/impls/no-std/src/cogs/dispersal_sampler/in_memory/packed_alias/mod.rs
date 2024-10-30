use alloc::{sync::Arc, vec::Vec};
use core::{marker::PhantomData, ops::Range};
use necsim_core_bond::NonNegativeF64;

use necsim_core::{
    cogs::{Habitat, MathsCore, RngCore},
    landscape::Location,
};

use crate::{
    alias::packed::AliasMethodSamplerAtom,
    array2d::{ArcArray2D, Array2D},
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

impl From<Range<usize>> for AliasSamplerRange {
    fn from(range: Range<usize>) -> Self {
        Self {
            start: range.start,
            end: range.end,
        }
    }
}

impl From<AliasSamplerRange> for Range<usize> {
    fn from(range: AliasSamplerRange) -> Self {
        range.start..range.end
    }
}

#[cfg_attr(feature = "cuda", derive(rust_cuda::lend::LendRustToCuda))]
#[cfg_attr(feature = "cuda", cuda(free = "M", free = "H", free = "G"))]
pub struct InMemoryPackedAliasDispersalSampler<M: MathsCore, H: Habitat<M>, G: RngCore<M>> {
    #[cfg_attr(feature = "cuda", cuda(embed))]
    alias_dispersal_ranges: ArcArray2D<AliasSamplerRange>,
    #[cfg_attr(feature = "cuda", cuda(embed))]
    alias_dispersal_buffer: Arc<[AliasMethodSamplerAtom<usize>]>,
    marker: PhantomData<(M, H, G)>,
}

impl<M: MathsCore, H: Habitat<M>, G: RngCore<M>> InMemoryDispersalSampler<M, H, G>
    for InMemoryPackedAliasDispersalSampler<M, H, G>
{
    fn new(
        dispersal: &Array2D<NonNegativeF64>,
        habitat: &H,
    ) -> Result<Self, InMemoryDispersalSamplerError> {
        check_in_memory_dispersal_contract(dispersal, habitat)?;

        let habitat_extent = habitat.get_extent();

        let mut event_weights: Vec<(usize, NonNegativeF64)> =
            Vec::with_capacity(dispersal.row_len());

        let mut alias_dispersal_buffer = Vec::new();

        let alias_dispersal_ranges = Array2D::from_iter_row_major(
            dispersal.rows_iter().map(|row| {
                event_weights.clear();

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
                    }
                }

                let range_from = alias_dispersal_buffer.len();

                if event_weights.is_empty() {
                    AliasSamplerRange::from(range_from..range_from)
                } else {
                    alias_dispersal_buffer
                        .append(&mut AliasMethodSamplerAtom::create(&event_weights));

                    AliasSamplerRange::from(range_from..alias_dispersal_buffer.len())
                }
            }),
            usize::from(habitat_extent.height()),
            usize::from(habitat_extent.width()),
        )
        .unwrap(); // infallible by PRE;

        Ok(Self {
            alias_dispersal_ranges,
            alias_dispersal_buffer: Arc::from(alias_dispersal_buffer.into_boxed_slice()),
            marker: PhantomData::<(M, H, G)>,
        })
    }
}

impl<M: MathsCore, H: Habitat<M>, G: RngCore<M>> core::fmt::Debug
    for InMemoryPackedAliasDispersalSampler<M, H, G>
{
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct(stringify!(InMemoryPackedAliasDispersalSampler))
            .field("alias_dispersal_ranges", &self.alias_dispersal_ranges)
            .field(
                "alias_dispersal_buffer",
                &format_args!(
                    "Box [ {:p}; {} ]",
                    &self.alias_dispersal_buffer,
                    self.alias_dispersal_buffer.len()
                ),
            )
            .finish()
    }
}

impl<M: MathsCore, H: Habitat<M>, G: RngCore<M>> Clone
    for InMemoryPackedAliasDispersalSampler<M, H, G>
{
    fn clone(&self) -> Self {
        Self {
            alias_dispersal_ranges: self.alias_dispersal_ranges.clone(),
            alias_dispersal_buffer: self.alias_dispersal_buffer.clone(),
            marker: PhantomData::<(M, H, G)>,
        }
    }
}
