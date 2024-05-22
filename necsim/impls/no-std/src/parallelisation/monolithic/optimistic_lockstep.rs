use core::ops::ControlFlow;

use necsim_core::{
    cogs::{
        ActiveLineageSampler, Backup, CoalescenceSampler, DispersalSampler, EventSampler, Habitat,
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

#[allow(clippy::type_complexity)]
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

    let mut simulation_backup = simulation.backup();

    let mut total_steps = 0_u64;

    while local_partition.reduce_vote_any(!simulation.is_done()) {
        // Get the next local emigration event time or +inf
        //  (we already know at least one partition has some next event time)
        let next_local_emigration_time = {
            let (_, new_steps) = simulation.simulate_incremental_early_stop(
                |simulation, _, _, _| {
                    if simulation.emigration_exit().is_empty() {
                        ControlFlow::Continue(())
                    } else {
                        ControlFlow::Break(())
                    }
                },
                &mut NullReporter,
            );

            total_steps += new_steps;

            simulation
                .emigration_exit_mut()
                .min()
                .map(|(_, first_emigration)| first_emigration.event_time)
        }
        .unwrap_or_else(PositiveF64::infinity);

        // Note: Immigration consistency
        //  If a wants to jump to b at the same time as b
        //  wants to jumps to a, one of the two is selected
        //  to go first deterministically based on which
        //  partition they belong to, and coalescence occurs.
        //  The non-selected individual's wish to jump is
        //  invalidated as there are now different circum-
        //  stances and since the next event must occur at
        //  a monotonically later time

        // Roll back the simulation to the latest safe point
        *simulation = simulation_backup.resume();

        match local_partition.reduce_vote_min_time(next_local_emigration_time) {
            // The partition with the next emigration event gets to simulate until and inclusive
            //  that event
            Ok(next_global_time) => {
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
                for immigrant in local_partition.migrate_individuals(
                    simulation.emigration_exit_mut(),
                    MigrationMode::Default,
                    MigrationMode::Default,
                ) {
                    simulation.immigration_entry_mut().push(immigrant);
                }
            },
            // All other partitions get to simulate until just before this next migration event
            Err(next_global_time) => {
                let (_, new_steps) = simulation.simulate_incremental_early_stop(
                    |_, _, next_event_time, _| {
                        if next_event_time >= next_global_time {
                            ControlFlow::Break(())
                        } else {
                            ControlFlow::Continue(())
                        }
                    },
                    local_partition.get_reporter(),
                );

                total_steps += new_steps;
            },
        }

        // Synchronise after performing any inter-partition migration
        while local_partition.wait_for_termination().is_continue() {
            for immigrant in local_partition.migrate_individuals(
                &mut core::iter::empty(),
                MigrationMode::Force,
                MigrationMode::Force,
            ) {
                simulation.immigration_entry_mut().push(immigrant);
            }
        }

        // Advance the simulation backup to this new safe point
        simulation_backup = simulation.backup();
    }

    local_partition.report_progress_sync(0_u64);

    let (global_time, global_steps) = local_partition.reduce_global_time_steps(
        simulation.active_lineage_sampler().get_last_event_time(),
        total_steps,
    );

    (Status::Done, global_time, global_steps)
}
