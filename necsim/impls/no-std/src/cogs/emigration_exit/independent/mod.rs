use core::marker::PhantomData;

use necsim_core::{
    cogs::{Backup, CoalescenceRngSample, EmigrationExit, Habitat, RngCore},
    landscape::{IndexedLocation, Location},
    lineage::{GlobalLineageReference, MigratingLineage},
    simulation::partial::emigration_exit::PartialSimulation,
};

use crate::{
    cogs::lineage_store::independent::IndependentLineageStore, decomposition::Decomposition,
};

pub mod choice;
use choice::EmigrationChoice;

#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct IndependentEmigrationExit<H: Habitat, C: Decomposition<H>, E: EmigrationChoice<H>> {
    decomposition: C,
    choice: E,
    emigrant: Option<(u32, MigratingLineage)>,
    _marker: PhantomData<H>,
}

#[contract_trait]
impl<H: Habitat, C: Decomposition<H>, E: EmigrationChoice<H>> Backup
    for IndependentEmigrationExit<H, C, E>
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
            _marker: PhantomData::<H>,
        }
    }
}

#[contract_trait]
impl<H: Habitat, C: Decomposition<H>, E: EmigrationChoice<H>, G: RngCore>
    EmigrationExit<H, G, GlobalLineageReference, IndependentLineageStore<H>>
    for IndependentEmigrationExit<H, C, E>
{
    #[must_use]
    #[inline]
    #[debug_requires(self.emigrant.is_none(), "can only hold one emigrant")]
    #[debug_ensures(ret.is_some() == (
        (
            old(self.decomposition.map_location_to_subdomain_rank(
                &dispersal_target, &simulation.habitat
            )) == self.decomposition.get_subdomain_rank()
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
        event_time: f64,
        simulation: &mut PartialSimulation<
            H,
            G,
            GlobalLineageReference,
            IndependentLineageStore<H>,
        >,
        rng: &mut G,
    ) -> Option<(GlobalLineageReference, IndexedLocation, Location, f64)> {
        let target_subdomain = self
            .decomposition
            .map_location_to_subdomain_rank(&dispersal_target, &simulation.habitat);

        if (target_subdomain == self.decomposition.get_subdomain_rank())
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
                event_time,
            ));
        }

        self.emigrant = Some((
            target_subdomain,
            MigratingLineage {
                global_reference: lineage_reference,
                dispersal_origin,
                dispersal_target,
                event_time,
                coalescence_rng_sample: CoalescenceRngSample::new(rng),
            },
        ));

        None
    }
}

impl<H: Habitat, C: Decomposition<H>, E: EmigrationChoice<H>> IndependentEmigrationExit<H, C, E> {
    #[must_use]
    pub fn new(decomposition: C, choice: E) -> Self {
        Self {
            decomposition,
            choice,
            emigrant: None,
            _marker: PhantomData::<H>,
        }
    }

    pub fn len(&self) -> usize {
        self.emigrant.is_some() as usize
    }

    pub fn is_empty(&self) -> bool {
        self.emigrant.is_none()
    }

    pub fn take(&mut self) -> Option<(u32, MigratingLineage)> {
        self.emigrant.take()
    }
}
