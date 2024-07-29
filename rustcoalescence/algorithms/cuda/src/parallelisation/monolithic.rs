use std::{collections::VecDeque, convert::TryInto, num::NonZeroU64, sync::atomic::AtomicU64};

use rust_cuda::{
    host::HostAndDeviceMutRef,
    kernel::Launcher,
    lend::{LendToCuda, RustToCuda},
    utils::exchange::wrapper::ExchangeWrapperOnHost,
};

use necsim_core::{
    cogs::{
        CoalescenceSampler, DispersalSampler, EmigrationExit, Habitat, ImmigrationEntry,
        LineageStore, MathsCore, PrimeableRng, SpeciationProbability, TurnoverRate,
    },
    lineage::Lineage,
    reporter::{boolean::Boolean, Reporter},
    simulation::Simulation,
};
use necsim_core_bond::NonNegativeF64;

use necsim_impls_no_std::{
    cogs::{
        active_lineage_sampler::singular::SingularActiveLineageSampler,
        event_sampler::tracking::MinSpeciationTrackingEventSampler,
    },
    parallelisation::{
        independent::{
            monolithic::reporter::{
                WaterLevelReporterConstructor, WaterLevelReporterProxy, WaterLevelReporterStrategy,
            },
            DedupCache, EventSlice,
        },
        Status,
    },
};
use necsim_partitioning_core::LocalPartition;

use necsim_impls_cuda::{event_buffer::EventBuffer, value_buffer::ValueBuffer};

use rustcoalescence_algorithms_cuda_gpu_kernel::simulate;

use crate::error::CudaError;

type Result<T, E = CudaError> = std::result::Result<T, E>;

#[expect(clippy::type_complexity, clippy::too_many_lines)]
pub fn simulate<
    'l,
    'p,
    M: MathsCore + Sync,
    H: Habitat<M> + RustToCuda + Sync,
    G: PrimeableRng<M> + RustToCuda + Sync,
    S: LineageStore<M, H> + RustToCuda + Sync,
    X: EmigrationExit<M, H, G, S> + RustToCuda + Sync,
    D: DispersalSampler<M, H, G> + RustToCuda + Sync,
    C: CoalescenceSampler<M, H, S> + RustToCuda + Sync,
    T: TurnoverRate<M, H> + RustToCuda + Sync,
    N: SpeciationProbability<M, H> + RustToCuda + Sync,
    E: MinSpeciationTrackingEventSampler<M, H, G, S, X, D, C, T, N> + RustToCuda + Sync,
    I: ImmigrationEntry<M> + RustToCuda + Sync,
    A: SingularActiveLineageSampler<M, H, G, S, X, D, C, T, N, E, I> + RustToCuda + Sync,
    P: Reporter,
    L: LocalPartition<'p, P>,
    LI: IntoIterator<Item = Lineage>,
