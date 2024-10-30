use necsim_core_bond::PositiveF64;

use super::{
    CoalescenceSampler, DispersalSampler, EmigrationExit, Habitat, LineageStore, MathsCore,
    RngCore, SpeciationProbability, TurnoverRate,
};
use crate::{
    event::{DispersalEvent, SpeciationEvent},
    lineage::Lineage,
    simulation::partial::event_sampler::PartialSimulation,
};

pub struct EventHandler<S, D, E> {
    pub speciation: S,
    pub dispersal: D,
    pub emigration: E,
}

#[contract_trait]
pub trait EventSampler<
    M: MathsCore,
    H: Habitat<M>,
    G: RngCore<M>,
    S: LineageStore<M, H>,
    X: EmigrationExit<M, H, G, S>,
    D: DispersalSampler<M, H, G>,
    C: CoalescenceSampler<M, H, S>,
    T: TurnoverRate<M, H>,
    N: SpeciationProbability<M, H>,
>: crate::cogs::Backup + core::fmt::Debug
{
    #[must_use]
    fn sample_event_for_lineage_at_event_time_or_emigrate<
        Q,
        Aux,
        FS: FnOnce(SpeciationEvent, Aux) -> Q,
        FD: FnOnce(DispersalEvent, Aux) -> Q,
        FE: FnOnce(Aux) -> Q,
    >(
        &mut self,
        lineage: Lineage,
        event_time: PositiveF64,
        simulation: &mut PartialSimulation<M, H, G, S, X, D, C, T, N>,
        rng: &mut G,
        handler: EventHandler<FS, FD, FE>,
        auxiliary: Aux,
    ) -> Q;
}
