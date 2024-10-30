use alloc::{collections::VecDeque, vec::Vec};
use core::{
    iter::FromIterator,
    num::{NonZeroU64, Wrapping},
    ops::ControlFlow,
};

use necsim_core_bond::NonNegativeF64;

use necsim_core::{
    cogs::{
        DispersalSampler, Habitat, MathsCore, PrimeableRng, SpeciationProbability, TurnoverRate,
    },
    event::DispersalEvent,
    landscape::IndexedLocation,
    lineage::{Lineage, LineageInteraction, MigratingLineage},
    reporter::Reporter,
    simulation::Simulation,
};

use necsim_partitioning_core::{LocalPartition, MigrationMode};

use crate::{
    cogs::{
        active_lineage_sampler::singular::SingularActiveLineageSampler,
        coalescence_sampler::independent::IndependentCoalescenceSampler,
        emigration_exit::independent::{choice::EmigrationChoice, IndependentEmigrationExit},
        event_sampler::{
            independent::IndependentEventSampler, tracking::MinSpeciationTrackingEventSampler,
        },
        immigration_entry::never::NeverImmigrationEntry,
        lineage_store::independent::IndependentLineageStore,
    },
    decomposition::Decomposition,
    parallelisation::Status,
};

use super::{reporter::IgnoreProgressReporterProxy, DedupCache};

#[expect(clippy::type_complexity)]
pub fn simulate<
    'p,
    M: MathsCore,
    H: Habitat<M>,
    C: Decomposition<M, H>,
    E: EmigrationChoice<M, H>,
    G: PrimeableRng<M>,
    D: DispersalSampler<M, H, G>,
    T: TurnoverRate<M, H>,
    N: SpeciationProbability<M, H>,
    A: SingularActiveLineageSampler<
        M,
        H,
        G,
        IndependentLineageStore<M, H>,
        IndependentEmigrationExit<M, H, C, E>,
        D,
        IndependentCoalescenceSampler<M, H>,
        T,
        N,
        IndependentEventSampler<M, H, G, IndependentEmigrationExit<M, H, C, E>, D, T, N>,
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
        IndependentEmigrationExit<M, H, C, E>,
        D,
        IndependentCoalescenceSampler<M, H>,
        T,
        N,
        IndependentEventSampler<M, H, G, IndependentEmigrationExit<M, H, C, E>, D, T, N>,
        NeverImmigrationEntry,
        A,
    >,
    lineages: L,
    dedup_cache: DedupCache,
    step_slice: NonZeroU64,
    local_partition: &mut P,
) -> (
    Status,
    NonNegativeF64,
    u64,
    impl IntoIterator<Item = Lineage>,
) {
    let mut lineages = VecDeque::from_iter(lineages);
    let mut proxy = IgnoreProgressReporterProxy::from(local_partition);
    let mut min_spec_samples = dedup_cache.construct(lineages.len());

    // Ensure that the progress bar starts with the expected target
    proxy.local_partition().report_progress_sync(
        (Wrapping(lineages.len() as u64) + simulation.get_balanced_remaining_work()).0,
    );

    let mut immigration_events = Vec::new();

    let mut total_steps = 0_u64;
    let mut max_time = NonNegativeF64::zero();

    let mut local_immigration_count = Wrapping(0_u64);

    while !lineages.is_empty()
        || simulation.active_lineage_sampler().number_active_lineages() > 0
        || !simulation.emigration_exit().is_empty()
        || proxy.local_partition().wait_for_termination().is_continue()
    {
        proxy.report_total_progress(
            (Wrapping(lineages.len() as u64) + simulation.get_balanced_remaining_work()
                - local_immigration_count)
                .0,
        );

        let previous_task = simulation
            .active_lineage_sampler_mut()
            .replace_active_lineage(lineages.pop_front());

        let previous_speciation_sample =
            simulation.event_sampler_mut().replace_min_speciation(None);

        if let Some(previous_speciation_sample) = previous_speciation_sample {
            if min_spec_samples.insert(previous_speciation_sample) {
                if let Some(previous_task) = previous_task {
                    lineages.push_back(previous_task);
                }
            }
        }

        // Note: Immigration consistency
        //  If a jumps to b at the same time as b jumps to a,
        //  no coalescence occurs as coalescence would only be
        //  detected at the next shared duplicate event

        let (new_time, new_steps) = simulation.simulate_incremental_early_stop(
            |_, steps, _, _| {
                if steps >= step_slice.get() {
                    ControlFlow::Break(())
                } else {
                    ControlFlow::Continue(())
                }
            },
            &mut proxy,
        );

        total_steps += new_steps;
        max_time = max_time.max(new_time);

        // Force migration when no local tasks remain
        let migration_mode = if lineages.is_empty() {
            MigrationMode::Force
        } else {
            MigrationMode::Default
        };

        // Send off the possible emigrant and recieve immigrants
        let immigrants = match simulation.emigration_exit_mut().take() {
            Some(emigrant) => proxy.local_partition().migrate_individuals(
                &mut core::iter::once(emigrant),
                migration_mode,
                migration_mode,
            ),
            None => proxy.local_partition().migrate_individuals(
                &mut core::iter::empty(),
                migration_mode,
                migration_mode,
            ),
        };

        // Create local Lineages from the MigrantLineags
        for immigrant in immigrants {
            let MigratingLineage {
                global_reference,
                dispersal_origin,
                dispersal_target,
                prior_time,
                event_time,
                coalescence_rng_sample,
                tie_breaker: _,
            } = immigrant;

            // Finish sampling the dispersal of the immigrating individual
            let target_index = coalescence_rng_sample.sample_coalescence_index::<M>(
                simulation
                    .habitat()
                    .get_habitat_at_location(&dispersal_target),
            );
            let dispersal_target = IndexedLocation::new(dispersal_target, target_index);

            // Cache the immigration event
            immigration_events.push(DispersalEvent {
                origin: dispersal_origin,
                prior_time,
                event_time,
                global_lineage_reference: global_reference.clone(),
                target: dispersal_target.clone(),
                interaction: LineageInteraction::Maybe,
            });

            // Since the simulation has no internal immigration,
            //  we have to manually keep score of the immigrations
            local_immigration_count += Wrapping(1_u64);

            // Append the new Lineage to the local task list
            lineages.push_back(Lineage {
                global_reference,
                indexed_location: dispersal_target,
                last_event_time: event_time.into(),
            });
        }

        // Report any immigration events
        while let Some(immigration_event) = immigration_events.pop() {
            proxy.report_dispersal(&immigration_event.into());
        }
    }

    proxy.local_partition().report_progress_sync(0_u64);

    let local_time = max_time;
    let local_steps = total_steps;

    (Status::Done, local_time, local_steps, lineages)
}
