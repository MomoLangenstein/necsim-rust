use core::marker::PhantomData;

use necsim_core::{
    cogs::{
        coalescence_sampler::CoalescenceRngSample, event_sampler::EventHandler, Backup,
        CoalescenceSampler, EmigrationExit, EventSampler, GloballyCoherentLineageStore, Habitat,
        MathsCore, RngCore, RngSampler, SeparableDispersalSampler, SpeciationProbability,
        TurnoverRate,
    },
    event::{DispersalEvent, SpeciationEvent},
    landscape::Location,
    lineage::{Lineage, LineageInteraction},
    simulation::partial::event_sampler::PartialSimulation,
};
use necsim_core_bond::{NonNegativeF64, PositiveF64};

use crate::cogs::{
    coalescence_sampler::conditional::ConditionalCoalescenceSampler,
    event_sampler::gillespie::GillespieEventSampler,
};

mod probability;

use probability::ProbabilityAtLocation;

#[expect(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct ConditionalGillespieEventSampler<
    M: MathsCore,
    H: Habitat<M>,
    G: RngCore<M>,
    S: GloballyCoherentLineageStore<M, H>,
    X: EmigrationExit<M, H, G, S>,
    D: SeparableDispersalSampler<M, H, G>,
    T: TurnoverRate<M, H>,
    N: SpeciationProbability<M, H>,
> {
    #[expect(clippy::type_complexity)]
    marker: PhantomData<(M, H, G, S, X, D, T, N)>,
}

impl<
        M: MathsCore,
        H: Habitat<M>,
        G: RngCore<M>,
        S: GloballyCoherentLineageStore<M, H>,
        X: EmigrationExit<M, H, G, S>,
        D: SeparableDispersalSampler<M, H, G>,
        T: TurnoverRate<M, H>,
        N: SpeciationProbability<M, H>,
    > Default for ConditionalGillespieEventSampler<M, H, G, S, X, D, T, N>
{
    fn default() -> Self {
        Self {
            marker: PhantomData::<(M, H, G, S, X, D, T, N)>,
        }
    }
}

#[contract_trait]
impl<
        M: MathsCore,
        H: Habitat<M>,
        G: RngCore<M>,
        S: GloballyCoherentLineageStore<M, H>,
        X: EmigrationExit<M, H, G, S>,
        D: SeparableDispersalSampler<M, H, G>,
        T: TurnoverRate<M, H>,
        N: SpeciationProbability<M, H>,
    > Backup for ConditionalGillespieEventSampler<M, H, G, S, X, D, T, N>
{
    unsafe fn backup_unchecked(&self) -> Self {
        Self {
            marker: PhantomData::<(M, H, G, S, X, D, T, N)>,
        }
    }
}

#[contract_trait]
impl<
        M: MathsCore,
        H: Habitat<M>,
        G: RngCore<M>,
        S: GloballyCoherentLineageStore<M, H>,
        X: EmigrationExit<M, H, G, S>,
        D: SeparableDispersalSampler<M, H, G>,
        T: TurnoverRate<M, H>,
        N: SpeciationProbability<M, H>,
    > EventSampler<M, H, G, S, X, D, ConditionalCoalescenceSampler<M, H, S>, T, N>
    for ConditionalGillespieEventSampler<M, H, G, S, X, D, T, N>
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
        simulation: &mut PartialSimulation<
            M,
            H,
            G,
            S,
            X,
            D,
            ConditionalCoalescenceSampler<M, H, S>,
            T,
            N,
        >,
        rng: &mut G,
        EventHandler {
            speciation,
            dispersal,
            emigration,
        }: EventHandler<FS, FD, FE>,
        auxiliary: Aux,
    ) -> Q {
        // The event is sampled after the active lineage has been removed from
        //  the lineage store, but it must be included in the calculation
        let probability_at_location = ProbabilityAtLocation::new(
            dispersal_origin.location(),
            &simulation.habitat,
            &simulation.lineage_store,
            &simulation.dispersal_sampler,
            &simulation.coalescence_sampler,
            &simulation.speciation_probability,
            false,
        );

        let event_sample = probability_at_location.total() * rng.sample_uniform_closed_open();

        if event_sample < probability_at_location.speciation() {
            // Speciation Event
            speciation(
                SpeciationEvent {
                    origin: dispersal_origin,
                    prior_time,
                    event_time,
                    global_lineage_reference: global_reference,
                },
                auxiliary,
            )
        } else if event_sample
            < (probability_at_location.speciation().get()
                + probability_at_location.out_dispersal().get())
        {
            // Out-Dispersal Event
            let dispersal_target = simulation
                .dispersal_sampler
                .sample_non_self_dispersal_from_location(
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
        } else {
            // In-Coalescence Event
            let (dispersal_target, coalescence) = simulation
                .coalescence_sampler
                .sample_coalescence_at_location(
                    dispersal_origin.location().clone(),
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
                    interaction: LineageInteraction::Coalescence(coalescence),
                },
                auxiliary,
            )
        }
    }
}

#[contract_trait]
impl<
        M: MathsCore,
        H: Habitat<M>,
        G: RngCore<M>,
        S: GloballyCoherentLineageStore<M, H>,
        X: EmigrationExit<M, H, G, S>,
        D: SeparableDispersalSampler<M, H, G>,
        T: TurnoverRate<M, H>,
        N: SpeciationProbability<M, H>,
    > GillespieEventSampler<M, H, G, S, X, D, ConditionalCoalescenceSampler<M, H, S>, T, N>
    for ConditionalGillespieEventSampler<M, H, G, S, X, D, T, N>
{
    #[must_use]
    fn get_event_rate_at_location(
        &self,
        location: &Location,
        habitat: &H,
        lineage_store: &S,
        dispersal_sampler: &D,
        coalescence_sampler: &ConditionalCoalescenceSampler<M, H, S>,
        turnover_rate: &T,
        speciation_probability: &N,
    ) -> NonNegativeF64 {
        // By PRE, all active lineages, including self, are in the lineage store
        let probability_at_location = ProbabilityAtLocation::new(
            location,
            habitat,
            lineage_store,
            dispersal_sampler,
            coalescence_sampler,
            speciation_probability,
            true,
        );

        let population = NonNegativeF64::from(
            lineage_store
                .get_local_lineage_references_at_location_unordered(location, habitat)
                .len(),
        );

        NonNegativeF64::from(probability_at_location.total())
            * population
            * turnover_rate.get_turnover_rate_at_location(location, habitat)
    }
}
