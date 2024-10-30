use necsim_core::{
    cogs::{DispersalSampler, Habitat, MathsCore, RngCore, SeparableDispersalSampler},
    landscape::Location,
};
use necsim_core_bond::ClosedUnitF64;

use crate::alias::packed::AliasMethodSamplerAtom;

use super::InMemoryPackedSeparableAliasDispersalSampler;

#[contract_trait]
impl<M: MathsCore, H: Habitat<M>, G: RngCore<M>> DispersalSampler<M, H, G>
    for InMemoryPackedSeparableAliasDispersalSampler<M, H, G>
{
    #[must_use]
    fn sample_dispersal_from_location(
        &self,
        location: &Location,
        habitat: &H,
        rng: &mut G,
    ) -> Location {
        use necsim_core::cogs::RngSampler;

        let location_row = location.y().wrapping_sub(habitat.get_extent().origin().y()) as usize;
        let location_column = location.x().wrapping_sub(habitat.get_extent().origin().x()) as usize;
        let self_dispersal_index =
            location_row * usize::from(habitat.get_extent().width()) + location_column;

        // Only safe as trait precondition that `location` is inside `habitat`
        let alias_range = unsafe {
            self.alias_dispersal_ranges
                .get(location_row, location_column)
                .unwrap_unchecked()
        };
        let self_dispersal = unsafe {
            self.self_dispersal
                .get(location_row, location_column)
                .unwrap_unchecked()
        };

        let dispersal_target_index: usize = if self_dispersal.self_dispersal == ClosedUnitF64::one()
        {
            // guaranteed self-dispersal
            self_dispersal_index
        } else if (
            // guaranteed non-self-dispersal
            self_dispersal.self_dispersal == ClosedUnitF64::zero()
        ) || (
            // self-dispersal with an underfull atom, so included
            self_dispersal.non_self_dispersal_event != self_dispersal_index
        ) || (
            // excluded self-dispersal, but we sampled non-self-dispersal
            rng.sample_uniform_closed_open() >= self_dispersal.self_dispersal
        ) {
            // Safe by the construction of `InMemoryPackedSeparableAliasDispersalSampler`
            let alias_dispersals_at_location = unsafe {
                &self
                    .alias_dispersal_buffer
                    .get_unchecked(alias_range.start..alias_range.end)
            };

            AliasMethodSamplerAtom::sample_event(alias_dispersals_at_location, rng)
        } else {
            // excluded self-dispersal, and we sampled it
            self_dispersal_index
        };

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

#[contract_trait]
impl<M: MathsCore, H: Habitat<M>, G: RngCore<M>> SeparableDispersalSampler<M, H, G>
    for InMemoryPackedSeparableAliasDispersalSampler<M, H, G>
{
    #[must_use]
    fn sample_non_self_dispersal_from_location(
        &self,
        location: &Location,
        habitat: &H,
        rng: &mut G,
    ) -> Location {
        let location_row = location.y().wrapping_sub(habitat.get_extent().origin().y()) as usize;
        let location_column = location.x().wrapping_sub(habitat.get_extent().origin().x()) as usize;
        let self_dispersal_index =
            location_row * usize::from(habitat.get_extent().width()) + location_column;

        // Only safe as trait precondition that `location` is inside `habitat`
        let alias_range = unsafe {
            self.alias_dispersal_ranges
                .get(location_row, location_column)
                .unwrap_unchecked()
        };
        let self_dispersal = unsafe {
            self.self_dispersal
                .get(location_row, location_column)
                .unwrap_unchecked()
        };

        // Safe by the construction of `InMemoryPackedSeparableAliasDispersalSampler`
        let alias_dispersals_at_location = unsafe {
            &self
                .alias_dispersal_buffer
                .get_unchecked(alias_range.start..alias_range.end)
        };

        // Since the atoms are pre-sorted s.t. all self-dispersal is on the right,
        //  we can exclude self-dispersal by providing 1-sd as the CDF limit
        let mut dispersal_target_index: usize = AliasMethodSamplerAtom::sample_event_with_cdf_limit(
            alias_dispersals_at_location,
            rng,
            // if non_self_dispersal_event is None, self-dispersal is already
            //  excluded from the alias sampler and so we can sample the full
            //  CDF
            if self_dispersal.non_self_dispersal_event == self_dispersal_index {
                ClosedUnitF64::one()
            } else {
                self_dispersal.self_dispersal.one_minus()
            },
        );

        // if rounding errors caused self-dispersal, replace with the non-self-dispersal
        //  event
        if dispersal_target_index == self_dispersal_index {
            dispersal_target_index = self_dispersal.non_self_dispersal_event;
        }

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

    #[must_use]
    #[debug_requires(habitat.get_extent().contains(location), "location is inside habitat extent")]
    fn get_self_dispersal_probability_at_location(
        &self,
        location: &Location,
        habitat: &H,
    ) -> ClosedUnitF64 {
        // Only safe as trait precondition that `location` is inside `habitat`
        unsafe {
            self.self_dispersal
                .get(
                    location.y().wrapping_sub(habitat.get_extent().origin().y()) as usize,
                    location.x().wrapping_sub(habitat.get_extent().origin().x()) as usize,
                )
                .unwrap_unchecked()
        }
        .self_dispersal
    }
}
