use core::{cmp::Ordering, ops::ControlFlow};

mod backup;
mod builder;
mod process;

pub mod partial;

use core::num::Wrapping;

use crate::{
    cogs::{
        ActiveLineageSampler, CoalescenceSampler, DispersalSampler, EmigrationExit, EventSampler,
        Habitat, ImmigrationEntry, LineageStore, MathsCore, RngCore, SpeciationProbability,
        TurnoverRate,
    },
    lineage::TieBreaker,
    reporter::Reporter,
};

#[allow(clippy::module_name_repetitions)] // FIXME: use expect
pub use builder::{Simulation, SimulationBuilder};
use necsim_core_bond::{NonNegativeF64, PositiveF64};

impl<
        M: MathsCore,
        H: Habitat<M>,
        G: RngCore<M>,
        S: LineageStore<M, H>,
        X: EmigrationExit<M, H, G, S>,
        D: DispersalSampler<M, H, G>,
        C: CoalescenceSampler<M, H, S>,
        T: TurnoverRate<M, H>,
        N: SpeciationProbability<M, H>,
        E: EventSampler<M, H, G, S, X, D, C, T, N>,
        I: ImmigrationEntry<M>,
        A: ActiveLineageSampler<M, H, G, S, X, D, C, T, N, E, I>,
    > Simulation<M, H, G, S, X, D, C, T, N, E, I, A>
{
    pub fn is_done(&self) -> bool {
        self.active_lineage_sampler.number_active_lineages() == 0
            && self.immigration_entry.peek_next_immigration().is_none()
    }

    pub fn get_balanced_remaining_work(&self) -> Wrapping<u64> {
        let local_remaining =
            Wrapping(self.active_lineage_sampler().number_active_lineages() as u64);

        local_remaining + self.migration_balance
    }

    #[inline]
    pub fn simulate_incremental_early_stop<
        F: FnMut(&Self, u64, PositiveF64, &P) -> ControlFlow<(), ()>,
        P: Reporter,
    >(
        &mut self,
        mut early_stop: F,
        reporter: &mut P,
    ) -> (NonNegativeF64, u64) {
        let mut steps = 0_u64;

        loop {
            reporter.report_progress(&self.get_balanced_remaining_work().0.into());

            let next_immigration_time_tie = self
                .immigration_entry
                .peek_next_immigration()
                .map(|lineage| (lineage.event_time, lineage.tie_breaker));

            let self_ptr = self as *const Self;
            let reporter_ptr = reporter as *const P;

            let old_rng = unsafe { self.rng.backup_unchecked() };
            let mut early_stop_flow = ControlFlow::Continue(());

            let early_peek_stop = |next_event_time| {
                // Safety: We are only passing in an immutable reference
                early_stop_flow =
                    early_stop(unsafe { &*self_ptr }, steps, next_event_time, unsafe {
                        &*reporter_ptr
                    });

                if early_stop_flow.is_break() {
                    return ControlFlow::Break(());
                }

                if let Some((next_immigration_time, next_immigration_tie_breaker)) =
                    next_immigration_time_tie
                {
                    return match (
                        next_immigration_time.cmp(&next_event_time),
                        next_immigration_tie_breaker,
                    ) {
                        (Ordering::Less, _) | (Ordering::Equal, TieBreaker::PreferImmigrant) => {
                            ControlFlow::Break(())
                        },
                        (Ordering::Greater, _) | (Ordering::Equal, TieBreaker::PreferLocal) => {
                            ControlFlow::Continue(())
                        },
                    };
                }

                ControlFlow::Continue(())
            };

            if self
                .simulate_and_report_local_step_or_early_stop_or_finish(reporter, early_peek_stop)
                .is_break()
            {
                if early_stop_flow.is_break() {
                    // Early stop, reset the RNG to before the event time peek to eliminate side
                    //  effects
                    break self.rng = old_rng;
                }

                // Check for migration as the alternative to finishing the simulation
                if let Some(migrating_lineage) =
                    self.immigration_entry_mut().next_optional_immigration()
                {
                    self.simulate_and_report_immigration_step(reporter, migrating_lineage);
                } else {
                    // Neither a local nor immigration event -> finish the simulation
                    break;
                }
            }

            steps += 1;
        }

        reporter.report_progress(&self.get_balanced_remaining_work().0.into());

        (self.active_lineage_sampler.get_last_event_time(), steps)
    }

    #[inline]
    pub fn simulate<P: Reporter>(mut self, reporter: &mut P) -> (NonNegativeF64, u64) {
        self.simulate_incremental_early_stop(|_, _, _, _| ControlFlow::Continue(()), reporter)
    }
}
