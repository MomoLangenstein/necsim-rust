use necsim_core::{
    cogs::{DispersalSampler, Habitat, MathsCore, RngCore, SeparableDispersalSampler},
    landscape::Location,
};
use necsim_core_bond::ClosedUnitF64;

use super::InMemorySeparableAliasDispersalSampler;

#[contract_trait]
impl<M: MathsCore, H: Habitat<M>, G: RngCore<M>> DispersalSampler<M, H, G>
    for InMemorySeparableAliasDispersalSampler<M, H, G>
{
    #[must_use]
    fn sample_dispersal_from_location(
        &self,
        location: &Location,
        habitat: &H,
        rng: &mut G,
    ) -> Location {
        use necsim_core::cogs::RngSampler;

        let self_dispersal_at_location =
            self.get_self_dispersal_probability_at_location(location, habitat);

        if self_dispersal_at_location >= 1.0_f64 {
            return location.clone();
        }

        if self_dispersal_at_location > 0.0_f64 && rng.sample_event(self_dispersal_at_location) {
            return location.clone();
        }

        self.sample_non_self_dispersal_from_location(location, habitat, rng)
    }
}

#[contract_trait]
impl<M: MathsCore, H: Habitat<M>, G: RngCore<M>> SeparableDispersalSampler<M, H, G>
    for InMemorySeparableAliasDispersalSampler<M, H, G>
{
    #[must_use]
    fn sample_non_self_dispersal_from_location(
        &self,
        location: &Location,
        habitat: &H,
        rng: &mut G,
    ) -> Location {
        let alias_dispersal_at_location = self.alias_dispersal[(
            location.y().wrapping_sub(habitat.get_extent().origin().y()) as usize,
            location.x().wrapping_sub(habitat.get_extent().origin().x()) as usize,
        )]
            .as_ref()
            .expect("habitat dispersal origin must disperse somewhere");

        let dispersal_target_index = alias_dispersal_at_location.sample_event(rng);

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
        self.self_dispersal[(
            location.y().wrapping_sub(habitat.get_extent().origin().y()) as usize,
            location.x().wrapping_sub(habitat.get_extent().origin().x()) as usize,
        )]
    }
}
