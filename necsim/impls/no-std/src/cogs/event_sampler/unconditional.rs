use core::marker::PhantomData;

use necsim_core::{
    cogs::{
        coalescence_sampler::CoalescenceRngSample,
        distribution::{Bernoulli, UniformClosedOpenUnit},
        event_sampler::EventHandler,
        Backup, CoalescenceSampler, DispersalSampler, Distribution, EmigrationExit, EventSampler,
        Habitat, LocallyCoherentLineageStore, MathsCore, Rng, Samples, SpeciationProbability,
        TurnoverRate,
    },
    event::{DispersalEvent, SpeciationEvent},
    lineage::Lineage,
    simulation::partial::event_sampler::PartialSimulation,
};
use necsim_core_bond::PositiveF64;

#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct UnconditionalEventSampler<
    M: MathsCore,
    H: Habitat<M>,
    G: Rng<M> + Samples<M, Bernoulli> + Samples<M, UniformClosedOpenUnit>,
    S: LocallyCoherentLineageStore<M, H>,
    X: EmigrationExit<M, H, G, S>,
    D: DispersalSampler<M, H, G>,
    C: CoalescenceSampler<M, H, S>,
    T: TurnoverRate<M, H>,
    N: SpeciationProbability<M, H>,
> {
    #[allow(clippy::type_complexity)]
    marker: PhantomData<(M, H, G, S, X, D, C, T, N)>,
}

impl<
        M: MathsCore,
        H: Habitat<M>,
        G: Rng<M> + Samples<M, Bernoulli> + Samples<M, UniformClosedOpenUnit>,
        S: LocallyCoherentLineageStore<M, H>,
        X: EmigrationExit<M, H, G, S>,
        D: DispersalSampler<M, H, G>,
        C: CoalescenceSampler<M, H, S>,
        T: TurnoverRate<M, H>,
        N: SpeciationProbability<M, H>,
    > Default for UnconditionalEventSampler<M, H, G, S, X, D, C, T, N>
{
    fn default() -> Self {
        Self {
            marker: PhantomData::<(M, H, G, S, X, D, C, T, N)>,
        }
    }
}

#[contract_trait]
impl<
        M: MathsCore,
        H: Habitat<M>,
        G: Rng<M> + Samples<M, Bernoulli> + Samples<M, UniformClosedOpenUnit>,
        S: LocallyCoherentLineageStore<M, H>,
        X: EmigrationExit<M, H, G, S>,
        D: DispersalSampler<M, H, G>,
        C: CoalescenceSampler<M, H, S>,
        T: TurnoverRate<M, H>,
        N: SpeciationProbability<M, H>,
    > Backup for UnconditionalEventSampler<M, H, G, S, X, D, C, T, N>
{
    unsafe fn backup_unchecked(&self) -> Self {
        Self {
            marker: PhantomData::<(M, H, G, S, X, D, C, T, N)>,
        }
    }
}

#[contract_trait]
impl<
        M: MathsCore,
        H: Habitat<M>,
        G: Rng<M> + Samples<M, Bernoulli> + Samples<M, UniformClosedOpenUnit>,
        S: LocallyCoherentLineageStore<M, H>,
        X: EmigrationExit<M, H, G, S>,
        D: DispersalSampler<M, H, G>,
        C: CoalescenceSampler<M, H, S>,
        T: TurnoverRate<M, H>,
        N: SpeciationProbability<M, H>,
    > EventSampler<M, H, G, S, X, D, C, T, N>
    for UnconditionalEventSampler<M, H, G, S, X, D, C, T, N>
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
        Lineage {
            global_reference,
            last_event_time: prior_time,
            indexed_location: dispersal_origin,
        }: Lineage,
        event_time: PositiveF64,
        simulation: &mut PartialSimulation<M, H, G, S, X, D, C, T, N>,
        rng: &mut G,
        EventHandler {
            speciation,
            dispersal,
            emigration,
        }: EventHandler<FS, FD, FE>,
        auxiliary: Aux,
    ) -> Q {
        if Bernoulli::sample_with(
            rng,
            simulation
                .speciation_probability
                .get_speciation_probability_at_location(
                    dispersal_origin.location(),
                    &simulation.habitat,
                ),
        ) {
            speciation(
                SpeciationEvent {
                    origin: dispersal_origin,
                    prior_time,
                    event_time,
                    global_lineage_reference: global_reference,
                },
                auxiliary,
            )
        } else {
            let dispersal_target = simulation.dispersal_sampler.sample_dispersal_from_location(
                dispersal_origin.location(),
                &simulation.habitat,
                rng,
            );

            // Check for emigration and return None iff lineage emigrated
            if let Some((
                global_reference,
                dispersal_origin,
                dispersal_target,
                prior_time,
                event_time,
            )) = simulation.with_mut_split_emigration_exit(|emigration_exit, simulation| {
                emigration_exit.optionally_emigrate(
                    global_reference,
                    dispersal_origin,
                    dispersal_target,
                    prior_time,
                    event_time,
                    simulation,
                    rng,
                )
            }) {
                let (dispersal_target, interaction) = simulation
                    .coalescence_sampler
                    .sample_interaction_at_location(
                        dispersal_target,
                        &simulation.habitat,
                        &simulation.lineage_store,
                        CoalescenceRngSample::new(rng),
                    );

                dispersal(
                    DispersalEvent {
                        origin: dispersal_origin,
                        prior_time,
                        event_time,
                        global_lineage_reference: global_reference,
                        target: dispersal_target,
                        interaction,
                    },
                    auxiliary,
                )
            } else {
                emigration(auxiliary)
            }
        }
    }
}
