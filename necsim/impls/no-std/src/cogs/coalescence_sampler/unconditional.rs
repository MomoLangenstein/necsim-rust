use core::marker::PhantomData;

use necsim_core::{
    cogs::{
        coalescence_sampler::CoalescenceRngSample, Backup, CoalescenceSampler, Habitat,
        LocallyCoherentLineageStore, MathsCore,
    },
    landscape::{IndexedLocation, Location},
    lineage::LineageInteraction,
};

use super::optional_coalescence;

#[expect(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct UnconditionalCoalescenceSampler<
    M: MathsCore,
    H: Habitat<M>,
    S: LocallyCoherentLineageStore<M, H>,
>(PhantomData<(M, H, S)>);

impl<M: MathsCore, H: Habitat<M>, S: LocallyCoherentLineageStore<M, H>> Default
    for UnconditionalCoalescenceSampler<M, H, S>
{
    fn default() -> Self {
        Self(PhantomData::<(M, H, S)>)
    }
}

#[contract_trait]
impl<M: MathsCore, H: Habitat<M>, S: LocallyCoherentLineageStore<M, H>> Backup
    for UnconditionalCoalescenceSampler<M, H, S>
{
    unsafe fn backup_unchecked(&self) -> Self {
        Self(PhantomData::<(M, H, S)>)
    }
}

#[contract_trait]
impl<M: MathsCore, H: Habitat<M>, S: LocallyCoherentLineageStore<M, H>> CoalescenceSampler<M, H, S>
    for UnconditionalCoalescenceSampler<M, H, S>
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
