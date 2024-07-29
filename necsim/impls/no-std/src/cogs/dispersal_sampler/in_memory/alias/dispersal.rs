use necsim_core::{
    cogs::{DispersalSampler, Habitat, MathsCore, RngCore},
    landscape::Location,
};

use super::InMemoryAliasDispersalSampler;

#[contract_trait]
impl<M: MathsCore, H: Habitat<M>, G: RngCore<M>> DispersalSampler<M, H, G>
    for InMemoryAliasDispersalSampler<M, H, G>
{
    #[must_use]
    fn sample_dispersal_from_location(
        &self,
        location: &Location,
        habitat: &H,
        rng: &mut G,
    ) -> Location {
        let alias_dispersal_at_location = self.alias_dispersal[(
            (location.y() - habitat.get_extent().origin().y()) as usize,
            (location.x() - habitat.get_extent().origin().x()) as usize,
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
}
