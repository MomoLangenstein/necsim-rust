#![deny(clippy::pedantic)]
#![allow(incomplete_features)]
#![feature(inline_const_pat)]

#[macro_use]
extern crate serde_derive_state;

use initialiser::CudaLineageStoreSampleInitialiser;
use necsim_core::{
    cogs::{MathsCore, PrimeableRng},
    lineage::Lineage,
    reporter::Reporter,
};
use necsim_core_bond::{NonNegativeF64, PositiveF64};

use necsim_impls_cuda::cogs::{maths::NvptxMathsCore, rng::CudaRng};
use necsim_impls_no_std::cogs::{
    active_lineage_sampler::independent::{
        event_time_sampler::{exp::ExpEventTimeSampler, r#const::ConstEventTimeSampler},
        IndependentActiveLineageSampler,
    },
    coalescence_sampler::independent::IndependentCoalescenceSampler,
    dispersal_sampler::trespassing::{
        uniform::UniformAntiTrespassingDispersalSampler, TrespassingDispersalSampler,
    },
    emigration_exit::never::NeverEmigrationExit,
    event_sampler::independent::IndependentEventSampler,
    immigration_entry::never::NeverImmigrationEntry,
    lineage_store::independent::IndependentLineageStore,
    origin_sampler::pre_sampler::OriginPreSampler,
    rng::wyhash::WyHash,
};
use necsim_partitioning_core::{
    partition::{Partition, PartitionSize},
    LocalPartition, Partitioning,
};

use rustcoalescence_algorithms::{
    result::{ResumeError, SimulationOutcome},
    strategy::RestartFixUpStrategy,
    Algorithm, AlgorithmDefaults, AlgorithmDispatch, AlgorithmParamters,
};
use rustcoalescence_scenarios::{Scenario, ScenarioCogs};

use rustcoalescence_algorithms_cuda_cpu_kernel::SimulationKernelPtx;
use rustcoalescence_algorithms_cuda_gpu_kernel::simulate;

use rust_cuda::{kernel::CompiledKernelPtx, lend::RustToCuda};

mod arguments;
mod cuda;
mod error;
mod info;
mod initialiser;
mod launch;
mod parallelisation;

use crate::{
    arguments::{CudaArguments, IsolatedParallelismMode, ParallelismMode},
    error::CudaError,
    initialiser::{
        fixup::FixUpInitialiser, genesis::GenesisInitialiser, resume::ResumeInitialiser,
    },
};

pub enum CudaAlgorithm {}

impl AlgorithmParamters for CudaAlgorithm {
    type Arguments = CudaArguments;
    type Error = CudaError;
}

impl AlgorithmDefaults for CudaAlgorithm {
    type MathsCore = NvptxMathsCore;
    type Rng<M: MathsCore> = CudaRng<M, WyHash<M>>;
}

impl<
        M: MathsCore + Sync,
        G: PrimeableRng<M> + RustToCuda + Sync,
        O: Scenario<M, G>,
        R: Reporter,
    > AlgorithmDispatch<M, G, O, R> for CudaAlgorithm
where
    O::Habitat: RustToCuda + Sync,
    O::DispersalSampler: RustToCuda + Sync,
    O::TurnoverRate: RustToCuda + Sync,
    O::SpeciationProbability: RustToCuda + Sync,
    SimulationKernelPtx<
        M,
        O::Habitat,
        G,
        IndependentLineageStore<M, O::Habitat>,
        NeverEmigrationExit,
        O::DispersalSampler,
        IndependentCoalescenceSampler<M, O::Habitat>,
        O::TurnoverRate,
        O::SpeciationProbability,
        IndependentEventSampler<
            M,
            O::Habitat,
            G,
            NeverEmigrationExit,
            O::DispersalSampler,
            O::TurnoverRate,
            O::SpeciationProbability,
        >,
        NeverImmigrationEntry,
        IndependentActiveLineageSampler<
            M,
            O::Habitat,
            G,
            NeverEmigrationExit,
            O::DispersalSampler,
            O::TurnoverRate,
            O::SpeciationProbability,
            ExpEventTimeSampler,
        >,
        R::ReportSpeciation,
        R::ReportDispersal,
    >: CompiledKernelPtx<
        simulate<
            M,
            O::Habitat,
            G,
            IndependentLineageStore<M, O::Habitat>,
            NeverEmigrationExit,
            O::DispersalSampler,
            IndependentCoalescenceSampler<M, O::Habitat>,
            O::TurnoverRate,
            O::SpeciationProbability,
            IndependentEventSampler<
                M,
                O::Habitat,
                G,
                NeverEmigrationExit,
                O::DispersalSampler,
                O::TurnoverRate,
                O::SpeciationProbability,
            >,
            NeverImmigrationEntry,
            IndependentActiveLineageSampler<
                M,
                O::Habitat,
                G,
                NeverEmigrationExit,
                O::DispersalSampler,
                O::TurnoverRate,
                O::SpeciationProbability,
                ExpEventTimeSampler,
            >,
            R::ReportSpeciation,
            R::ReportDispersal,
        >,
    >,
    SimulationKernelPtx<
        M,
        O::Habitat,
        G,
        IndependentLineageStore<M, O::Habitat>,
        NeverEmigrationExit,
        TrespassingDispersalSampler<
            M,
            O::Habitat,
            G,
            O::DispersalSampler,
            UniformAntiTrespassingDispersalSampler<M, O::Habitat, G>,
        >,
        IndependentCoalescenceSampler<M, O::Habitat>,
        O::TurnoverRate,
        O::SpeciationProbability,
        IndependentEventSampler<
            M,
            O::Habitat,
            G,
            NeverEmigrationExit,
            TrespassingDispersalSampler<
                M,
                O::Habitat,
                G,
                O::DispersalSampler,
                UniformAntiTrespassingDispersalSampler<M, O::Habitat, G>,
            >,
            O::TurnoverRate,
            O::SpeciationProbability,
        >,
        NeverImmigrationEntry,
        IndependentActiveLineageSampler<
            M,
            O::Habitat,
            G,
            NeverEmigrationExit,
            TrespassingDispersalSampler<
                M,
                O::Habitat,
                G,
                O::DispersalSampler,
                UniformAntiTrespassingDispersalSampler<M, O::Habitat, G>,
            >,
            O::TurnoverRate,
            O::SpeciationProbability,
            ConstEventTimeSampler,
        >,
        R::ReportSpeciation,
        R::ReportDispersal,
    >: CompiledKernelPtx<
        simulate<
            M,
            O::Habitat,
            G,
            IndependentLineageStore<M, O::Habitat>,
            NeverEmigrationExit,
            TrespassingDispersalSampler<
                M,
                O::Habitat,
                G,
                O::DispersalSampler,
                UniformAntiTrespassingDispersalSampler<M, O::Habitat, G>,
            >,
            IndependentCoalescenceSampler<M, O::Habitat>,
            O::TurnoverRate,
            O::SpeciationProbability,
            IndependentEventSampler<
                M,
                O::Habitat,
                G,
                NeverEmigrationExit,
                TrespassingDispersalSampler<
                    M,
                    O::Habitat,
                    G,
                    O::DispersalSampler,
                    UniformAntiTrespassingDispersalSampler<M, O::Habitat, G>,
                >,
                O::TurnoverRate,
                O::SpeciationProbability,
            >,
            NeverImmigrationEntry,
            IndependentActiveLineageSampler<
                M,
                O::Habitat,
                G,
                NeverEmigrationExit,
                TrespassingDispersalSampler<
                    M,
                    O::Habitat,
                    G,
                    O::DispersalSampler,
                    UniformAntiTrespassingDispersalSampler<M, O::Habitat, G>,
                >,
                O::TurnoverRate,
                O::SpeciationProbability,
                ConstEventTimeSampler,
            >,
            R::ReportSpeciation,
            R::ReportDispersal,
        >,
    >,
{
    type Algorithm<'p, P: LocalPartition<'p, R>> = Self;

    fn get_logical_partition_size<P: Partitioning>(
        args: &Self::Arguments,
        _partitioning: &P,
    ) -> PartitionSize {
        match &args.parallelism_mode {
            ParallelismMode::Monolithic(_) => PartitionSize::MONOLITHIC,
            ParallelismMode::IsolatedIndividuals(IsolatedParallelismMode { partition, .. })
            | ParallelismMode::IsolatedLandscape(IsolatedParallelismMode { partition, .. }) => {
                partition.size()
            },
        }
    }
}

impl<
        'p,
        M: MathsCore + Sync,
        G: PrimeableRng<M> + RustToCuda + Sync,
        O: Scenario<M, G>,
        R: Reporter,
        P: LocalPartition<'p, R>,
    > Algorithm<'p, M, G, O, R, P> for CudaAlgorithm
where
    O::Habitat: RustToCuda + Sync,
    O::DispersalSampler: RustToCuda + Sync,
    O::TurnoverRate: RustToCuda + Sync,
    O::SpeciationProbability: RustToCuda + Sync,
    SimulationKernelPtx<
        M,
        O::Habitat,
        G,
        IndependentLineageStore<M, O::Habitat>,
        NeverEmigrationExit,
        O::DispersalSampler,
        IndependentCoalescenceSampler<M, O::Habitat>,
        O::TurnoverRate,
        O::SpeciationProbability,
        IndependentEventSampler<
            M,
            O::Habitat,
            G,
            NeverEmigrationExit,
            O::DispersalSampler,
            O::TurnoverRate,
            O::SpeciationProbability,
        >,
        NeverImmigrationEntry,
        IndependentActiveLineageSampler<
            M,
            O::Habitat,
            G,
            NeverEmigrationExit,
            O::DispersalSampler,
            O::TurnoverRate,
            O::SpeciationProbability,
            ExpEventTimeSampler,
        >,
        R::ReportSpeciation,
        R::ReportDispersal,
    >: CompiledKernelPtx<
        simulate<
            M,
            O::Habitat,
            G,
            IndependentLineageStore<M, O::Habitat>,
            NeverEmigrationExit,
            O::DispersalSampler,
            IndependentCoalescenceSampler<M, O::Habitat>,
            O::TurnoverRate,
            O::SpeciationProbability,
            IndependentEventSampler<
                M,
                O::Habitat,
                G,
                NeverEmigrationExit,
                O::DispersalSampler,
                O::TurnoverRate,
                O::SpeciationProbability,
            >,
            NeverImmigrationEntry,
            IndependentActiveLineageSampler<
                M,
                O::Habitat,
                G,
                NeverEmigrationExit,
                O::DispersalSampler,
                O::TurnoverRate,
                O::SpeciationProbability,
                ExpEventTimeSampler,
            >,
            R::ReportSpeciation,
            R::ReportDispersal,
        >,
    >,
    SimulationKernelPtx<
        M,
        O::Habitat,
        G,
        IndependentLineageStore<M, O::Habitat>,
        NeverEmigrationExit,
        TrespassingDispersalSampler<
            M,
            O::Habitat,
            G,
            O::DispersalSampler,
            UniformAntiTrespassingDispersalSampler<M, O::Habitat, G>,
        >,
        IndependentCoalescenceSampler<M, O::Habitat>,
        O::TurnoverRate,
        O::SpeciationProbability,
        IndependentEventSampler<
            M,
            O::Habitat,
            G,
            NeverEmigrationExit,
            TrespassingDispersalSampler<
                M,
                O::Habitat,
                G,
                O::DispersalSampler,
                UniformAntiTrespassingDispersalSampler<M, O::Habitat, G>,
            >,
            O::TurnoverRate,
            O::SpeciationProbability,
        >,
        NeverImmigrationEntry,
        IndependentActiveLineageSampler<
            M,
            O::Habitat,
            G,
            NeverEmigrationExit,
            TrespassingDispersalSampler<
                M,
                O::Habitat,
                G,
                O::DispersalSampler,
                UniformAntiTrespassingDispersalSampler<M, O::Habitat, G>,
            >,
            O::TurnoverRate,
            O::SpeciationProbability,
            ConstEventTimeSampler,
        >,
        R::ReportSpeciation,
        R::ReportDispersal,
    >: CompiledKernelPtx<
        simulate<
            M,
            O::Habitat,
            G,
            IndependentLineageStore<M, O::Habitat>,
            NeverEmigrationExit,
            TrespassingDispersalSampler<
                M,
                O::Habitat,
                G,
                O::DispersalSampler,
                UniformAntiTrespassingDispersalSampler<M, O::Habitat, G>,
            >,
            IndependentCoalescenceSampler<M, O::Habitat>,
            O::TurnoverRate,
            O::SpeciationProbability,
            IndependentEventSampler<
                M,
                O::Habitat,
                G,
                NeverEmigrationExit,
                TrespassingDispersalSampler<
                    M,
                    O::Habitat,
                    G,
                    O::DispersalSampler,
                    UniformAntiTrespassingDispersalSampler<M, O::Habitat, G>,
                >,
                O::TurnoverRate,
                O::SpeciationProbability,
            >,
            NeverImmigrationEntry,
            IndependentActiveLineageSampler<
                M,
                O::Habitat,
                G,
                NeverEmigrationExit,
                TrespassingDispersalSampler<
                    M,
                    O::Habitat,
                    G,
                    O::DispersalSampler,
                    UniformAntiTrespassingDispersalSampler<M, O::Habitat, G>,
                >,
                O::TurnoverRate,
                O::SpeciationProbability,
                ConstEventTimeSampler,
            >,
            R::ReportSpeciation,
            R::ReportDispersal,
        >,
    >,
{
    type LineageStore = IndependentLineageStore<M, O::Habitat>;

    fn get_logical_partition(args: &Self::Arguments, _local_partition: &P) -> Partition {
        match &args.parallelism_mode {
            ParallelismMode::Monolithic(_) => Partition::monolithic(),
            ParallelismMode::IsolatedIndividuals(IsolatedParallelismMode { partition, .. })
            | ParallelismMode::IsolatedLandscape(IsolatedParallelismMode { partition, .. }) => {
                *partition
            },
        }
    }

    fn initialise_and_simulate<I: Iterator<Item = u64>>(
        args: Self::Arguments,
        rng: G,
        scenario: ScenarioCogs<M, G, O>,
        pre_sampler: OriginPreSampler<M, I>,
        pause_before: Option<NonNegativeF64>,
        local_partition: &mut P,
    ) -> Result<SimulationOutcome<M, G>, Self::Error> {
        launch::initialise_and_simulate::<_, _, _, _, _, _, _, _, SimulationKernelPtx<
            _,
            _,
            _,
            _,
            _,
            <GenesisInitialiser as CudaLineageStoreSampleInitialiser<_, _, O, _>>::DispersalSampler,
            _,
            _,
            _,
            _,
            _,
            <GenesisInitialiser as CudaLineageStoreSampleInitialiser<_, _, O, _>>::ActiveLineageSampler<_, _>,
            _,
            _,
        >>(
            &args,
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
    fn resume_and_simulate<I: Iterator<Item = u64>, L: ExactSizeIterator<Item = Lineage>>(
        args: Self::Arguments,
        rng: G,
        scenario: ScenarioCogs<M, G, O>,
        pre_sampler: OriginPreSampler<M, I>,
        lineages: L,
        resume_after: Option<NonNegativeF64>,
        pause_before: Option<NonNegativeF64>,
        local_partition: &mut P,
    ) -> Result<SimulationOutcome<M, G>, ResumeError<Self::Error>> {
        launch::initialise_and_simulate::<_, _, _, _, _, _, _, _, SimulationKernelPtx<
            _,
            _,
            _,
            _,
            _,
            <ResumeInitialiser<L> as CudaLineageStoreSampleInitialiser<_, _, O, _>>::DispersalSampler,
            _,
            _,
            _,
            _,
            _,
            <ResumeInitialiser<L> as CudaLineageStoreSampleInitialiser<_, _, O, _>>::ActiveLineageSampler<_, _>,
            _,
            _,
        >>(
            &args,
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
    fn fixup_for_restart<I: Iterator<Item = u64>, L: ExactSizeIterator<Item = Lineage>>(
        args: Self::Arguments,
        rng: G,
        scenario: ScenarioCogs<M, G, O>,
        pre_sampler: OriginPreSampler<M, I>,
        lineages: L,
        restart_at: PositiveF64,
        fixup_strategy: RestartFixUpStrategy,
        local_partition: &mut P,
    ) -> Result<SimulationOutcome<M, G>, ResumeError<Self::Error>> {
        launch::initialise_and_simulate::<_, _, _, _, _, _, _, _, SimulationKernelPtx<
            _,
            _,
            _,
            _,
            _,
            <FixUpInitialiser<L> as CudaLineageStoreSampleInitialiser<_, _, O, _>>::DispersalSampler,
            _,
            _,
            _,
            _,
            _,
            <FixUpInitialiser<L> as CudaLineageStoreSampleInitialiser<_, _, O, _>>::ActiveLineageSampler<_, ConstEventTimeSampler>,
            _,
            _,
        >>(
            &args,
            rng,
            scenario,
            pre_sampler,
            Some(PositiveF64::max_after(restart_at.into(),
        restart_at.into()).into()),     local_partition,
            FixUpInitialiser {
                lineages,
                restart_at,
                fixup_strategy,
            },
        )
    }
}
