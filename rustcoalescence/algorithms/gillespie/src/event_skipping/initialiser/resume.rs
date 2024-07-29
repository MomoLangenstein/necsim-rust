use necsim_core::{
    cogs::{
        EmigrationExit, GloballyCoherentLineageStore, ImmigrationEntry, MathsCore, RngCore,
        SeparableDispersalSampler,
    },
    lineage::Lineage,
    reporter::Reporter,
};
use necsim_core_bond::NonNegativeF64;

use necsim_impls_no_std::cogs::{
    active_lineage_sampler::alias::location::LocationAliasActiveLineageSampler,
    coalescence_sampler::conditional::ConditionalCoalescenceSampler,
    event_sampler::gillespie::conditional::ConditionalGillespieEventSampler,
    origin_sampler::{resuming::ResumingOriginSampler, TrustedOriginSampler},
};
use necsim_partitioning_core::LocalPartition;

use rustcoalescence_algorithms::result::ResumeError;
use rustcoalescence_scenarios::Scenario;

use super::EventSkippingLineageStoreSampleInitialiser;

#[expect(clippy::module_name_repetitions)]
pub struct ResumeInitialiser<L: ExactSizeIterator<Item = Lineage>> {
    pub lineages: L,
    pub resume_after: Option<NonNegativeF64>,
}

impl<L: ExactSizeIterator<Item = Lineage>, M: MathsCore, G: RngCore<M>, O: Scenario<M, G>>
    EventSkippingLineageStoreSampleInitialiser<M, G, O, ResumeError<!>> for ResumeInitialiser<L>
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
        ResumeError<!>,
    >
    where
        O::Habitat: 'h,
    {
        let habitat = origin_sampler.habitat();
        let pre_sampler = origin_sampler.into_pre_sampler();

        let event_sampler = ConditionalGillespieEventSampler::default();

        let (lineage_store, active_lineage_sampler, exceptional_lineages) =
            LocationAliasActiveLineageSampler::resume_with_store(
                ResumingOriginSampler::new(habitat, pre_sampler, self.lineages),
                &dispersal_sampler,
                coalescence_sampler,
                turnover_rate,
                speciation_probability,
                &event_sampler,
                self.resume_after.unwrap_or(NonNegativeF64::zero()),
            );

        if !exceptional_lineages.is_empty() {
            return Err(ResumeError::Sample(exceptional_lineages));
        }

        Ok((
            lineage_store,
            dispersal_sampler,
            event_sampler,
            active_lineage_sampler,
        ))
    }
}
