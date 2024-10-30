use std::marker::PhantomData;

use necsim_core::{
    cogs::{MathsCore, PrimeableRng},
    reporter::Reporter,
    simulation::SimulationBuilder,
};
use necsim_core_bond::NonNegativeF64;

use necsim_impls_no_std::{
    cogs::{
        active_lineage_sampler::independent::event_time_sampler::poisson::PoissonEventTimeSampler,
        coalescence_sampler::independent::IndependentCoalescenceSampler,
        emigration_exit::{
            independent::{
                choice::{
                    always::AlwaysEmigrationChoice, probabilistic::ProbabilisticEmigrationChoice,
                },
                IndependentEmigrationExit,
            },
            never::NeverEmigrationExit,
        },
        event_sampler::independent::IndependentEventSampler,
        immigration_entry::never::NeverImmigrationEntry,
        origin_sampler::{
            decomposition::DecompositionOriginSampler, pre_sampler::OriginPreSampler,
        },
    },
    parallelisation::{self, Status},
};
use necsim_partitioning_core::LocalPartition;

use rustcoalescence_algorithms::result::SimulationOutcome;
use rustcoalescence_scenarios::{Scenario, ScenarioCogs};

use crate::{
    arguments::{
        IndependentArguments, IsolatedParallelismMode, MonolithicParallelismMode, ParallelismMode,
        ProbabilisticParallelismMode,
    },
    initialiser::IndependentLineageStoreSampleInitialiser,
};

#[expect(clippy::too_many_lines)]
pub fn initialise_and_simulate<
    'p,
    M: MathsCore,
    G: PrimeableRng<M>,
    O: Scenario<M, G>,
    R: Reporter,
    P: LocalPartition<'p, R>,
    I: Iterator<Item = u64>,
    L: IndependentLineageStoreSampleInitialiser<M, G, O, Error>,
    Error,
