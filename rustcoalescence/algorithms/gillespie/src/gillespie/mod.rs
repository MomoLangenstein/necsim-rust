use necsim_core::{
    cogs::{LocallyCoherentLineageStore, MathsCore, SplittableRng},
    reporter::Reporter,
};
use necsim_impls_no_std::cogs::{
    lineage_store::coherent::locally::classical::ClassicalLineageStore,
    maths::intrinsics::IntrinsicsMathsCore,
};
use necsim_impls_std::cogs::rng::pcg::Pcg;

use necsim_partitioning_core::{partition::PartitionSize, LocalPartition, Partitioning};
use rustcoalescence_algorithms::{AlgorithmDefaults, AlgorithmDispatch, AlgorithmParamters};
use rustcoalescence_scenarios::Scenario;

use crate::arguments::{get_gillespie_logical_partition_size, GillespieArguments};

mod classical;
mod turnover;

#[expect(clippy::module_name_repetitions, clippy::empty_enum)]
pub enum GillespieAlgorithm {}

impl AlgorithmParamters for GillespieAlgorithm {
    type Arguments = GillespieArguments;
    type Error = !;
}

impl AlgorithmDefaults for GillespieAlgorithm {
    type MathsCore = IntrinsicsMathsCore;
    type Rng<M: MathsCore> = Pcg<M>;
}

impl<M: MathsCore, G: SplittableRng<M>, O: Scenario<M, G>, R: Reporter>
    AlgorithmDispatch<M, G, O, R> for GillespieAlgorithm
where
    O::LineageStore<ClassicalLineageStore<M, O::Habitat>>:
        LocallyCoherentLineageStore<M, O::Habitat>,
{
    type Algorithm<'p, P: LocalPartition<'p, R>> = Self;

    fn get_logical_partition_size<P: Partitioning>(
        args: &Self::Arguments,
        partitioning: &P,
    ) -> PartitionSize {
        get_gillespie_logical_partition_size(args, partitioning)
    }
}
