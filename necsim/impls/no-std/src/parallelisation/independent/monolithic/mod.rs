use alloc::collections::VecDeque;
use core::{
    num::{NonZeroU64, Wrapping},
    ops::ControlFlow,
};

use necsim_core_bond::{NonNegativeF64, PositiveF64};

use necsim_core::{
    cogs::{
        DispersalSampler, Habitat, MathsCore, PrimeableRng, SpeciationProbability, TurnoverRate,
    },
    lineage::Lineage,
    reporter::{boolean::Boolean, Reporter},
    simulation::Simulation,
};

use necsim_partitioning_core::LocalPartition;

use crate::{
    cogs::{
        active_lineage_sampler::singular::SingularActiveLineageSampler,
        coalescence_sampler::independent::IndependentCoalescenceSampler,
        emigration_exit::never::NeverEmigrationExit,
        event_sampler::{
            independent::IndependentEventSampler, tracking::MinSpeciationTrackingEventSampler,
        },
        immigration_entry::never::NeverImmigrationEntry,
        lineage_store::independent::IndependentLineageStore,
    },
    parallelisation::Status,
};

use crate::parallelisation::independent::{DedupCache, EventSlice};

pub mod reporter;

use reporter::{
    WaterLevelReporterConstructor, WaterLevelReporterProxy, WaterLevelReporterStrategy,
};

#[expect(clippy::type_complexity, clippy::too_many_lines)]
pub fn simulate<
    'p,
    M: MathsCore,
    H: Habitat<M>,
    G: PrimeableRng<M>,
    D: DispersalSampler<M, H, G>,
    T: TurnoverRate<M, H>,
    N: SpeciationProbability<M, H>,
    A: SingularActiveLineageSampler<
        M,
        H,
        G,
        IndependentLineageStore<M, H>,
        NeverEmigrationExit,
        D,
        IndependentCoalescenceSampler<M, H>,
        T,
        N,
        IndependentEventSampler<M, H, G, NeverEmigrationExit, D, T, N>,
        NeverImmigrationEntry,
    >,
    R: Reporter,
    P: LocalPartition<'p, R>,
    L: IntoIterator<Item = Lineage>,
