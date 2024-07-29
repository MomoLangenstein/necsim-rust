use core::marker::PhantomData;

use necsim_core::{
    cogs::{
        coalescence_sampler::CoalescenceRngSample, Backup, EmigrationExit, Habitat, MathsCore,
        RngCore,
    },
    landscape::{IndexedLocation, Location},
    lineage::{GlobalLineageReference, MigratingLineage, TieBreaker},
    simulation::partial::emigration_exit::PartialSimulation,
};
use necsim_core_bond::{NonNegativeF64, PositiveF64};

use crate::{
    cogs::lineage_store::independent::IndependentLineageStore, decomposition::Decomposition,
};

pub mod choice;
use choice::EmigrationChoice;

#[expect(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct IndependentEmigrationExit<
    M: MathsCore,
    H: Habitat<M>,
    C: Decomposition<M, H>,
    E: EmigrationChoice<M, H>,
> {
    decomposition: C,
    choice: E,
    emigrant: Option<(u32, MigratingLineage)>,
    _marker: PhantomData<(M, H)>,
}

#[contract_trait]
impl<M: MathsCore, H: Habitat<M>, C: Decomposition<M, H>, E: EmigrationChoice<M, H>> Backup
    for IndependentEmigrationExit<M, H, C, E>
{
    unsafe fn backup_unchecked(&self) -> Self {
        Self {
            decomposition: self.decomposition.backup_unchecked(),
            choice: self.choice.backup_unchecked(),
            emigrant: self
                .emigrant
                .as_ref()
                .map(|(partition, migrating_lineage)| {
                    (*partition, migrating_lineage.backup_unchecked())
                }),
            _marker: PhantomData::<(M, H)>,
        }
    }
}

#[contract_trait]
impl<
        M: MathsCore,
        H: Habitat<M>,
        C: Decomposition<M, H>,
        E: EmigrationChoice<M, H>,
        G: RngCore<M>,
    > EmigrationExit<M, H, G, IndependentLineageStore<M, H>>
    for IndependentEmigrationExit<M, H, C, E>
{
    #[must_use]
    #[inline]
    #[debug_requires(self.emigrant.is_none(), "can only hold one emigrant")]
    #[debug_ensures(ret.is_some() == (
        (
            old(self.decomposition.map_location_to_subdomain_rank(
                &dispersal_target, &simulation.habitat
            )) == self.decomposition.get_subdomain().rank()
        ) || !old(self.choice.should_lineage_emigrate(
            &dispersal_origin,
            event_time,
            &simulation.habitat,
        ))
    ), "lineage only emigrates to other subdomains")]
    fn optionally_emigrate(
        &mut self,
        lineage_reference: GlobalLineageReference,
        dispersal_origin: IndexedLocation,
        dispersal_target: Location,
        prior_time: NonNegativeF64,
        event_time: PositiveF64,
        simulation: &mut PartialSimulation<M, H, G, IndependentLineageStore<M, H>>,
        rng: &mut G,
    ) -> Option<(
        GlobalLineageReference,
        IndexedLocation,
        Location,
        NonNegativeF64,
        PositiveF64,
    )> {
        let target_subdomain = self
            .decomposition
            .map_location_to_subdomain_rank(&dispersal_target, &simulation.habitat);

        if (target_subdomain == self.decomposition.get_subdomain().rank())
            || !self.choice.should_lineage_emigrate(
                &dispersal_origin,
                event_time,
                &simulation.habitat,
            )
        {
            return Some((
                lineage_reference,
                dispersal_origin,
                dispersal_target,
                prior_time,
                event_time,
            ));
        }

        self.emigrant = Some((
            target_subdomain,
            MigratingLineage {
                global_reference: lineage_reference,
                dispersal_origin,
                dispersal_target,
                prior_time,
                event_time,
                coalescence_rng_sample: CoalescenceRngSample::new(rng),
                tie_breaker: if self.decomposition.get_subdomain().rank() < target_subdomain {
                    TieBreaker::PreferImmigrant
                } else {
                    TieBreaker::PreferLocal
                },
            },
        ));

        None
    }
}

impl<M: MathsCore, H: Habitat<M>, C: Decomposition<M, H>, E: EmigrationChoice<M, H>>
    IndependentEmigrationExit<M, H, C, E>
{
    #[must_use]
    pub fn new(decomposition: C, choice: E) -> Self {
        Self {
            decomposition,
            choice,
            emigrant: None,
            _marker: PhantomData::<(M, H)>,
        }
    }

    pub fn len(&self) -> usize {
        usize::from(self.emigrant.is_some())
    }

    pub fn is_empty(&self) -> bool {
        self.emigrant.is_none()
    }

    pub fn take(&mut self) -> Option<(u32, MigratingLineage)> {
        self.emigrant.take()
    }
}
