use core::ops::Range;

use necsim_core::{
    cogs::{DispersalSampler, Habitat, MathsCore, RngCore},
    landscape::Location,
};

use crate::alias::packed::AliasMethodSamplerAtom;

use super::InMemoryPackedAliasDispersalSampler;

#[contract_trait]
impl<M: MathsCore, H: Habitat<M>, G: RngCore<M>> DispersalSampler<M, H, G>
    for InMemoryPackedAliasDispersalSampler<M, H, G>
{
    #[must_use]
    fn sample_dispersal_from_location(
        &self,
        location: &Location,
        habitat: &H,
        rng: &mut G,
    ) -> Location {
        let location_row = location.y().wrapping_sub(habitat.get_extent().origin().y()) as usize;
        let location_column = location.x().wrapping_sub(habitat.get_extent().origin().x()) as usize;

        // Only safe as trait precondition that `location` is inside `habitat`
        let alias_range = unsafe {
            Range::<usize>::from(
                self.alias_dispersal_ranges
                    .get(location_row, location_column)
                    .unwrap_unchecked()
                    .clone(),
            )
        };

        // Safe by the construction of `InMemoryPackedAliasDispersalSampler`
        let alias_dispersals_at_location =
            unsafe { &self.alias_dispersal_buffer.get_unchecked(alias_range) };

        let dispersal_target_index: usize =
            AliasMethodSamplerAtom::sample_event(alias_dispersals_at_location, rng);

        #[expect(clippy::cast_possible_truncation)]
        Location::new(
            habitat.get_extent().origin().x().wrapping_add(
                (dispersal_target_index % usize::from(habitat.get_extent().width())) as u32,
            ),
            habitat.get_extent().origin().y().wrapping_add(
                (dispersal_target_index / usize::from(habitat.get_extent().width())) as u32,
            ),
        )
    }
}
