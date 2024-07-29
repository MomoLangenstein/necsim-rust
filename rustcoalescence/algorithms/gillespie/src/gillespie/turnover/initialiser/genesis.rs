use necsim_core::{
    cogs::{
        CoalescenceSampler, EmigrationExit, EventSampler, ImmigrationEntry,
        LocallyCoherentLineageStore, MathsCore, RngCore,
    },
    reporter::Reporter,
};

use necsim_impls_no_std::cogs::{
    active_lineage_sampler::alias::individual::IndividualAliasActiveLineageSampler,
    origin_sampler::TrustedOriginSampler,
};
use necsim_partitioning_core::LocalPartition;

use rustcoalescence_scenarios::Scenario;

use super::GillespieLineageStoreSampleInitialiser;

#[expect(clippy::module_name_repetitions)]
pub struct GenesisInitialiser;

impl<M: MathsCore, G: RngCore<M>, O: Scenario<M, G>>
    GillespieLineageStoreSampleInitialiser<M, G, O, !> for GenesisInitialiser
{
    type ActiveLineageSampler<
        S: LocallyCoherentLineageStore<M, O::Habitat>,
        X: EmigrationExit<M, O::Habitat, G, S>,
        C: CoalescenceSampler<M, O::Habitat, S>,
        E: EventSampler<
            M,
            O::Habitat,
            G,
            S,
            X,
            Self::DispersalSampler,
            C,
            O::TurnoverRate,
            O::SpeciationProbability,
        >,
        I: ImmigrationEntry<M>,
    > = IndividualAliasActiveLineageSampler<
        M,
        O::Habitat,
        G,
        S,
        X,
        Self::DispersalSampler,
        C,
        O::TurnoverRate,
        O::SpeciationProbability,
        E,
        I,
    >;
    type DispersalSampler = O::DispersalSampler;

    fn init<
        'h,
        'p,
        T: TrustedOriginSampler<'h, M, Habitat = O::Habitat>,
        S: LocallyCoherentLineageStore<M, O::Habitat>,
        X: EmigrationExit<M, O::Habitat, G, S>,
        C: CoalescenceSampler<M, O::Habitat, S>,
        E: EventSampler<
            M,
            O::Habitat,
            G,
            S,
            X,
            Self::DispersalSampler,
            C,
            O::TurnoverRate,
            O::SpeciationProbability,
        >,
        I: ImmigrationEntry<M>,
        Q: Reporter,
        P: LocalPartition<'p, Q>,
    >(
        self,
        origin_sampler: T,
        dispersal_sampler: O::DispersalSampler,
        turnover_rate: &O::TurnoverRate,
        _local_partition: &mut P,
    ) -> Result<
        (
            S,
            Self::DispersalSampler,
            Self::ActiveLineageSampler<S, X, C, E, I>,
        ),
        !,
    >
    where
        O::Habitat: 'h,
    {
        let (lineage_store, active_lineage_sampler) =
            IndividualAliasActiveLineageSampler::init_with_store(origin_sampler, turnover_rate);

        Ok((lineage_store, dispersal_sampler, active_lineage_sampler))
    }
}