>(
    simulation: &mut Simulation<M, H, G, S, X, D, C, T, N, E, I, A>,
    mut launcher: Launcher<simulate<
        M,
        H,
        G,
        S,
        X,
        D,
        C,
        T,
        N,
        E,
        I,
        A,
        <<WaterLevelReporterStrategy as WaterLevelReporterConstructor<L::IsLive, P, L>>::WaterLevelReporter as Reporter>::ReportSpeciation,
        <<WaterLevelReporterStrategy as WaterLevelReporterConstructor<L::IsLive, P, L>>::WaterLevelReporter as Reporter>::ReportDispersal,
    >>,
    config: (DedupCache, NonZeroU64),
    lineages: LI,
    event_slice: EventSlice,
    pause_before: Option<NonNegativeF64>,
    local_partition: &'l mut L,
) -> Result<(
    Status,
    NonNegativeF64,
    u64,
    impl IntoIterator<Item = Lineage>,
)> {
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
    local_partition.report_progress_sync(slow_lineages.len() as u64);

    let event_slice = event_slice.capacity(slow_lineages.len());

    let mut proxy = <WaterLevelReporterStrategy as WaterLevelReporterConstructor<
        L::IsLive,
        P,
        L,
    >>::WaterLevelReporter::new(event_slice.get(), local_partition);

    let (dedup_cache, step_slice) = config;

    let intial_max_time = slow_lineages
        .iter()
        .map(|(lineage, _)| lineage.last_event_time)
        .max()
        .unwrap_or(NonNegativeF64::zero());

    // Initialise the total_time_max and total_steps_sum atomics
    let mut total_time_max = AtomicU64::new(intial_max_time.get().to_bits());
    let mut total_steps_sum = AtomicU64::new(0_u64);

    let mut task_list = ExchangeWrapperOnHost::new(ValueBuffer::new(
        &launcher.config.block,
        &launcher.config.grid,
    )?)?;
    let mut event_buffer: ExchangeWrapperOnHost<
        EventBuffer<
            <<WaterLevelReporterStrategy as WaterLevelReporterConstructor<
                L::IsLive,
                P,
                L,
            >>::WaterLevelReporter as Reporter>::ReportSpeciation,
            <<WaterLevelReporterStrategy as WaterLevelReporterConstructor<
                L::IsLive,
                P,
                L,
            >>::WaterLevelReporter as Reporter>::ReportDispersal,
        >,
    > = ExchangeWrapperOnHost::new(EventBuffer::new(
        &launcher.config.block, &launcher.config.grid,
        step_slice.get().try_into().unwrap_or(usize::MAX),
    )?)?;
    let mut min_spec_sample_buffer = ExchangeWrapperOnHost::new(ValueBuffer::new(
        &launcher.config.block,
        &launcher.config.grid,
    )?)?;
    let mut next_event_time_buffer = ExchangeWrapperOnHost::new(ValueBuffer::new(
        &launcher.config.block,
        &launcher.config.grid,
    )?)?;

    let mut min_spec_samples = dedup_cache.construct(slow_lineages.len());

    let mut level_time = slow_lineages
        .iter()
        .map(|(lineage, _)| lineage.last_event_time)
        .min()
        .unwrap_or(NonNegativeF64::zero());

    let cpu_habitat = simulation.habitat().backup();
    let cpu_turnover_rate = simulation.turnover_rate().backup();
    let cpu_speciation_probability = simulation.speciation_probability().backup();

    HostAndDeviceMutRef::with_new(&mut total_time_max, |total_time_max| -> Result<()> {
        HostAndDeviceMutRef::with_new(&mut total_steps_sum, |total_steps_sum| -> Result<()> {
            simulation.lend_to_cuda(|simulation_cuda_repr| -> Result<()> {
                while !slow_lineages.is_empty()
                    && pause_before.map_or(true, |pause_before| level_time < pause_before)
                {
                    let total_event_rate: NonNegativeF64 = if P::ReportDispersal::VALUE {
                        // Full event rate lambda with speciation
                        slow_lineages
                            .iter()
                            .map(|(lineage, _)| {
                                cpu_turnover_rate.get_turnover_rate_at_location(
                                    lineage.indexed_location.location(),
                                    &cpu_habitat,
                                )
                            })
                            .sum()
                    } else if P::ReportSpeciation::VALUE {
                        // Only speciation event rate lambda * nu
                        slow_lineages
                            .iter()
                            .map(|(lineage, _)| {
                                let location = lineage.indexed_location.location();

                                cpu_turnover_rate
                                    .get_turnover_rate_at_location(location, &cpu_habitat)
                                    * cpu_speciation_probability
                                        .get_speciation_probability_at_location(
                                            location,
                                            &cpu_habitat,
                                        )
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

                    // Simulate all slow lineages until they have finished or exceeded the
                    //  new water  level
                    while !slow_lineages.is_empty() {
                        // Move the event buffer and min speciation sample buffer to CUDA
                        let mut event_buffer_cuda_async =
                            event_buffer.move_to_device_async(launcher.stream)?;
                        let mut min_spec_sample_buffer_cuda_async =
                            min_spec_sample_buffer.move_to_device_async(launcher.stream)?;
                        let mut next_event_time_buffer_cuda_async =
                            next_event_time_buffer.move_to_device_async(launcher.stream)?;

                        // Upload the new tasks from the front of the task queue
                        for mut task in task_list.iter_mut() {
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

                            task.replace(next_slow_lineage);
                        }

                        // Move the task list to CUDA
                        let mut task_list_cuda_async =
                            task_list.move_to_device_async(launcher.stream)?;

                        let launch = launcher.launch9_async(
                            simulation_cuda_repr.as_async(launcher.stream).extract_ref(),
                            task_list_cuda_async.as_mut_async(),
                            event_buffer_cuda_async.as_mut_async(),
                            min_spec_sample_buffer_cuda_async.as_mut_async(),
                            next_event_time_buffer_cuda_async.as_mut_async(),
                            total_time_max
                                .as_ref()
                                .as_async(launcher.stream)
                                .extract_ref(),
                            total_steps_sum
                                .as_ref()
                                .as_async(launcher.stream)
                                .extract_ref(),
                            step_slice.get(),
                            level_time,
                        )?;

                        let min_spec_sample_buffer_host_async =
                            min_spec_sample_buffer_cuda_async
                                .move_to_host_async(launcher.stream)?;
                        let next_event_time_buffer_host_async =
                            next_event_time_buffer_cuda_async
                                .move_to_host_async(launcher.stream)?;
                        let task_list_host_async =
                            task_list_cuda_async.move_to_host_async(launcher.stream)?;
                        let event_buffer_host_async =
                            event_buffer_cuda_async.move_to_host_async(launcher.stream)?;

                        task_list = task_list_host_async.synchronize()?;
                        next_event_time_buffer = next_event_time_buffer_host_async.synchronize()?;
                        min_spec_sample_buffer = min_spec_sample_buffer_host_async.synchronize()?;

                        launch.synchronize()?;

                        // Fetch the completion of the tasks
                        for ((mut spec_sample, mut next_event_time), mut task) in
                            min_spec_sample_buffer
                                .iter_mut()
                                .zip(next_event_time_buffer.iter_mut())
                                .zip(task_list.iter_mut())
                        {
                            let duplicate_individual = spec_sample
                                .take()
                                .map_or(false, |spec_sample| !min_spec_samples.insert(spec_sample));

                            if let (Some(task), Some(next_event_time)) =
                                (task.take(), next_event_time.take())
                            {
                                if !duplicate_individual {
                                    // Reclassify lineages as either slow (still below
                                    //  the metaphorical water level) or fast
                                    if next_event_time < level_time {
                                        slow_lineages.push_back((task, next_event_time.into()));
                                    } else {
                                        fast_lineages.push_back((task, next_event_time.into()));
                                    }
                                }
                            }
                        }

                        event_buffer = event_buffer_host_async.synchronize()?;
                        // TODO: explore partial sorting on the GPU
                        event_buffer.report_events_unordered(&mut proxy);

                        proxy.local_partition().get_reporter().report_progress(
                            &((slow_lineages.len() as u64) + (fast_lineages.len() as u64)).into(),
                        );
                    }

                    // Fast lineages are now slow again
                    std::mem::swap(&mut slow_lineages, &mut fast_lineages);
                }

                Ok(())
            })?;

            // [Report all remaining events]
            proxy.finalise();

            Ok(())
        })
    })?;

    // Safety: Max of NonNegativeF64 values from the GPU
    let total_time_max =
        unsafe { NonNegativeF64::new_unchecked(f64::from_bits(total_time_max.into_inner())) };
    let total_steps_sum = total_steps_sum.into_inner();

    local_partition.report_progress_sync(slow_lineages.len() as u64);

    let status = Status::paused(local_partition.reduce_vote_any(!slow_lineages.is_empty()));
    let local_time = total_time_max;
    let local_steps = total_steps_sum;
    let lineages = slow_lineages.into_iter().map(|(lineage, _)| lineage);

    // Note: The simulation requires no mutation, since all components are
    //       either immutable or have singular swap states, and the list
    //       of all lineages (which does change) is returned separately

    Ok((status, local_time, local_steps, lineages))
}
