use core::ops::ControlFlow;

use necsim_core::{
    cogs::{
        ActiveLineageSampler, CoalescenceSampler, DispersalSampler, EventSampler, Habitat,
        LocallyCoherentLineageStore, MathsCore, RngCore, SpeciationProbability, TurnoverRate,
    },
    reporter::{NullReporter, Reporter},
    simulation::Simulation,
};
use necsim_core_bond::{NonNegativeF64, PositiveF64};

use necsim_partitioning_core::{LocalPartition, MigrationMode};

use crate::{
    cogs::{
        emigration_exit::domain::DomainEmigrationExit,
        immigration_entry::buffered::BufferedImmigrationEntry,
    },
    decomposition::Decomposition,
    parallelisation::Status,
};

#[expect(clippy::type_complexity)]
pub fn simulate<
    'p,
    M: MathsCore,
    H: Habitat<M>,
    G: RngCore<M>,
    S: LocallyCoherentLineageStore<M, H>,
    D: DispersalSampler<M, H, G>,
    C: CoalescenceSampler<M, H, S>,
    T: TurnoverRate<M, H>,
    N: SpeciationProbability<M, H>,
    O: Decomposition<M, H>,
    E: EventSampler<M, H, G, S, DomainEmigrationExit<M, H, O>, D, C, T, N>,
    A: ActiveLineageSampler<
        M,
        H,
        G,
        S,
        DomainEmigrationExit<M, H, O>,
        D,
        C,
        T,
        N,
        E,
        BufferedImmigrationEntry,
    >,
    P: Reporter,
    L: LocalPartition<'p, P>,
>(
    simulation: &mut Simulation<
        M,
        H,
        G,
        S,
        DomainEmigrationExit<M, H, O>,
        D,
        C,
        T,
        N,
        E,
        BufferedImmigrationEntry,
        A,
    >,
    local_partition: &mut L,
) -> (Status, NonNegativeF64, u64) {
    // Ensure that the progress bar starts with the expected target
    local_partition.report_progress_sync(simulation.get_balanced_remaining_work().0);

    let mut total_steps = 0_u64;

    while local_partition.reduce_vote_any(!simulation.is_done()) {
        let mut next_local_time = None;

        // Simulate for zero-steps (immediate early stop) without side effects
        //  to peek the next local event time
        simulation.simulate_incremental_early_stop(
            |_, _, next_event_time, _| {
                next_local_time = Some(next_event_time);

                ControlFlow::Break(())
            },
            &mut NullReporter,
        );

        // Get the next local event time or +inf
        //  (we already know at least one partition has some next event time)
        let next_local_time = next_local_time.unwrap_or_else(PositiveF64::infinity);

        // Note: Immigration consistency
        //  If a wants to jump to b at the same time as b
        //  wants to jumps to a, one of the two is selected
        //  to go first deterministically based on which
        //  partition they belong to, and coalescence occurs.
        //  The non-selected individual's wish to jump is
        //  invalidated as there are now different circum-
        //  stances and since the next event must occur at
        //  a monotonically later time

        // The partition with the next event gets to simulate just the next step
        if let Ok(next_global_time) = local_partition.reduce_vote_min_time(next_local_time) {
            let (_, new_steps) = simulation.simulate_incremental_early_stop(
                |_, _, next_event_time, _| {
                    if next_event_time > next_global_time {
                        ControlFlow::Break(())
                    } else {
                        ControlFlow::Continue(())
                    }
                },
                local_partition.get_reporter(),
            );

            total_steps += new_steps;

            // Send off any emigration that might have occurred
            let (emigration_exit, immigration_entry) = simulation.migration_portals_mut();
            immigration_entry.extend(local_partition.migrate_individuals(
                emigration_exit,
                MigrationMode::Default,
                MigrationMode::Default,
            ));
        }

        // Synchronise after performing any inter-partition migration
        while local_partition.wait_for_termination().is_continue() {
            simulation
                .immigration_entry_mut()
                .extend(local_partition.migrate_individuals(
                    &mut core::iter::empty(),
                    MigrationMode::Force,
                    MigrationMode::Force,
                ));
        }
    }

    local_partition.report_progress_sync(0_u64);

    let local_time = simulation.active_lineage_sampler().get_last_event_time();
    let local_steps = total_steps;

    (Status::Done, local_time, local_steps)
}
