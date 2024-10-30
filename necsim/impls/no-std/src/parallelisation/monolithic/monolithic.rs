use core::ops::ControlFlow;

use necsim_core::{
    cogs::{
        ActiveLineageSampler, CoalescenceSampler, DispersalSampler, EventSampler, Habitat,
        LocallyCoherentLineageStore, MathsCore, RngCore, SpeciationProbability, TurnoverRate,
    },
    reporter::Reporter,
    simulation::Simulation,
};
use necsim_core_bond::NonNegativeF64;

use necsim_partitioning_core::LocalPartition;

use crate::{
    cogs::{
        emigration_exit::never::NeverEmigrationExit,
        immigration_entry::never::NeverImmigrationEntry,
    },
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
    E: EventSampler<M, H, G, S, NeverEmigrationExit, D, C, T, N>,
    A: ActiveLineageSampler<M, H, G, S, NeverEmigrationExit, D, C, T, N, E, NeverImmigrationEntry>,
    P: Reporter,
    L: LocalPartition<'p, P>,
>(
    simulation: &mut Simulation<
        M,
        H,
        G,
        S,
        NeverEmigrationExit,
        D,
        C,
        T,
        N,
        E,
        NeverImmigrationEntry,
        A,
    >,
    pause_before: Option<NonNegativeF64>,
    local_partition: &mut L,
) -> (Status, NonNegativeF64, u64) {
    // Ensure that the progress bar starts with the expected target
    local_partition.report_progress_sync(simulation.get_balanced_remaining_work().0);

    // Note: Immigration consistency
    //  If the underlying ActiveLineageSampler picks
    //  two individuals a and b s.t. a wants to jump
    //  to b at the same time as b wants to jump to
    //  a, it has to make a deterministic choice
    //  between the two for which one goes first and
    //  coalesces into the other. The non-selected
    //  individual's wish to jump is invalidated as
    //  there are now different circumstances and
    //  since the next event must occur at a monoton-
    //  ically later time

    let (time, steps) = simulation.simulate_incremental_early_stop(
        |_, _, next_event_time, _| {
            pause_before.map_or(ControlFlow::Continue(()), |pause_before| {
                if next_event_time >= pause_before {
                    ControlFlow::Break(())
                } else {
                    ControlFlow::Continue(())
                }
            })
        },
        local_partition.get_reporter(),
    );

    local_partition.report_progress_sync(simulation.get_balanced_remaining_work().0);

    let status = Status::paused(
        local_partition
            .reduce_vote_any(simulation.active_lineage_sampler().number_active_lineages() > 0),
    );
    let local_time = time;
    let local_steps = steps;

    (status, local_time, local_steps)
}