>(
    args: &IndependentArguments,
    rng: G,
    scenario: ScenarioCogs<M, G, O>,
    pre_sampler: OriginPreSampler<M, I>,
    pause_before: Option<NonNegativeF64>,
    local_partition: &mut P,
    lineage_store_sampler_initialiser: L,
) -> Result<SimulationOutcome<M, G>, Error> {
    match args.parallelism_mode {
        ParallelismMode::Monolithic(MonolithicParallelismMode { event_slice })
        | ParallelismMode::IsolatedIndividuals(IsolatedParallelismMode { event_slice, .. })
        | ParallelismMode::IsolatedLandscape(IsolatedParallelismMode { event_slice, .. }) => {
            let ScenarioCogs {
                habitat,
                dispersal_sampler,
                turnover_rate,
                speciation_probability,
                origin_sampler_auxiliary,
                decomposition_auxiliary,
                ..
            } = scenario;
            let coalescence_sampler = IndependentCoalescenceSampler::default();
            let event_sampler = IndependentEventSampler::default();

            let (lineage_store, dispersal_sampler, active_lineage_sampler, lineages, passthrough) =
                match args.parallelism_mode {
                    // Apply no lineage origin partitioning in the `Monolithic` mode
                    ParallelismMode::Monolithic(..) => lineage_store_sampler_initialiser.init(
                        O::sample_habitat(&habitat, pre_sampler, origin_sampler_auxiliary),
                        dispersal_sampler,
                        PoissonEventTimeSampler::new(args.delta_t),
                    )?,
                    // Apply lineage origin partitioning in the `IsolatedIndividuals` mode
                    ParallelismMode::IsolatedIndividuals(IsolatedParallelismMode {
                        partition,
                        ..
                    }) => lineage_store_sampler_initialiser.init(
                        O::sample_habitat(
                            &habitat,
                            pre_sampler.partition(partition),
                            origin_sampler_auxiliary,
                        ),
                        dispersal_sampler,
                        PoissonEventTimeSampler::new(args.delta_t),
                    )?,
                    // Apply lineage origin partitioning in the `IsolatedLandscape` mode
                    ParallelismMode::IsolatedLandscape(IsolatedParallelismMode {
                        partition,
                        ..
                    }) => lineage_store_sampler_initialiser.init(
                        DecompositionOriginSampler::new(
                            O::sample_habitat(&habitat, pre_sampler, origin_sampler_auxiliary),
                            &O::decompose(&habitat, partition, decomposition_auxiliary),
                        ),
                        dispersal_sampler,
                        PoissonEventTimeSampler::new(args.delta_t),
                    )?,
                    _ => unsafe { std::hint::unreachable_unchecked() },
                };

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

            let (mut status, time, steps, lineages) =
                parallelisation::independent::monolithic::simulate(
                    &mut simulation,
                    lineages,
                    args.dedup_cache,
                    args.step_slice,
                    event_slice,
                    pause_before,
                    local_partition,
                );

            if !passthrough.is_empty() {
                status = Status::Paused;
            }

            match status {
                Status::Done => Ok(SimulationOutcome::Done { time, steps }),
                Status::Paused => Ok(SimulationOutcome::Paused {
                    time,
                    steps,
                    lineages: lineages.into_iter().chain(passthrough).collect(),
                    rng: simulation.rng_mut().clone(),
                    marker: PhantomData::<M>,
                }),
            }
        },
        ParallelismMode::Individuals => {
            let ScenarioCogs {
                habitat,
                dispersal_sampler,
                turnover_rate,
                speciation_probability,
                origin_sampler_auxiliary,
                decomposition_auxiliary: _,
                ..
            } = scenario;
            let coalescence_sampler = IndependentCoalescenceSampler::default();
            let event_sampler = IndependentEventSampler::default();

            let (lineage_store, dispersal_sampler, active_lineage_sampler, lineages, _passthrough) =
                lineage_store_sampler_initialiser.init(
                    O::sample_habitat(
                        &habitat,
                        pre_sampler.partition(local_partition.get_partition()),
                        origin_sampler_auxiliary,
                    ),
                    dispersal_sampler,
                    PoissonEventTimeSampler::new(args.delta_t),
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

            let (_status, time, steps, _lineages) =
                parallelisation::independent::individuals::simulate(
                    &mut simulation,
                    lineages,
                    args.dedup_cache,
                    args.step_slice,
                    local_partition,
                );

            // TODO: Adapt for parallel pausing
            // TODO: Adapt for lineage passthrough
            Ok(SimulationOutcome::Done { time, steps })
        },
        ParallelismMode::Landscape => {
            let ScenarioCogs {
                habitat,
                dispersal_sampler,
                turnover_rate,
                speciation_probability,
                origin_sampler_auxiliary,
                decomposition_auxiliary,
                ..
            } = scenario;
            let coalescence_sampler = IndependentCoalescenceSampler::default();
            let event_sampler = IndependentEventSampler::default();

            let decomposition = O::decompose(
                &habitat,
                local_partition.get_partition(),
                decomposition_auxiliary,
            );

            let (lineage_store, dispersal_sampler, active_lineage_sampler, lineages, _passthrough) =
                lineage_store_sampler_initialiser.init(
                    DecompositionOriginSampler::new(
                        O::sample_habitat(&habitat, pre_sampler, origin_sampler_auxiliary),
                        &decomposition,
                    ),
                    dispersal_sampler,
                    PoissonEventTimeSampler::new(args.delta_t),
                )?;

            let emigration_exit =
                IndependentEmigrationExit::new(decomposition, AlwaysEmigrationChoice::default());
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

            let (_status, time, steps, _lineages) =
                parallelisation::independent::landscape::simulate(
                    &mut simulation,
                    lineages,
                    args.dedup_cache,
                    args.step_slice,
                    local_partition,
                );

            // TODO: Adapt for parallel pausing
            // TODO: Adapt for lineage passthrough
            Ok(SimulationOutcome::Done { time, steps })
        },
        ParallelismMode::Probabilistic(ProbabilisticParallelismMode {
            communication_probability,
        }) => {
            let ScenarioCogs {
                habitat,
                dispersal_sampler,
                turnover_rate,
                speciation_probability,
                origin_sampler_auxiliary,
                decomposition_auxiliary,
                ..
            } = scenario;
            let coalescence_sampler = IndependentCoalescenceSampler::default();
            let event_sampler = IndependentEventSampler::default();

            let decomposition = O::decompose(
                &habitat,
                local_partition.get_partition(),
                decomposition_auxiliary,
            );

            let (lineage_store, dispersal_sampler, active_lineage_sampler, lineages, _passthrough) =
                lineage_store_sampler_initialiser.init(
                    DecompositionOriginSampler::new(
                        O::sample_habitat(&habitat, pre_sampler, origin_sampler_auxiliary),
                        &decomposition,
                    ),
                    dispersal_sampler,
                    PoissonEventTimeSampler::new(args.delta_t),
                )?;

            let emigration_exit = IndependentEmigrationExit::new(
                decomposition,
                ProbabilisticEmigrationChoice::new(communication_probability),
            );
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

            let (_status, time, steps, _lineages) =
                parallelisation::independent::landscape::simulate(
                    &mut simulation,
                    lineages,
                    args.dedup_cache,
                    args.step_slice,
                    local_partition,
                );

            // TODO: Adapt for parallel pausing
            // TODO: Adapt for lineage passthrough
            Ok(SimulationOutcome::Done { time, steps })
        },
    }
}
