use necsim_core::{
    cogs::{EmigrationExit, MathsCore, PrimeableRng},
    lineage::Lineage,
};

use necsim_impls_no_std::cogs::{
    active_lineage_sampler::independent::{
        event_time_sampler::EventTimeSampler, IndependentActiveLineageSampler,
    },
    lineage_store::independent::IndependentLineageStore,
    origin_sampler::TrustedOriginSampler,
};

use rustcoalescence_scenarios::Scenario;

use super::IndependentLineageStoreSampleInitialiser;

#[expect(clippy::module_name_repetitions)]
pub struct GenesisInitialiser;

impl<M: MathsCore, G: PrimeableRng<M>, O: Scenario<M, G>>
    IndependentLineageStoreSampleInitialiser<M, G, O, !> for GenesisInitialiser
{
    type ActiveLineageSampler<
        X: EmigrationExit<M, O::Habitat, G, IndependentLineageStore<M, O::Habitat>>,
        J: EventTimeSampler<M, O::Habitat, G, O::TurnoverRate>,
    > = IndependentActiveLineageSampler<
        M,
        O::Habitat,
        G,
        X,
        Self::DispersalSampler,
        O::TurnoverRate,
        O::SpeciationProbability,
        J,
    >;
    type DispersalSampler = O::DispersalSampler;

    fn init<
        'h,
        T: TrustedOriginSampler<'h, M, Habitat = O::Habitat>,
        J: EventTimeSampler<M, O::Habitat, G, O::TurnoverRate>,
        X: EmigrationExit<M, O::Habitat, G, IndependentLineageStore<M, O::Habitat>>,
    >(
        self,
        origin_sampler: T,
        dispersal_sampler: O::DispersalSampler,
        event_time_sampler: J,
    ) -> Result<
        (
            IndependentLineageStore<M, O::Habitat>,
            Self::DispersalSampler,
            Self::ActiveLineageSampler<X, J>,
            Vec<Lineage>,
            Vec<Lineage>,
        ),
        !,
    >
    where
        O::Habitat: 'h,
    {
        let (lineage_store, active_lineage_sampler, lineages) =
            IndependentActiveLineageSampler::init_with_store_and_lineages(
                origin_sampler,
                event_time_sampler,
            );

        Ok((
            lineage_store,
            dispersal_sampler,
            active_lineage_sampler,
            lineages,
            Vec::new(),
        ))
    }
}
