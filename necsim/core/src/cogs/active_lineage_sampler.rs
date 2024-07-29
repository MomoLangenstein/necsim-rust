use core::ops::ControlFlow;

use necsim_core_bond::{NonNegativeF64, PositiveF64};

use super::{
    CoalescenceSampler, DispersalSampler, EmigrationExit, EventSampler, Habitat, ImmigrationEntry,
    LineageStore, MathsCore, RngCore, SpeciationProbability, TurnoverRate,
};

use crate::{lineage::Lineage, simulation::partial::active_lineage_sampler::PartialSimulation};

#[contract_trait]
pub trait ActiveLineageSampler<
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
>: crate::cogs::Backup + core::fmt::Debug
{
    type LineageIterator<'a>: Iterator<Item = &'a Lineage>
    where
        M: 'a,
        H: 'a,
        S: 'a,
        Self: 'a;

    #[must_use]
    fn number_active_lineages(&self) -> usize;

    #[must_use]
    fn iter_active_lineages_ordered<'a>(
        &'a self,
        habitat: &'a H,
        lineage_store: &'a S,
    ) -> Self::LineageIterator<'a>;

    #[must_use]
    fn get_last_event_time(&self) -> NonNegativeF64;

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
    #[debug_ensures(
        ret.is_none() || ret.as_ref().unwrap().1 > old(self.get_last_event_time()),
        "event occurs later than last event time"
    )]
    #[debug_ensures(if let Some((ref _lineage, event_time)) = ret {
        self.get_last_event_time() == event_time
    } else { true }, "updates the time of the last event")]
    fn pop_active_lineage_and_event_time<P: FnOnce(PositiveF64) -> ControlFlow<(), ()>>(
        &mut self,
        simulation: &mut PartialSimulation<M, H, G, S, X, D, C, T, N, E>,
        rng: &mut G,
        early_peek_stop: P,
    ) -> Option<(Lineage, PositiveF64)>;

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
    );
}
