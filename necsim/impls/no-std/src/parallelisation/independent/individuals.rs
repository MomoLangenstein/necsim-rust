use alloc::collections::VecDeque;
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
    lineage::Lineage,
    reporter::Reporter,
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

use super::{reporter::IgnoreProgressReporterProxy, DedupCache};

#[expect(clippy::type_complexity)]
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

    let mut total_steps = 0_u64;
    let mut max_time = NonNegativeF64::zero();

    while !lineages.is_empty()
        || simulation.active_lineage_sampler().number_active_lineages() > 0
        || proxy.local_partition().wait_for_termination().is_continue()
    {
        proxy.report_total_progress(
            (Wrapping(lineages.len() as u64) + simulation.get_balanced_remaining_work()).0,
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
    }

    proxy.local_partition().report_progress_sync(0_u64);

    let local_time = max_time;
    let local_steps = total_steps;

    (Status::Done, local_time, local_steps, lineages)
}
