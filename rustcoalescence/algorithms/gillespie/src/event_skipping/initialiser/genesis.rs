use necsim_core::{
    cogs::{
        EmigrationExit, GloballyCoherentLineageStore, ImmigrationEntry, MathsCore, RngCore,
        SeparableDispersalSampler,
    },
    reporter::Reporter,
};

use necsim_impls_no_std::cogs::{
    active_lineage_sampler::alias::location::LocationAliasActiveLineageSampler,
    coalescence_sampler::conditional::ConditionalCoalescenceSampler,
    event_sampler::gillespie::conditional::ConditionalGillespieEventSampler,
    origin_sampler::TrustedOriginSampler,
};
use necsim_partitioning_core::LocalPartition;

use rustcoalescence_scenarios::Scenario;

use super::EventSkippingLineageStoreSampleInitialiser;

#[expect(clippy::module_name_repetitions)]
pub struct GenesisInitialiser;

impl<M: MathsCore, G: RngCore<M>, O: Scenario<M, G>>
    EventSkippingLineageStoreSampleInitialiser<M, G, O, !> for GenesisInitialiser
where
    O::DispersalSampler: SeparableDispersalSampler<M, O::Habitat, G>,
{
    type ActiveLineageSampler<
        S: GloballyCoherentLineageStore<M, O::Habitat>,
        X: EmigrationExit<M, O::Habitat, G, S>,
        I: ImmigrationEntry<M>,
    > = LocationAliasActiveLineageSampler<
        M,
        O::Habitat,
        G,
        S,
        X,
        Self::DispersalSampler,
        ConditionalCoalescenceSampler<M, O::Habitat, S>,
        O::TurnoverRate,
        O::SpeciationProbability,
        ConditionalGillespieEventSampler<
            M,
            O::Habitat,
            G,
            S,
            X,
            Self::DispersalSampler,
            O::TurnoverRate,
            O::SpeciationProbability,
        >,
        I,
    >;
    type DispersalSampler = O::DispersalSampler;

    fn init<
        'h,
        'p,
        T: TrustedOriginSampler<'h, M, Habitat = O::Habitat>,
        S: GloballyCoherentLineageStore<M, O::Habitat>,
        X: EmigrationExit<M, O::Habitat, G, S>,
        I: ImmigrationEntry<M>,
        Q: Reporter,
        P: LocalPartition<'p, Q>,
    >(
        self,
        origin_sampler: T,
        dispersal_sampler: O::DispersalSampler,
        coalescence_sampler: &ConditionalCoalescenceSampler<M, O::Habitat, S>,
        turnover_rate: &O::TurnoverRate,
        speciation_probability: &O::SpeciationProbability,
        _local_partition: &mut P,
    ) -> Result<
        (
            S,
            Self::DispersalSampler,
            ConditionalGillespieEventSampler<
                M,
                O::Habitat,
                G,
                S,
                X,
                Self::DispersalSampler,
                O::TurnoverRate,
                O::SpeciationProbability,
            >,
            Self::ActiveLineageSampler<S, X, I>,
        ),
        !,
    >
    where
        O::Habitat: 'h,
    {
        let event_sampler = ConditionalGillespieEventSampler::default();

        let (lineage_store, active_lineage_sampler) =
            LocationAliasActiveLineageSampler::init_with_store(
                origin_sampler,
                &dispersal_sampler,
                coalescence_sampler,
                turnover_rate,
                speciation_probability,
                &event_sampler,
            );

        Ok((
            lineage_store,
            dispersal_sampler,
            event_sampler,
            active_lineage_sampler,
        ))
    }
}
