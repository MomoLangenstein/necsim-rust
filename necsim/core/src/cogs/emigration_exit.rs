use necsim_core_bond::{NonNegativeF64, PositiveF64};

use crate::{
    cogs::{Habitat, LineageStore, MathsCore, RngCore},
    landscape::{IndexedLocation, Location},
    lineage::GlobalLineageReference,
    simulation::partial::emigration_exit::PartialSimulation,
};

#[expect(clippy::too_many_arguments)]
#[contract_trait]
pub trait EmigrationExit<M: MathsCore, H: Habitat<M>, G: RngCore<M>, S: LineageStore<M, H>>:
    crate::cogs::Backup + core::fmt::Debug
{
    #[must_use]
    #[debug_ensures(match &ret {
        Some((
            ret_lineage_reference,
            ret_dispersal_origin,
            ret_dispersal_target,
            ret_prior_time,
            ret_event_time,
        )) => {
            ret_lineage_reference == &old(global_reference.clone()) &&
            ret_dispersal_origin == &old(dispersal_origin.clone()) &&
            ret_dispersal_target == &old(dispersal_target.clone()) &&
            ret_prior_time == &old(prior_time) &&
            ret_event_time == &old(event_time)
        },
        None => true,
    }, "if ret is Some, it returns the input parameters unchanged")]
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
    )>;
}
