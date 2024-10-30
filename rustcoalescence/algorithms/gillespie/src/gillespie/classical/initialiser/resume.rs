use necsim_core::{
    cogs::{EmigrationExit, ImmigrationEntry, LocallyCoherentLineageStore, MathsCore, RngCore},
    lineage::Lineage,
    reporter::Reporter,
};
use necsim_core_bond::NonNegativeF64;

use necsim_impls_no_std::cogs::{
    active_lineage_sampler::classical::ClassicalActiveLineageSampler,
    origin_sampler::{resuming::ResumingOriginSampler, TrustedOriginSampler},
};
use necsim_partitioning_core::LocalPartition;

use rustcoalescence_algorithms::result::ResumeError;
use rustcoalescence_scenarios::Scenario;

use super::ClassicalLineageStoreSampleInitialiser;

#[expect(clippy::module_name_repetitions)]
pub struct ResumeInitialiser<L: ExactSizeIterator<Item = Lineage>> {
    pub lineages: L,
    pub resume_after: Option<NonNegativeF64>,
}

impl<L: ExactSizeIterator<Item = Lineage>, M: MathsCore, G: RngCore<M>, O: Scenario<M, G>>
    ClassicalLineageStoreSampleInitialiser<M, G, O, ResumeError<!>> for ResumeInitialiser<L>
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
        ResumeError<!>,
    >
    where
        O::Habitat: 'h,
    {
        let habitat = origin_sampler.habitat();
        let pre_sampler = origin_sampler.into_pre_sampler();

        let (lineage_store, active_lineage_sampler, exceptional_lineages) =
            ClassicalActiveLineageSampler::resume_with_store(
                ResumingOriginSampler::new(habitat, pre_sampler, self.lineages),
                self.resume_after.unwrap_or(NonNegativeF64::zero()),
            );

        if !exceptional_lineages.is_empty() {
            return Err(ResumeError::Sample(exceptional_lineages));
        }

        Ok((lineage_store, dispersal_sampler, active_lineage_sampler))
    }
}
