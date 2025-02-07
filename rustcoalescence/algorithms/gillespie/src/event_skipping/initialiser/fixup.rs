use necsim_core::{
    cogs::{
        EmigrationExit, GloballyCoherentLineageStore, ImmigrationEntry, MathsCore, RngCore,
        SeparableDispersalSampler,
    },
    event::DispersalEvent,
    lineage::{Lineage, LineageInteraction},
    reporter::Reporter,
};
use necsim_core_bond::PositiveF64;

use necsim_impls_no_std::cogs::{
    active_lineage_sampler::{
        alias::location::LocationAliasActiveLineageSampler,
        resuming::{
            lineage::{ExceptionalLineage, SplitExceptionalLineages},
            RestartFixUpActiveLineageSampler,
        },
    },
    coalescence_sampler::conditional::ConditionalCoalescenceSampler,
    dispersal_sampler::trespassing::{
        uniform::UniformAntiTrespassingDispersalSampler, TrespassingDispersalSampler,
    },
    event_sampler::gillespie::conditional::ConditionalGillespieEventSampler,
    origin_sampler::{resuming::ResumingOriginSampler, TrustedOriginSampler},
};
use necsim_partitioning_core::LocalPartition;

use rustcoalescence_algorithms::{
    result::ResumeError,
    strategy::{
        CoalescenceStrategy, OutOfDemeStrategy, OutOfHabitatStrategy, RestartFixUpStrategy,
    },
};
use rustcoalescence_scenarios::Scenario;

use super::EventSkippingLineageStoreSampleInitialiser;

pub struct FixUpInitialiser<L: ExactSizeIterator<Item = Lineage>> {
    pub lineages: L,
    pub restart_at: PositiveF64,
    pub fixup_strategy: RestartFixUpStrategy,
}

impl<L: ExactSizeIterator<Item = Lineage>, M: MathsCore, G: RngCore<M>, O: Scenario<M, G>>
    EventSkippingLineageStoreSampleInitialiser<M, G, O, ResumeError<!>> for FixUpInitialiser<L>
where
    O::DispersalSampler: SeparableDispersalSampler<M, O::Habitat, G>,
{
    type ActiveLineageSampler<
        S: GloballyCoherentLineageStore<M, O::Habitat>,
        X: EmigrationExit<M, O::Habitat, G, S>,
        I: ImmigrationEntry<M>,
    > = RestartFixUpActiveLineageSampler<
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
        LocationAliasActiveLineageSampler<
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
        >,
    >;
    type DispersalSampler = TrespassingDispersalSampler<
        M,
        O::Habitat,
        G,
        O::DispersalSampler,
        UniformAntiTrespassingDispersalSampler<M, O::Habitat, G>,
    >;

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
        local_partition: &mut P,
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
        let dispersal_sampler = TrespassingDispersalSampler::new(
            dispersal_sampler,
            UniformAntiTrespassingDispersalSampler::default(),
        );

        let (lineage_store, active_lineage_sampler, exceptional_lineages) =
            LocationAliasActiveLineageSampler::resume_with_store(
                ResumingOriginSampler::new(habitat, pre_sampler, self.lineages),
                &dispersal_sampler,
                coalescence_sampler,
                turnover_rate,
                speciation_probability,
                &event_sampler,
                self.restart_at.into(),
            );

        let SplitExceptionalLineages {
            mut coalescence,
            out_of_deme,
            out_of_habitat,
        } = ExceptionalLineage::split_vec(exceptional_lineages);

        let mut exceptional_lineages = Vec::new();
        let mut fixable_lineages = Vec::new();

        match self.fixup_strategy.coalescence {
            CoalescenceStrategy::Abort => {
                exceptional_lineages.extend(
                    coalescence
                        .into_iter()
                        .map(|(child, parent)| ExceptionalLineage::Coalescence { child, parent }),
                );
            },
            CoalescenceStrategy::Coalescence => {
                coalescence.sort_unstable();

                for (coalescing_lineage, parent) in coalescence {
                    local_partition.get_reporter().report_dispersal(
                        (&DispersalEvent {
                            global_lineage_reference: coalescing_lineage.global_reference,
                            prior_time: coalescing_lineage.last_event_time,
                            event_time: self.restart_at,
                            origin: coalescing_lineage.indexed_location.clone(),
                            target: coalescing_lineage.indexed_location,
                            interaction: LineageInteraction::Coalescence(parent),
                        })
                            .into(),
                    );
                }
            },
        }

        match self.fixup_strategy.out_of_deme {
            OutOfDemeStrategy::Abort => {
                exceptional_lineages
                    .extend(out_of_deme.into_iter().map(ExceptionalLineage::OutOfDeme));
            },
            OutOfDemeStrategy::Dispersal => {
                fixable_lineages.extend(out_of_deme);
            },
        }

        match self.fixup_strategy.out_of_habitat {
            OutOfHabitatStrategy::Abort => {
                exceptional_lineages.extend(
                    out_of_habitat
                        .into_iter()
                        .map(ExceptionalLineage::OutOfHabitat),
                );
            },
            OutOfHabitatStrategy::UniformDispersal => {
                fixable_lineages.extend(out_of_habitat);
            },
        }

        if !exceptional_lineages.is_empty() {
            return Err(ResumeError::Sample(exceptional_lineages));
        }

        fixable_lineages.sort_unstable();

        let active_lineage_sampler = RestartFixUpActiveLineageSampler::new(
            active_lineage_sampler,
            fixable_lineages,
            self.restart_at,
        );

        Ok((
            lineage_store,
            dispersal_sampler,
            event_sampler,
            active_lineage_sampler,
        ))
    }
}
