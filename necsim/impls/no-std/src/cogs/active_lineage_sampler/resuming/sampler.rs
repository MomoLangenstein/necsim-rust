use core::ops::ControlFlow;

use necsim_core::{
    cogs::{
        ActiveLineageSampler, CoalescenceSampler, DispersalSampler, EmigrationExit, EventSampler,
        Habitat, ImmigrationEntry, LineageStore, MathsCore, RngCore, SpeciationProbability,
        TurnoverRate,
    },
    lineage::Lineage,
    simulation::partial::active_lineage_sampler::PartialSimulation,
};
use necsim_core_bond::{NonNegativeF64, PositiveF64};

use super::RestartFixUpActiveLineageSampler;

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
    > ActiveLineageSampler<M, H, G, S, X, D, C, T, N, E, I>
    for RestartFixUpActiveLineageSampler<M, H, G, S, X, D, C, T, N, E, I, A>
{
    type LineageIterator<'a> = impl Iterator<Item = &'a Lineage> where H: 'a, G: 'a, S: 'a, X: 'a, D: 'a, C: 'a, T: 'a, N: 'a, E: 'a, I: 'a, A: 'a;

    #[must_use]
    fn number_active_lineages(&self) -> usize {
        // All pre- and post-conditions are maintained
        self.__contracts_impl_number_active_lineages()
    }

    #[must_use]
    fn __contracts_impl_number_active_lineages(&self) -> usize {
        self.fixable_lineages.len() + self.inner.number_active_lineages()
    }

    #[must_use]
    fn iter_active_lineages_ordered<'a>(
        &'a self,
        habitat: &'a H,
        lineage_store: &'a S,
    ) -> Self::LineageIterator<'a> {
        // Note: inlined from __contracts_impl_iter_active_lineages_ordered
        self.fixable_lineages.iter().chain(
            self.inner
                .iter_active_lineages_ordered(habitat, lineage_store),
        )
    }

    #[must_use]
    fn __contracts_impl_iter_active_lineages_ordered<'a>(
        &'a self,
        habitat: &'a H,
        lineage_store: &'a S,
    ) -> Self::LineageIterator<'a> {
        self.fixable_lineages.iter().chain(
            self.inner
                .iter_active_lineages_ordered(habitat, lineage_store),
        )
    }

    #[must_use]
    fn get_last_event_time(&self) -> NonNegativeF64 {
        // All pre- and post-conditions are maintained
        self.__contracts_impl_get_last_event_time()
    }

    #[must_use]
    fn __contracts_impl_get_last_event_time(&self) -> NonNegativeF64 {
        if self.fixable_lineages.is_empty() {
            self.inner.get_last_event_time()
        } else {
            self.restart_time.into()
        }
    }

    #[must_use]
    #[debug_ensures(match ret {
        Some(_) => {
            self.number_active_lineages() ==
            old(self.number_active_lineages()) - 1
        },
        None => {
            self.number_active_lineages() ==
            old(self.number_active_lineages())
        },
    }, "removes an active lineage if `Some(_)` returned")]
    #[debug_ensures(
        old(self.number_active_lineages()) != 0 || ret.is_none(),
        "returns `None` if no lineages are left"
    )]
    #[debug_ensures(if let Some((ref _lineage, event_time)) = ret {
        self.get_last_event_time() == event_time
    } else { true }, "updates the time of the last event")]
    fn pop_active_lineage_and_event_time<F: FnOnce(PositiveF64) -> ControlFlow<(), ()>>(
        &mut self,
        simulation: &mut PartialSimulation<M, H, G, S, X, D, C, T, N, E>,
        rng: &mut G,
        early_peek_stop: F,
    ) -> Option<(Lineage, PositiveF64)> {
        // Explicitly circumvent the postcondition that the next event time is
        //  after the previous last event time
        self.__contracts_impl_pop_active_lineage_and_event_time(simulation, rng, early_peek_stop)
    }

    #[must_use]
    fn __contracts_impl_pop_active_lineage_and_event_time<
        F: FnOnce(PositiveF64) -> ControlFlow<(), ()>,
    >(
        &mut self,
        simulation: &mut PartialSimulation<M, H, G, S, X, D, C, T, N, E>,
        rng: &mut G,
        early_peek_stop: F,
    ) -> Option<(Lineage, PositiveF64)> {
        if self.fixable_lineages.is_empty() {
            return self
                .inner
                .pop_active_lineage_and_event_time(simulation, rng, early_peek_stop);
        }

        if early_peek_stop(self.restart_time).is_break() {
            return None;
        }

        // Safety: We just checked that there is at least one fixable lineage
        let fixable_lineage = unsafe { self.fixable_lineages.pop().unwrap_unchecked() };

        Some((fixable_lineage, self.restart_time))
    }

    #[allow(clippy::no_effect_underscore_binding)]
    #[debug_ensures(
        self.number_active_lineages() == old(self.number_active_lineages()) + 1,
        "adds an active lineage"
    )]
    #[debug_ensures(
        self.get_last_event_time() == old(lineage.last_event_time),
        "updates the time of the last event"
    )]
    fn push_active_lineage(
        &mut self,
        lineage: Lineage,
        simulation: &mut PartialSimulation<M, H, G, S, X, D, C, T, N, E>,
        rng: &mut G,
    ) {
        // All pre- and post-conditions are maintained
        self.__contracts_impl_push_active_lineage(lineage, simulation, rng);
    }

    fn __contracts_impl_push_active_lineage(
        &mut self,
        lineage: Lineage,
        simulation: &mut PartialSimulation<M, H, G, S, X, D, C, T, N, E>,
        rng: &mut G,
    ) {
        self.inner.push_active_lineage(lineage, simulation, rng);
    }
}
