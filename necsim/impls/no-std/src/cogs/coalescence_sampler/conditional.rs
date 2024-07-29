use core::marker::PhantomData;

use necsim_core::{
    cogs::{
        coalescence_sampler::CoalescenceRngSample, Backup, CoalescenceSampler,
        GloballyCoherentLineageStore, Habitat, MathsCore,
    },
    landscape::{IndexedLocation, Location},
    lineage::{GlobalLineageReference, LineageInteraction},
};
use necsim_core_bond::ClosedUnitF64;

use super::optional_coalescence;

#[expect(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct ConditionalCoalescenceSampler<
    M: MathsCore,
    H: Habitat<M>,
    S: GloballyCoherentLineageStore<M, H>,
>(PhantomData<(M, H, S)>);

impl<M: MathsCore, H: Habitat<M>, S: GloballyCoherentLineageStore<M, H>> Default
    for ConditionalCoalescenceSampler<M, H, S>
{
    fn default() -> Self {
        Self(PhantomData::<(M, H, S)>)
    }
}

#[contract_trait]
impl<M: MathsCore, H: Habitat<M>, S: GloballyCoherentLineageStore<M, H>> Backup
    for ConditionalCoalescenceSampler<M, H, S>
{
    unsafe fn backup_unchecked(&self) -> Self {
        Self(PhantomData::<(M, H, S)>)
    }
}

#[contract_trait]
impl<M: MathsCore, H: Habitat<M>, S: GloballyCoherentLineageStore<M, H>> CoalescenceSampler<M, H, S>
    for ConditionalCoalescenceSampler<M, H, S>
{
    #[must_use]
    fn sample_interaction_at_location(
        &self,
        location: Location,
        habitat: &H,
        lineage_store: &S,
        coalescence_rng_sample: CoalescenceRngSample,
    ) -> (IndexedLocation, LineageInteraction) {
        optional_coalescence::sample_interaction_at_location(
            location,
            habitat,
            lineage_store,
            coalescence_rng_sample,
        )
    }
}

impl<M: MathsCore, H: Habitat<M>, S: GloballyCoherentLineageStore<M, H>>
    ConditionalCoalescenceSampler<M, H, S>
{
    #[must_use]
    pub fn sample_coalescence_at_location(
        &self,
        location: Location,
        habitat: &H,
        lineage_store: &S,
        coalescence_rng_sample: CoalescenceRngSample,
    ) -> (IndexedLocation, GlobalLineageReference) {
        let lineages_at_location =
            lineage_store.get_local_lineage_references_at_location_unordered(&location, habitat);

        #[expect(clippy::cast_possible_truncation)]
        let population = lineages_at_location.len() as u32;

        let chosen_coalescence_index =
            coalescence_rng_sample.sample_coalescence_index::<M>(population);
        let chosen_coalescence = &lineages_at_location[chosen_coalescence_index as usize];

        let lineage = &lineage_store[chosen_coalescence];

        let indexed_location = IndexedLocation::new(location, lineage.indexed_location.index());

        (indexed_location, lineage.global_reference.clone())
    }

    #[must_use]
    #[debug_requires(habitat.get_habitat_at_location(location) > 0, "location is habitable")]
    pub fn get_coalescence_probability_at_location(
        &self,
        location: &Location,
        habitat: &H,
        lineage_store: &S,
        lineage_store_includes_self: bool,
    ) -> ClosedUnitF64 {
        // If the lineage store includes self, the population must be decremented
        //  to avoid coalescence with the currently active lineage

        #[expect(clippy::cast_precision_loss)]
        let population = (lineage_store
            .get_local_lineage_references_at_location_unordered(location, habitat)
            .len()
            - usize::from(lineage_store_includes_self)) as f64;
        let habitat = f64::from(habitat.get_habitat_at_location(location));

        // Safety: Normalised probability in range [0.0; 1.0]
        unsafe { ClosedUnitF64::new_unchecked(population / habitat) }
    }
}
