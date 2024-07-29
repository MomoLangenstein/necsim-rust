use core::marker::PhantomData;

use necsim_core::{
    cogs::{
        coalescence_sampler::CoalescenceRngSample, event_sampler::EventHandler, Backup,
        CoalescenceSampler, DispersalSampler, EmigrationExit, EventSampler,
        GloballyCoherentLineageStore, Habitat, MathsCore, RngCore, SpeciationProbability,
        TurnoverRate,
    },
    event::{DispersalEvent, SpeciationEvent},
    landscape::Location,
    lineage::Lineage,
    simulation::partial::event_sampler::PartialSimulation,
};
use necsim_core_bond::{NonNegativeF64, PositiveF64};

use super::GillespieEventSampler;

#[expect(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct UnconditionalGillespieEventSampler<
    M: MathsCore,
    H: Habitat<M>,
    G: RngCore<M>,
    S: GloballyCoherentLineageStore<M, H>,
    X: EmigrationExit<M, H, G, S>,
    D: DispersalSampler<M, H, G>,
    C: CoalescenceSampler<M, H, S>,
    T: TurnoverRate<M, H>,
    N: SpeciationProbability<M, H>,
> {
    #[expect(clippy::type_complexity)]
    marker: PhantomData<(M, H, G, S, X, D, C, T, N)>,
}

impl<
        M: MathsCore,
        H: Habitat<M>,
        G: RngCore<M>,
        S: GloballyCoherentLineageStore<M, H>,
        X: EmigrationExit<M, H, G, S>,
        D: DispersalSampler<M, H, G>,
        C: CoalescenceSampler<M, H, S>,
        T: TurnoverRate<M, H>,
        N: SpeciationProbability<M, H>,
    > Default for UnconditionalGillespieEventSampler<M, H, G, S, X, D, C, T, N>
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
        G: RngCore<M>,
        S: GloballyCoherentLineageStore<M, H>,
        X: EmigrationExit<M, H, G, S>,
        D: DispersalSampler<M, H, G>,
        C: CoalescenceSampler<M, H, S>,
        T: TurnoverRate<M, H>,
        N: SpeciationProbability<M, H>,
    > Backup for UnconditionalGillespieEventSampler<M, H, G, S, X, D, C, T, N>
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
        G: RngCore<M>,
        S: GloballyCoherentLineageStore<M, H>,
        X: EmigrationExit<M, H, G, S>,
        D: DispersalSampler<M, H, G>,
        C: CoalescenceSampler<M, H, S>,
        T: TurnoverRate<M, H>,
        N: SpeciationProbability<M, H>,
    > EventSampler<M, H, G, S, X, D, C, T, N>
    for UnconditionalGillespieEventSampler<M, H, G, S, X, D, C, T, N>
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
        use necsim_core::cogs::RngSampler;

        if rng.sample_event(
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

#[contract_trait]
impl<
        M: MathsCore,
        H: Habitat<M>,
        G: RngCore<M>,
        S: GloballyCoherentLineageStore<M, H>,
        X: EmigrationExit<M, H, G, S>,
        D: DispersalSampler<M, H, G>,
        C: CoalescenceSampler<M, H, S>,
        T: TurnoverRate<M, H>,
        N: SpeciationProbability<M, H>,
    > GillespieEventSampler<M, H, G, S, X, D, C, T, N>
    for UnconditionalGillespieEventSampler<M, H, G, S, X, D, C, T, N>
{
    #[must_use]
    fn get_event_rate_at_location(
        &self,
        location: &Location,
        habitat: &H,
        lineage_store: &S,
        _dispersal_sampler: &D,
        _coalescence_sampler: &C,
        turnover_rate: &T,
        _speciation_probability: &N,
    ) -> NonNegativeF64 {
        let population = NonNegativeF64::from(
            lineage_store
                .get_local_lineage_references_at_location_unordered(location, habitat)
                .len(),
        );

        population * turnover_rate.get_turnover_rate_at_location(location, habitat)
    }
}
