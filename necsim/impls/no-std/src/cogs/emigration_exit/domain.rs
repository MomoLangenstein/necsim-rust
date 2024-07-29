use alloc::vec::Vec;
use core::marker::PhantomData;

use necsim_core_bond::{NonNegativeF64, PositiveF64};

use necsim_core::{
    cogs::{
        coalescence_sampler::CoalescenceRngSample, Backup, EmigrationExit, Habitat,
        LocallyCoherentLineageStore, MathsCore, RngCore,
    },
    landscape::{IndexedLocation, Location},
    lineage::{GlobalLineageReference, MigratingLineage, TieBreaker},
    simulation::partial::emigration_exit::PartialSimulation,
};

use crate::decomposition::Decomposition;

#[expect(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct DomainEmigrationExit<M: MathsCore, H: Habitat<M>, C: Decomposition<M, H>> {
    decomposition: C,
    emigrants: Vec<(u32, MigratingLineage)>,
    _marker: PhantomData<(M, H)>,
}

#[contract_trait]
impl<M: MathsCore, H: Habitat<M>, C: Decomposition<M, H>> Backup for DomainEmigrationExit<M, H, C> {
    unsafe fn backup_unchecked(&self) -> Self {
        Self {
            decomposition: self.decomposition.backup_unchecked(),
            emigrants: self
                .emigrants
                .iter()
                .map(|(partition, migrating_lineage)| {
                    (*partition, migrating_lineage.backup_unchecked())
                })
                .collect(),
            _marker: PhantomData::<(M, H)>,
        }
    }
}

#[contract_trait]
impl<
        M: MathsCore,
        H: Habitat<M>,
        C: Decomposition<M, H>,
        G: RngCore<M>,
        S: LocallyCoherentLineageStore<M, H>,
    > EmigrationExit<M, H, G, S> for DomainEmigrationExit<M, H, C>
{
    #[must_use]
    #[debug_ensures(ret.is_some() == (
        old(self.decomposition.map_location_to_subdomain_rank(
            &dispersal_target, &simulation.habitat
        )) == self.decomposition.get_subdomain().rank()
    ), "lineage only emigrates to other subdomains")]
    fn optionally_emigrate(
        &mut self,
        global_reference: GlobalLineageReference,
        dispersal_origin: IndexedLocation,
        dispersal_target: Location,
        prior_time: NonNegativeF64,
        event_time: PositiveF64,
        simulation: &mut PartialSimulation<M, H, G, S>,
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

        if target_subdomain == self.decomposition.get_subdomain().rank() {
            return Some((
                global_reference,
                dispersal_origin,
                dispersal_target,
                prior_time,
                event_time,
            ));
        }

        self.emigrants.push((
            target_subdomain,
            MigratingLineage {
                global_reference,
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

impl<M: MathsCore, H: Habitat<M>, C: Decomposition<M, H>> DomainEmigrationExit<M, H, C> {
    #[must_use]
    pub fn new(decomposition: C) -> Self {
        Self {
            decomposition,
            emigrants: Vec::new(),
            _marker: PhantomData::<(M, H)>,
        }
    }

    pub fn len(&self) -> usize {
        self.emigrants.len()
    }

    pub fn is_empty(&self) -> bool {
        self.emigrants.is_empty()
    }
}

impl<M: MathsCore, H: Habitat<M>, C: Decomposition<M, H>> Iterator
    for DomainEmigrationExit<M, H, C>
{
    type Item = (u32, MigratingLineage);

    fn next(&mut self) -> Option<Self::Item> {
        self.emigrants.pop()
    }
}
