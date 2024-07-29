use necsim_core::{
    cogs::{EmigrationExit, ImmigrationEntry, LocallyCoherentLineageStore, MathsCore, RngCore},
    reporter::Reporter,
};

use necsim_impls_no_std::cogs::{
    active_lineage_sampler::classical::ClassicalActiveLineageSampler,
    origin_sampler::TrustedOriginSampler,
};
use necsim_partitioning_core::LocalPartition;

use rustcoalescence_scenarios::Scenario;

use super::ClassicalLineageStoreSampleInitialiser;

#[expect(clippy::module_name_repetitions)]
pub struct GenesisInitialiser;

impl<M: MathsCore, G: RngCore<M>, O: Scenario<M, G>>
    ClassicalLineageStoreSampleInitialiser<M, G, O, !> for GenesisInitialiser
{
    type ActiveLineageSampler<
        S: LocallyCoherentLineageStore<M, O::Habitat>,
        X: EmigrationExit<M, O::Habitat, G, S>,
        I: ImmigrationEntry<M>,
    > = ClassicalActiveLineageSampler<
        M,
        O::Habitat,
        G,
        S,
        X,
        Self::DispersalSampler,
        O::SpeciationProbability,
        I,
    >;
    type DispersalSampler = O::DispersalSampler;

    fn init<
        'h,
        'p,
        T: TrustedOriginSampler<'h, M, Habitat = O::Habitat>,
        S: LocallyCoherentLineageStore<M, O::Habitat>,
        X: EmigrationExit<M, O::Habitat, G, S>,
        I: ImmigrationEntry<M>,
        Q: Reporter,
        P: LocalPartition<'p, Q>,
    >(
        self,
        origin_sampler: T,
        dispersal_sampler: O::DispersalSampler,
        _local_partition: &mut P,
    ) -> Result<
        (
            S,
            Self::DispersalSampler,
            Self::ActiveLineageSampler<S, X, I>,
        ),
        !,
    >
    where
        O::Habitat: 'h,
    {
        let (lineage_store, active_lineage_sampler) =
            ClassicalActiveLineageSampler::init_with_store(origin_sampler);

        Ok((lineage_store, dispersal_sampler, active_lineage_sampler))
    }
}