>(
    simulation: &mut Simulation<
        M,
        H,
        G,
        IndependentLineageStore<M, H>,
        NeverEmigrationExit,
        D,
        IndependentCoalescenceSampler<M, H>,
        T,
        N,
        IndependentEventSampler<M, H, G, NeverEmigrationExit, D, T, N>,
        NeverImmigrationEntry,
        A,
    >,
    lineages: L,
    dedup_cache: DedupCache,
    step_slice: NonZeroU64,
    event_slice: EventSlice,
    pause_before: Option<NonNegativeF64>,
    local_partition: &mut P,
) -> (
    Status,
    NonNegativeF64,
    u64,
    impl IntoIterator<Item = Lineage>,
) {
    let mut slow_lineages = lineages
        .into_iter()
        .map(|lineage| {
            // We only need a strict lower bound here,
            //  i.e. that the next event >= pessimistic_next_event_time
            let pessimistic_next_event_time = lineage.last_event_time;

            (lineage, pessimistic_next_event_time)
        })
        .collect::<VecDeque<_>>();
    let mut fast_lineages = VecDeque::new();

    // Ensure that the progress bar starts with the expected target
    local_partition.report_progress_sync(
        (Wrapping(slow_lineages.len() as u64) + simulation.get_balanced_remaining_work()).0,
    );

    let event_slice = event_slice.capacity(slow_lineages.len());

    let mut proxy = <WaterLevelReporterStrategy as WaterLevelReporterConstructor<
        P::IsLive,
        R,
        P,
    >>::WaterLevelReporter::new(event_slice.get(), local_partition);
    let mut min_spec_samples = dedup_cache.construct(slow_lineages.len());

    let mut total_steps = 0_u64;
    let mut max_time = slow_lineages
        .iter()
        .map(|(lineage, _)| lineage.last_event_time)
        .max()
        .unwrap_or(NonNegativeF64::zero());

    let mut level_time = slow_lineages
        .iter()
        .map(|(lineage, _)| lineage.last_event_time)
        .min()
        .unwrap_or(NonNegativeF64::zero());

    while !slow_lineages.is_empty()
        && pause_before.map_or(true, |pause_before| level_time < pause_before)
    {
        // Calculate a new water-level time which all individuals should reach
        let total_event_rate: NonNegativeF64 = if R::ReportDispersal::VALUE {
            // Full event rate lambda with speciation
            slow_lineages
                .iter()
                .map(|(lineage, _)| {
                    simulation.turnover_rate().get_turnover_rate_at_location(
                        lineage.indexed_location.location(),
                        simulation.habitat(),
                    )
                })
                .sum()
        } else if R::ReportSpeciation::VALUE {
            // Only speciation event rate lambda * nu
            slow_lineages
                .iter()
                .map(|(lineage, _)| {
                    let location = lineage.indexed_location.location();

                    simulation
                        .turnover_rate()
                        .get_turnover_rate_at_location(location, simulation.habitat())
                        * simulation
                            .speciation_probability()
                            .get_speciation_probability_at_location(location, simulation.habitat())
                })
                .sum()
        } else {
            // No events produced -> no restriction
            NonNegativeF64::zero()
        };

        level_time += NonNegativeF64::from(event_slice.get()) / total_event_rate;

        if let Some(pause_before) = pause_before {
            level_time = level_time.min(pause_before);
        }

        // [Report all events below the water level] + Advance the water level
        proxy.advance_water_level(level_time);

        let mut previous_next_event_time: Option<PositiveF64> = None;

        // Simulate all slow lineages until they have finished or exceeded the new water
        //  level
        while !slow_lineages.is_empty()
            || simulation.active_lineage_sampler().number_active_lineages() > 0
        {
            let next_slow_lineage = loop {
                match slow_lineages.pop_front() {
                    None => break None,
                    Some((slow_lineage, next_event)) if next_event < level_time => {
                        break Some(slow_lineage)
                    },
                    Some((fast_lineage, next_event)) => {
                        fast_lineages.push_back((fast_lineage, next_event));
                    },
                }
            };

            let previous_task = simulation
                .active_lineage_sampler_mut()
                .replace_active_lineage(next_slow_lineage);

            let previous_speciation_sample =
                simulation.event_sampler_mut().replace_min_speciation(None);

            if let (Some(previous_task), Some(previous_next_event_time)) =
                (previous_task, previous_next_event_time)
            {
                let duplicate_individual = previous_speciation_sample
                    .map_or(false, |spec_sample| !min_spec_samples.insert(spec_sample));

                if !duplicate_individual {
                    // Reclassify lineages as either slow (still below water) or fast
                    if previous_next_event_time < level_time {
                        slow_lineages.push_back((previous_task, previous_next_event_time.into()));
                    } else {
                        fast_lineages.push_back((previous_task, previous_next_event_time.into()));
                    }
                }
            }

            // Note: Coalescence consistency
            //  If a jumps to b at the same time as b jumps to a,
            //  no coalescence occurs as coalescence would only be
            //  detected at the next shared duplicate event

            previous_next_event_time = None;

            let (new_time, new_steps) = simulation.simulate_incremental_early_stop(
                |_, steps, next_event_time, _| {
                    previous_next_event_time = Some(next_event_time);

                    if steps >= step_slice.get() || next_event_time >= level_time {
                        ControlFlow::Break(())
                    } else {
                        ControlFlow::Continue(())
                    }
                },
                &mut proxy,
            );

            total_steps += new_steps;
            max_time = max_time.max(new_time);

            proxy.local_partition().get_reporter().report_progress(
                &(Wrapping(slow_lineages.len() as u64)
                    + Wrapping(fast_lineages.len() as u64)
                    + simulation.get_balanced_remaining_work())
                .0
                .into(),
            );
        }

        // Fast lineages are now slow again
        core::mem::swap(&mut slow_lineages, &mut fast_lineages);
    }

    // [Report all remaining events]
    proxy.finalise();

    local_partition.report_progress_sync(
        (Wrapping(slow_lineages.len() as u64) + simulation.get_balanced_remaining_work()).0,
    );

    let status = Status::paused(local_partition.reduce_vote_any(!slow_lineages.is_empty()));
    let local_time = max_time;
    let local_steps = total_steps;
    let lineages = slow_lineages.into_iter().map(|(lineage, _)| lineage);

    (status, local_time, local_steps, lineages)
}
