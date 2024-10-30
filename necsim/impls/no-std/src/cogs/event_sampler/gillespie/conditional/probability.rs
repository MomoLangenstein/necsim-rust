use necsim_core::{
    cogs::{
        GloballyCoherentLineageStore, Habitat, MathsCore, RngCore, SeparableDispersalSampler,
        SpeciationProbability,
    },
    landscape::Location,
};
use necsim_core_bond::ClosedUnitF64;

use crate::cogs::coalescence_sampler::conditional::ConditionalCoalescenceSampler;

#[expect(clippy::module_name_repetitions)]
pub struct ProbabilityAtLocation {
    speciation: ClosedUnitF64,
    out_dispersal: ClosedUnitF64,
    self_coalescence: ClosedUnitF64,
}

impl ProbabilityAtLocation {
    pub fn new<
        M: MathsCore,
        H: Habitat<M>,
        G: RngCore<M>,
        S: GloballyCoherentLineageStore<M, H>,
        D: SeparableDispersalSampler<M, H, G>,
        N: SpeciationProbability<M, H>,
    >(
        location: &Location,
        habitat: &H,
        lineage_store: &S,
        dispersal_sampler: &D,
        coalescence_sampler: &ConditionalCoalescenceSampler<M, H, S>,
        speciation_probability: &N,
        lineage_store_includes_self: bool,
    ) -> Self {
        let speciation_probability =
            speciation_probability.get_speciation_probability_at_location(location, habitat);
        let self_dispersal_probability =
            dispersal_sampler.get_self_dispersal_probability_at_location(location, habitat);
        let coalescence_probability_at_location = coalescence_sampler
            .get_coalescence_probability_at_location(
                location,
                habitat,
                lineage_store,
                lineage_store_includes_self,
            );

        Self {
            speciation: speciation_probability,
            out_dispersal: speciation_probability.one_minus()
                * self_dispersal_probability.one_minus(),
            self_coalescence: speciation_probability.one_minus()
                * self_dispersal_probability
                * coalescence_probability_at_location,
        }
    }

    pub fn speciation(&self) -> ClosedUnitF64 {
        self.speciation
    }

    pub fn out_dispersal(&self) -> ClosedUnitF64 {
        self.out_dispersal
    }

    pub fn self_coalescence(&self) -> ClosedUnitF64 {
        self.self_coalescence
    }

    pub fn total(&self) -> ClosedUnitF64 {
        let total =
            self.speciation().get() + self.out_dispersal().get() + self.self_coalescence().get();

        // Safety: Sum of disjoint event probabilities is in [0.0; 1.0]
        unsafe { ClosedUnitF64::new_unchecked(total) }
    }
}
