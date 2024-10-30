use alloc::vec::Vec;
use core::{fmt, marker::PhantomData};

use necsim_core_bond::{NonNegativeF64, PositiveF64};

use necsim_core::cogs::{
    Backup, CoalescenceSampler, DispersalSampler, EmigrationExit, EventSampler, Habitat,
    ImmigrationEntry, LocallyCoherentLineageStore, MathsCore, RngCore, SpeciationProbability,
    TurnoverRate,
};

use crate::cogs::{
    active_lineage_sampler::resuming::lineage::ExceptionalLineage,
    origin_sampler::{TrustedOriginSampler, UntrustedOriginSampler},
};

use super::sampler::stack::DynamicAliasMethodStackSampler;

mod sampler;

#[expect(clippy::module_name_repetitions)]
pub struct IndividualAliasActiveLineageSampler<
    M: MathsCore,
    H: Habitat<M>,
    G: RngCore<M>,
    S: LocallyCoherentLineageStore<M, H>,
    X: EmigrationExit<M, H, G, S>,
    D: DispersalSampler<M, H, G>,
    C: CoalescenceSampler<M, H, S>,
    T: TurnoverRate<M, H>,
    N: SpeciationProbability<M, H>,
    E: EventSampler<M, H, G, S, X, D, C, T, N>,
    I: ImmigrationEntry<M>,
> {
    alias_sampler: DynamicAliasMethodStackSampler<S::LocalLineageReference>,
    number_active_lineages: usize,
    last_event_time: NonNegativeF64,
    #[expect(clippy::type_complexity)]
    marker: PhantomData<(M, H, G, S, X, D, C, T, N, E, I)>,
}

impl<
        M: MathsCore,
        H: Habitat<M>,
        G: RngCore<M>,
        S: LocallyCoherentLineageStore<M, H>,
        X: EmigrationExit<M, H, G, S>,
        D: DispersalSampler<M, H, G>,
        C: CoalescenceSampler<M, H, S>,
        T: TurnoverRate<M, H>,
        N: SpeciationProbability<M, H>,
        E: EventSampler<M, H, G, S, X, D, C, T, N>,
        I: ImmigrationEntry<M>,
    > IndividualAliasActiveLineageSampler<M, H, G, S, X, D, C, T, N, E, I>
{
    #[must_use]
    pub fn init_with_store<'h, O: TrustedOriginSampler<'h, M, Habitat = H>>(
        origin_sampler: O,
        turnover_rate: &T,
    ) -> (S, Self)
    where
        H: 'h,
    {
        let (lineage_store, active_lineage_sampler, _) =
            Self::resume_with_store(origin_sampler, turnover_rate, NonNegativeF64::zero());

        (lineage_store, active_lineage_sampler)
    }

    #[must_use]
    pub fn resume_with_store<'h, O: UntrustedOriginSampler<'h, M, Habitat = H>>(
        mut origin_sampler: O,
        turnover_rate: &T,
        resume_time: NonNegativeF64,
    ) -> (S, Self, Vec<ExceptionalLineage>)
    where
        H: 'h,
    {
        #[expect(clippy::cast_possible_truncation)]
        let capacity = origin_sampler.full_upper_bound_size_hint() as usize;

        let mut lineage_store = S::with_capacity(origin_sampler.habitat(), capacity);

        let mut alias_sampler = DynamicAliasMethodStackSampler::new();
        let mut number_active_lineages: usize = 0;
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

            let turnover_rate = turnover_rate.get_turnover_rate_at_location(
                lineage.indexed_location.location(),
                origin_sampler.habitat(),
            );

            if let Ok(event_rate) = PositiveF64::new(turnover_rate.get()) {
                last_event_time = last_event_time.max(lineage.last_event_time);

                let local_reference = lineage_store
                    .insert_lineage_locally_coherent(lineage, origin_sampler.habitat());

                alias_sampler.add_push(local_reference, event_rate);

                number_active_lineages += 1;
            }
        }

        (
            lineage_store,
            Self {
                alias_sampler,
                number_active_lineages,
                last_event_time,
                marker: PhantomData::<(M, H, G, S, X, D, C, T, N, E, I)>,
            },
            exceptional_lineages,
        )
    }
}

impl<
        M: MathsCore,
        H: Habitat<M>,
        G: RngCore<M>,
        S: LocallyCoherentLineageStore<M, H>,
        X: EmigrationExit<M, H, G, S>,
        D: DispersalSampler<M, H, G>,
        C: CoalescenceSampler<M, H, S>,
        T: TurnoverRate<M, H>,
        N: SpeciationProbability<M, H>,
        E: EventSampler<M, H, G, S, X, D, C, T, N>,
        I: ImmigrationEntry<M>,
    > fmt::Debug for IndividualAliasActiveLineageSampler<M, H, G, S, X, D, C, T, N, E, I>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("IndividualAliasActiveLineageSampler")
            .field("alias_sampler", &self.alias_sampler)
            .field("number_active_lineages", &self.number_active_lineages)
            .finish_non_exhaustive()
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
        C: CoalescenceSampler<M, H, S>,
        T: TurnoverRate<M, H>,
        N: SpeciationProbability<M, H>,
        E: EventSampler<M, H, G, S, X, D, C, T, N>,
        I: ImmigrationEntry<M>,
    > Backup for IndividualAliasActiveLineageSampler<M, H, G, S, X, D, C, T, N, E, I>
{
    unsafe fn backup_unchecked(&self) -> Self {
        Self {
            alias_sampler: self.alias_sampler.backup_unchecked(),
            number_active_lineages: self.number_active_lineages,
            last_event_time: self.last_event_time,
            marker: PhantomData::<(M, H, G, S, X, D, C, T, N, E, I)>,
        }
    }
}
