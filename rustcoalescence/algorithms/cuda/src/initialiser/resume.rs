use necsim_core::{
    cogs::{EmigrationExit, MathsCore, PrimeableRng},
    lineage::Lineage,
};
use necsim_core_bond::NonNegativeF64;

use necsim_impls_no_std::cogs::{
    active_lineage_sampler::{
        independent::{event_time_sampler::EventTimeSampler, IndependentActiveLineageSampler},
        resuming::lineage::ExceptionalLineage,
    },
    lineage_store::independent::IndependentLineageStore,
    origin_sampler::{resuming::ResumingOriginSampler, TrustedOriginSampler},
};

use rustcoalescence_algorithms::result::ResumeError;
use rustcoalescence_scenarios::Scenario;

use rust_cuda::lend::RustToCuda;

use crate::CudaError;

use super::CudaLineageStoreSampleInitialiser;

#[expect(clippy::module_name_repetitions)]
pub struct ResumeInitialiser<L: ExactSizeIterator<Item = Lineage>> {
    pub lineages: L,
    pub resume_after: Option<NonNegativeF64>,
}

impl<
        L: ExactSizeIterator<Item = Lineage>,
        M: MathsCore + Sync,
        G: PrimeableRng<M> + RustToCuda + Sync,
        O: Scenario<M, G>,
    > CudaLineageStoreSampleInitialiser<M, G, O, ResumeError<CudaError>> for ResumeInitialiser<L>
where
    O::Habitat: RustToCuda + Sync,
    O::DispersalSampler: RustToCuda + Sync,
    O::TurnoverRate: RustToCuda + Sync,
    O::SpeciationProbability: RustToCuda + Sync,
{
    type ActiveLineageSampler<
        X: EmigrationExit<M, O::Habitat, G, IndependentLineageStore<M, O::Habitat>>
            + RustToCuda
            + Sync,
        J: EventTimeSampler<M, O::Habitat, G, O::TurnoverRate> + RustToCuda + Sync,
    > = IndependentActiveLineageSampler<
        M,
        O::Habitat,
        G,
        X,
        Self::DispersalSampler,
        O::TurnoverRate,
        O::SpeciationProbability,
        J,
    >;
    type DispersalSampler = O::DispersalSampler;

    fn init<
        'h,
        T: TrustedOriginSampler<'h, M, Habitat = O::Habitat>,
        J: EventTimeSampler<M, O::Habitat, G, O::TurnoverRate> + RustToCuda + Sync,
        X: EmigrationExit<M, O::Habitat, G, IndependentLineageStore<M, O::Habitat>>
            + RustToCuda
            + Sync,
    >(
        self,
        origin_sampler: T,
        dispersal_sampler: O::DispersalSampler,
        event_time_sampler: J,
    ) -> Result<
        (
            IndependentLineageStore<M, O::Habitat>,
            Self::DispersalSampler,
            Self::ActiveLineageSampler<X, J>,
            Vec<Lineage>,
            Vec<Lineage>,
        ),
        ResumeError<CudaError>,
    >
    where
        O::Habitat: 'h,
    {
        let habitat = origin_sampler.habitat();
        let pre_sampler = origin_sampler.into_pre_sampler();

        let (lineage_store, active_lineage_sampler, mut lineages, mut exceptional_lineages) =
            IndependentActiveLineageSampler::resume_with_store_and_lineages(
                ResumingOriginSampler::new(habitat, pre_sampler, self.lineages),
                event_time_sampler,
                self.resume_after.unwrap_or(NonNegativeF64::zero()),
            );

        // The Independent algorithm can deal with late coalescence
        lineages.extend(ExceptionalLineage::drain_coalescing_lineages(
            &mut exceptional_lineages,
        ));

        if !exceptional_lineages.is_empty() {
            return Err(ResumeError::Sample(exceptional_lineages));
        }

        Ok((
            lineage_store,
            dispersal_sampler,
            active_lineage_sampler,
            lineages,
            Vec::new(),
        ))
    }
}
