use necsim_core::{
    cogs::{LocallyCoherentLineageStore, MathsCore, SplittableRng},
    lineage::Lineage,
    reporter::Reporter,
};
use necsim_core_bond::{NonNegativeF64, PositiveF64};

use necsim_impls_no_std::cogs::{
    lineage_store::coherent::locally::classical::ClassicalLineageStore,
    origin_sampler::pre_sampler::OriginPreSampler,
};
use necsim_partitioning_core::{partition::Partition, LocalPartition};

use rustcoalescence_algorithms::{
    result::{ResumeError, SimulationOutcome},
    strategy::RestartFixUpStrategy,
    Algorithm,
};
use rustcoalescence_scenarios::{Scenario, ScenarioCogs};

use crate::arguments::get_gillespie_logical_partition;

use super::GillespieAlgorithm;

mod initialiser;
mod launch;

use initialiser::{
    fixup::FixUpInitialiser, genesis::GenesisInitialiser, resume::ResumeInitialiser,
};

// Default 'Gillespie' implementation for any turnover sampler
impl<
        'p,
        O: Scenario<M, G>,
        R: Reporter,
        P: LocalPartition<'p, R>,
        M: MathsCore,
        G: SplittableRng<M>,
    > Algorithm<'p, M, G, O, R, P> for GillespieAlgorithm
where
    O::LineageStore<ClassicalLineageStore<M, O::Habitat>>:
        LocallyCoherentLineageStore<M, O::Habitat>,
{
    type LineageStore = O::LineageStore<ClassicalLineageStore<M, O::Habitat>>;

    default fn get_logical_partition(args: &Self::Arguments, local_partition: &P) -> Partition {
        get_gillespie_logical_partition(args, local_partition)
    }

    default fn initialise_and_simulate<I: Iterator<Item = u64>>(
        args: Self::Arguments,
        rng: G,
        scenario: ScenarioCogs<M, G, O>,
        pre_sampler: OriginPreSampler<M, I>,
        pause_before: Option<NonNegativeF64>,
        local_partition: &mut P,
    ) -> Result<SimulationOutcome<M, G>, Self::Error> {
        launch::initialise_and_simulate(
            args,
            rng,
            scenario,
            pre_sampler,
            pause_before,
            local_partition,
            GenesisInitialiser,
        )
    }

    /// # Errors
    ///
    /// Returns a `ContinueError::Sample` if initialising the resuming
    ///  simulation failed
    default fn resume_and_simulate<
        I: Iterator<Item = u64>,
        L: ExactSizeIterator<Item = Lineage>,
    >(
        args: Self::Arguments,
        rng: G,
        scenario: ScenarioCogs<M, G, O>,
        pre_sampler: OriginPreSampler<M, I>,
        lineages: L,
        resume_after: Option<NonNegativeF64>,
        pause_before: Option<NonNegativeF64>,
        local_partition: &mut P,
    ) -> Result<SimulationOutcome<M, G>, ResumeError<Self::Error>> {
        launch::initialise_and_simulate(
            args,
            rng,
            scenario,
            pre_sampler,
            pause_before,
            local_partition,
            ResumeInitialiser {
                lineages,
                resume_after,
            },
        )
    }

    /// # Errors
    ///
    /// Returns a `ContinueError<Self::Error>` if fixing up the restarting
    ///  simulation (incl. running the algorithm) failed
    default fn fixup_for_restart<I: Iterator<Item = u64>, L: ExactSizeIterator<Item = Lineage>>(
        args: Self::Arguments,
        rng: G,
        scenario: ScenarioCogs<M, G, O>,
        pre_sampler: OriginPreSampler<M, I>,
        lineages: L,
        restart_at: PositiveF64,
        fixup_strategy: RestartFixUpStrategy,
        local_partition: &mut P,
    ) -> Result<SimulationOutcome<M, G>, ResumeError<Self::Error>> {
        launch::initialise_and_simulate(
            args,
            rng,
            scenario,
            pre_sampler,
            Some(PositiveF64::max_after(restart_at.into(), restart_at.into()).into()),
            local_partition,
            FixUpInitialiser {
                lineages,
                restart_at,
                fixup_strategy,
            },
        )
    }
}
