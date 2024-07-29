use alloc::vec::Vec;
use core::marker::PhantomData;

use necsim_core::cogs::{
    Backup, DispersalSampler, EmigrationExit, Habitat, ImmigrationEntry,
    LocallyCoherentLineageStore, MathsCore, RngCore, SpeciationProbability,
};
use necsim_core_bond::NonNegativeF64;

use crate::cogs::{
    active_lineage_sampler::resuming::lineage::ExceptionalLineage,
    origin_sampler::{TrustedOriginSampler, UntrustedOriginSampler},
};

mod sampler;

#[expect(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct ClassicalActiveLineageSampler<
    M: MathsCore,
    H: Habitat<M>,
    G: RngCore<M>,
    S: LocallyCoherentLineageStore<M, H>,
    X: EmigrationExit<M, H, G, S>,
    D: DispersalSampler<M, H, G>,
    N: SpeciationProbability<M, H>,
    I: ImmigrationEntry<M>,
> {
    active_lineage_references: Vec<S::LocalLineageReference>,
    last_event_time: NonNegativeF64,
    #[expect(clippy::type_complexity)]
    _marker: PhantomData<(M, H, G, S, X, D, N, I)>,
}

impl<
        M: MathsCore,
        H: Habitat<M>,
        G: RngCore<M>,
        S: LocallyCoherentLineageStore<M, H>,
        X: EmigrationExit<M, H, G, S>,
        D: DispersalSampler<M, H, G>,
        N: SpeciationProbability<M, H>,
        I: ImmigrationEntry<M>,
    > ClassicalActiveLineageSampler<M, H, G, S, X, D, N, I>
{
    #[must_use]
    pub fn init_with_store<'h, O: TrustedOriginSampler<'h, M, Habitat = H>>(
        origin_sampler: O,
    ) -> (S, Self)
    where
        H: 'h,
    {
        let (lineage_store, active_lineage_sampler, _) =
            Self::resume_with_store(origin_sampler, NonNegativeF64::zero());

        (lineage_store, active_lineage_sampler)
    }

    #[must_use]
    pub fn resume_with_store<'h, O: UntrustedOriginSampler<'h, M, Habitat = H>>(
        mut origin_sampler: O,
        resume_time: NonNegativeF64,
    ) -> (S, Self, Vec<ExceptionalLineage>)
    where
        H: 'h,
    {
        #[expect(clippy::cast_possible_truncation)]
        let capacity = origin_sampler.full_upper_bound_size_hint() as usize;

        let mut lineage_store = S::with_capacity(origin_sampler.habitat(), capacity);

        let mut active_lineage_references = Vec::with_capacity(capacity);
        let mut last_event_time = resume_time;

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

            if let Some(parent) = lineage_store.get_global_lineage_reference_at_indexed_location(
                &lineage.indexed_location,
                origin_sampler.habitat(),
            ) {
                exceptional_lineages.push(ExceptionalLineage::Coalescence {
                    child: lineage,
                    parent: parent.clone(),
                });
                continue;
            }

            last_event_time = last_event_time.max(lineage.last_event_time);

            active_lineage_references.push(
                lineage_store.insert_lineage_locally_coherent(lineage, origin_sampler.habitat()),
            );
        }

        (
            lineage_store,
            Self {
                active_lineage_references,
                last_event_time,
                _marker: PhantomData::<(M, H, G, S, X, D, N, I)>,
            },
            exceptional_lineages,
        )
    }
}

#[contract_trait]
impl<
        M: MathsCore,
        H: Habitat<M>,
        G: RngCore<M>,
        S: LocallyCoherentLineageStore<M, H>,
        X: EmigrationExit<M, H, G, S>,
        D: DispersalSampler<M, H, G>,
        N: SpeciationProbability<M, H>,
        I: ImmigrationEntry<M>,
    > Backup for ClassicalActiveLineageSampler<M, H, G, S, X, D, N, I>
{
    unsafe fn backup_unchecked(&self) -> Self {
        Self {
            active_lineage_references: self
                .active_lineage_references
                .iter()
                .map(|x| x.backup_unchecked())
                .collect(),
            last_event_time: self.last_event_time,
            _marker: PhantomData::<(M, H, G, S, X, D, N, I)>,
        }
    }
}
