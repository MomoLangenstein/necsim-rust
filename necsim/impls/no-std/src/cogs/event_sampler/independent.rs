use core::marker::PhantomData;

use necsim_core::{
    cogs::{
        coalescence_sampler::CoalescenceRngSample, event_sampler::EventHandler, Backup,
        CoalescenceSampler, DispersalSampler, EmigrationExit, EventSampler, Habitat, MathsCore,
        RngCore, SpeciationProbability, TurnoverRate,
    },
    event::{DispersalEvent, SpeciationEvent},
    lineage::Lineage,
    simulation::partial::event_sampler::PartialSimulation,
};
use necsim_core_bond::PositiveF64;

use crate::cogs::{
    coalescence_sampler::independent::IndependentCoalescenceSampler,
    lineage_store::independent::IndependentLineageStore,
};

use super::tracking::{MinSpeciationTrackingEventSampler, SpeciationSample};

#[expect(clippy::module_name_repetitions)]
#[derive(Debug)]
#[cfg_attr(feature = "cuda", derive(rust_cuda::lend::LendRustToCuda))]
#[cfg_attr(
    feature = "cuda",
    cuda(
        free = "M",
        free = "H",
        free = "G",
        free = "X",
        free = "D",
        free = "T",
        free = "N"
    )
)]
pub struct IndependentEventSampler<
    M: MathsCore,
    H: Habitat<M>,
    G: RngCore<M>,
    X: EmigrationExit<M, H, G, IndependentLineageStore<M, H>>,
    D: DispersalSampler<M, H, G>,
    T: TurnoverRate<M, H>,
    N: SpeciationProbability<M, H>,
> {
    #[cfg_attr(feature = "cuda", cuda(embed))]
    min_spec_sample: Option<SpeciationSample>,
    marker: PhantomData<(M, H, G, X, D, T, N)>,
}

impl<
        M: MathsCore,
        H: Habitat<M>,
        G: RngCore<M>,
        X: EmigrationExit<M, H, G, IndependentLineageStore<M, H>>,
        D: DispersalSampler<M, H, G>,
        T: TurnoverRate<M, H>,
        N: SpeciationProbability<M, H>,
    > Default for IndependentEventSampler<M, H, G, X, D, T, N>
{
    fn default() -> Self {
        Self {
            min_spec_sample: None,
            marker: PhantomData::<(M, H, G, X, D, T, N)>,
        }
    }
}

#[contract_trait]
impl<
        M: MathsCore,
        H: Habitat<M>,
        G: RngCore<M>,
        X: EmigrationExit<M, H, G, IndependentLineageStore<M, H>>,
        D: DispersalSampler<M, H, G>,
        T: TurnoverRate<M, H>,
        N: SpeciationProbability<M, H>,
    > Backup for IndependentEventSampler<M, H, G, X, D, T, N>
{
    unsafe fn backup_unchecked(&self) -> Self {
        Self {
            min_spec_sample: self.min_spec_sample.clone(),
            marker: PhantomData::<(M, H, G, X, D, T, N)>,
        }
    }
}

#[contract_trait]
impl<
        M: MathsCore,
        H: Habitat<M>,
        G: RngCore<M>,
        X: EmigrationExit<M, H, G, IndependentLineageStore<M, H>>,
        D: DispersalSampler<M, H, G>,
        T: TurnoverRate<M, H>,
        N: SpeciationProbability<M, H>,
    >
    EventSampler<
        M,
        H,
        G,
        IndependentLineageStore<M, H>,
        X,
        D,
        IndependentCoalescenceSampler<M, H>,
        T,
        N,
    > for IndependentEventSampler<M, H, G, X, D, T, N>
{
    #[must_use]
    #[inline]
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
            IndependentLineageStore<M, H>,
            X,
            D,
            IndependentCoalescenceSampler<M, H>,
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
        use necsim_core::cogs::RngSampler;

        let speciation_sample = rng.sample_uniform_closed_open();

        SpeciationSample::update_min(
            &mut self.min_spec_sample,
            speciation_sample,
            event_time,
            &dispersal_origin,
        );

        if speciation_sample
            < simulation
                .speciation_probability
                .get_speciation_probability_at_location(
                    dispersal_origin.location(),
                    &simulation.habitat,
                )
        {
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

impl<
        M: MathsCore,
        H: Habitat<M>,
        G: RngCore<M>,
        X: EmigrationExit<M, H, G, IndependentLineageStore<M, H>>,
        D: DispersalSampler<M, H, G>,
        T: TurnoverRate<M, H>,
        N: SpeciationProbability<M, H>,
    >
    MinSpeciationTrackingEventSampler<
        M,
        H,
        G,
        IndependentLineageStore<M, H>,
        X,
        D,
        IndependentCoalescenceSampler<M, H>,
        T,
        N,
    > for IndependentEventSampler<M, H, G, X, D, T, N>
{
    fn replace_min_speciation(
        &mut self,
        new: Option<SpeciationSample>,
    ) -> Option<SpeciationSample> {
        // `core::mem::replace()` would be semantically better
        //  - but `clone()` does not spill to local memory
        let old_value = self.min_spec_sample.clone();

        self.min_spec_sample = new;

        old_value
    }
}
