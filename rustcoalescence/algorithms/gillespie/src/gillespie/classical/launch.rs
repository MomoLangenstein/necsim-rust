use std::{hint::unreachable_unchecked, marker::PhantomData};

use necsim_core::{
    cogs::{ActiveLineageSampler, LocallyCoherentLineageStore, MathsCore, SplittableRng},
    reporter::Reporter,
    simulation::SimulationBuilder,
};
use necsim_core_bond::NonNegativeF64;

use necsim_impls_no_std::{
    cogs::{
        coalescence_sampler::unconditional::UnconditionalCoalescenceSampler,
        emigration_exit::{domain::DomainEmigrationExit, never::NeverEmigrationExit},
        event_sampler::unconditional::UnconditionalEventSampler,
        immigration_entry::{buffered::BufferedImmigrationEntry, never::NeverImmigrationEntry},
        lineage_store::coherent::locally::classical::ClassicalLineageStore,
        origin_sampler::{
            decomposition::DecompositionOriginSampler, pre_sampler::OriginPreSampler,
        },
        turnover_rate::uniform::UniformTurnoverRate,
    },
    parallelisation::{self, Status},
};
use necsim_partitioning_core::LocalPartition;

use rustcoalescence_algorithms::result::SimulationOutcome;
use rustcoalescence_scenarios::{Scenario, ScenarioCogs};

use crate::arguments::{
    AveragingParallelismMode, GillespieArguments, OptimisticParallelismMode, ParallelismMode,
};

use super::initialiser::ClassicalLineageStoreSampleInitialiser;

#[expect(clippy::too_many_lines)]
pub fn initialise_and_simulate<
    'p,
    M: MathsCore,
    G: SplittableRng<M>,
    O: Scenario<M, G, TurnoverRate = UniformTurnoverRate>,
    R: Reporter,
    P: LocalPartition<'p, R>,
    I: Iterator<Item = u64>,
    L: ClassicalLineageStoreSampleInitialiser<M, G, O, Error>,
    Error,
>(
    args: GillespieArguments,
    rng: G,
    scenario: ScenarioCogs<M, G, O>,
    pre_sampler: OriginPreSampler<M, I>,
    pause_before: Option<NonNegativeF64>,
    local_partition: &mut P,
    lineage_store_sampler_initialiser: L,
) -> Result<SimulationOutcome<M, G>, Error>
where
    O::LineageStore<ClassicalLineageStore<M, O::Habitat>>:
        LocallyCoherentLineageStore<M, O::Habitat>,
{
    match args.parallelism_mode {
        ParallelismMode::Monolithic => {
            let ScenarioCogs {
                habitat,
                dispersal_sampler,
                turnover_rate,
                speciation_probability,
                origin_sampler_auxiliary,
                decomposition_auxiliary: _,
                ..
            } = scenario;
            let coalescence_sampler = UnconditionalCoalescenceSampler::default();
            let event_sampler = UnconditionalEventSampler::default();

            let (lineage_store, dispersal_sampler, active_lineage_sampler): (
                O::LineageStore<ClassicalLineageStore<M, O::Habitat>>,
                _,
                _,
            ) = lineage_store_sampler_initialiser.init(
                O::sample_habitat(&habitat, pre_sampler, origin_sampler_auxiliary),
                dispersal_sampler,
                local_partition,
            )?;

            let emigration_exit = NeverEmigrationExit::default();
            let immigration_entry = NeverImmigrationEntry::default();

            let mut simulation = SimulationBuilder {
                maths: PhantomData::<M>,
                habitat,
                lineage_store,
                dispersal_sampler,
                coalescence_sampler,
                turnover_rate,
                speciation_probability,
                emigration_exit,
                event_sampler,
                active_lineage_sampler,
                rng,
                immigration_entry,
            }
            .build();

            let (status, time, steps) = parallelisation::monolithic::monolithic::simulate(
                &mut simulation,
                pause_before,
                local_partition,
            );

            match status {
                Status::Done => Ok(SimulationOutcome::Done { time, steps }),
                Status::Paused => Ok(SimulationOutcome::Paused {
                    time,
                    steps,
                    lineages: simulation
                        .active_lineage_sampler()
                        .iter_active_lineages_ordered(
                            simulation.habitat(),
                            simulation.lineage_store(),
                        )
                        .cloned()
                        .collect(),
                    rng: simulation.rng_mut().clone(),
                    marker: PhantomData::<M>,
                }),
            }
        },
        non_monolithic_parallelism_mode => {
            let rng = rng.split_to_stream(u64::from(local_partition.get_partition().rank()));

            let ScenarioCogs {
                habitat,
                dispersal_sampler,
                turnover_rate,
                speciation_probability,
                origin_sampler_auxiliary,
                decomposition_auxiliary,
                ..
            } = scenario;
            let coalescence_sampler = UnconditionalCoalescenceSampler::default();
            let event_sampler = UnconditionalEventSampler::default();

            let decomposition = O::decompose(
                &habitat,
                local_partition.get_partition(),
                decomposition_auxiliary,
            );
            let origin_sampler = DecompositionOriginSampler::new(
                O::sample_habitat(&habitat, pre_sampler, origin_sampler_auxiliary),
                &decomposition,
            );

            let (lineage_store, dispersal_sampler, active_lineage_sampler): (
                O::LineageStore<ClassicalLineageStore<M, O::Habitat>>,
                _,
                _,
            ) = lineage_store_sampler_initialiser.init(
                origin_sampler,
                dispersal_sampler,
                local_partition,
            )?;

            let emigration_exit = DomainEmigrationExit::new(decomposition);
            let immigration_entry = BufferedImmigrationEntry::default();

            let mut simulation = SimulationBuilder {
                maths: PhantomData::<M>,
                habitat,
                lineage_store,
                dispersal_sampler,
                coalescence_sampler,
                turnover_rate,
                speciation_probability,
                emigration_exit,
                event_sampler,
                active_lineage_sampler,
                rng,
                immigration_entry,
            }
            .build();

            let (_status, time, steps) = match non_monolithic_parallelism_mode {
                ParallelismMode::Monolithic => unsafe { unreachable_unchecked() },
                ParallelismMode::Optimistic(OptimisticParallelismMode { delta_sync }) => {
                    parallelisation::monolithic::optimistic::simulate(
                        &mut simulation,
                        delta_sync,
                        local_partition,
                    )
                },
                ParallelismMode::Lockstep => parallelisation::monolithic::lockstep::simulate(
                    &mut simulation,
                    local_partition,
                ),
                ParallelismMode::OptimisticLockstep => {
                    parallelisation::monolithic::optimistic_lockstep::simulate(
                        &mut simulation,
                        local_partition,
                    )
                },
                ParallelismMode::Averaging(AveragingParallelismMode { delta_sync }) => {
                    parallelisation::monolithic::averaging::simulate(
                        &mut simulation,
                        delta_sync,
                        local_partition,
                    )
                },
            };

            // TODO: Adapt for parallel pausing
            Ok(SimulationOutcome::Done { time, steps })
        },
    }
}
