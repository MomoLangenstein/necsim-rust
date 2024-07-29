use alloc::vec::Vec;
use core::marker::PhantomData;

use necsim_core::{
    cogs::{
        Backup, DispersalSampler, EmigrationExit, Habitat, MathsCore, PrimeableRng,
        SpeciationProbability, TurnoverRate,
    },
    lineage::Lineage,
};
use necsim_core_bond::NonNegativeF64;

use crate::cogs::{
    active_lineage_sampler::resuming::lineage::ExceptionalLineage,
    lineage_store::independent::IndependentLineageStore,
    origin_sampler::{TrustedOriginSampler, UntrustedOriginSampler},
};

mod sampler;
mod singular;

pub mod event_time_sampler;

use event_time_sampler::EventTimeSampler;

#[expect(clippy::module_name_repetitions)]
#[derive(Debug)]
#[cfg_attr(feature = "cuda", derive(rust_cuda::lend::LendRustToCuda))]
#[cfg_attr(feature = "cuda", cuda(free = "M"))]
pub struct IndependentActiveLineageSampler<
    M: MathsCore,
    H: Habitat<M>,
    G: PrimeableRng<M>,
    X: EmigrationExit<M, H, G, IndependentLineageStore<M, H>>,
    D: DispersalSampler<M, H, G>,
    T: TurnoverRate<M, H>,
    N: SpeciationProbability<M, H>,
    J: EventTimeSampler<M, H, G, T>,
> {
    #[cfg_attr(feature = "cuda", cuda(embed))]
    active_lineage: Option<Lineage>,
    min_event_time: NonNegativeF64,
    last_event_time: NonNegativeF64,
    #[cfg_attr(feature = "cuda", cuda(embed))]
    event_time_sampler: J,
    marker: PhantomData<(M, H, G, X, D, T, N)>,
}

impl<
        M: MathsCore,
        H: Habitat<M>,
        G: PrimeableRng<M>,
        X: EmigrationExit<M, H, G, IndependentLineageStore<M, H>>,
        D: DispersalSampler<M, H, G>,
        T: TurnoverRate<M, H>,
        N: SpeciationProbability<M, H>,
        J: EventTimeSampler<M, H, G, T>,
    > IndependentActiveLineageSampler<M, H, G, X, D, T, N, J>
{
    #[must_use]
    pub fn init_with_store_and_lineages<'h, O: TrustedOriginSampler<'h, M, Habitat = H>>(
        origin_sampler: O,
        event_time_sampler: J,
    ) -> (IndependentLineageStore<M, H>, Self, Vec<Lineage>)
    where
        H: 'h,
    {
        let (lineage_store, active_lineage_sampler, lineages, _) =
            Self::resume_with_store_and_lineages(
                origin_sampler,
                event_time_sampler,
                NonNegativeF64::zero(),
            );

        (lineage_store, active_lineage_sampler, lineages)
    }

    #[must_use]
    pub fn resume_with_store_and_lineages<'h, O: UntrustedOriginSampler<'h, M, Habitat = H>>(
        mut origin_sampler: O,
        event_time_sampler: J,
        resume_time: NonNegativeF64,
    ) -> (
        IndependentLineageStore<M, H>,
        Self,
        Vec<Lineage>,
        Vec<ExceptionalLineage>,
    )
    where
        H: 'h,
    {
        #[expect(clippy::cast_possible_truncation)]
        let capacity = origin_sampler.full_upper_bound_size_hint() as usize;

        let mut lineages = Vec::with_capacity(capacity);
        let mut exceptional_lineages = Vec::new();

        while let Some(lineage) = origin_sampler.next() {
            if !origin_sampler
                .habitat()
                .is_location_habitable(lineage.indexed_location.location())
            {
                exceptional_lineages.push(ExceptionalLineage::OutOfHabitat(lineage));
                continue;
            }

            if lineage.indexed_location.index()
                >= origin_sampler
                    .habitat()
                    .get_habitat_at_location(lineage.indexed_location.location())
            {
                exceptional_lineages.push(ExceptionalLineage::OutOfDeme(lineage));
                continue;
            }

            lineages.push(lineage);
        }

        (
            IndependentLineageStore::default(),
            Self {
                active_lineage: None,
                min_event_time: resume_time,
                last_event_time: NonNegativeF64::zero(),
                event_time_sampler,
                marker: PhantomData::<(M, H, G, X, D, T, N)>,
            },
            lineages,
            exceptional_lineages,
        )
    }
}

#[contract_trait]
impl<
        M: MathsCore,
        H: Habitat<M>,
        G: PrimeableRng<M>,
        X: EmigrationExit<M, H, G, IndependentLineageStore<M, H>>,
        D: DispersalSampler<M, H, G>,
        T: TurnoverRate<M, H>,
        N: SpeciationProbability<M, H>,
        J: EventTimeSampler<M, H, G, T>,
    > Backup for IndependentActiveLineageSampler<M, H, G, X, D, T, N, J>
{
    unsafe fn backup_unchecked(&self) -> Self {
        Self {
            active_lineage: self.active_lineage.clone(),
            min_event_time: self.min_event_time,
            last_event_time: self.last_event_time,
            event_time_sampler: self.event_time_sampler.clone(),
            marker: PhantomData::<(M, H, G, X, D, T, N)>,
        }
    }
}
