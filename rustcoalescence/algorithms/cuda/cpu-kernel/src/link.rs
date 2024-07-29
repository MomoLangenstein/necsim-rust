use necsim_core::{
    cogs::{
        CoalescenceSampler, DispersalSampler, EmigrationExit, Habitat, ImmigrationEntry,
        LineageStore, MathsCore, PrimeableRng, SpeciationProbability, TurnoverRate,
    },
    reporter::boolean::Boolean,
};

use necsim_impls_no_std::cogs::{
    active_lineage_sampler::singular::SingularActiveLineageSampler,
    event_sampler::tracking::MinSpeciationTrackingEventSampler,
};

use rust_cuda::lend::RustToCuda;

#[expect(clippy::type_complexity)]
pub struct SimulationKernelPtx<
    M: MathsCore + Sync,
    H: Habitat<M> + RustToCuda + Sync,
    G: PrimeableRng<M> + RustToCuda + Sync,
    S: LineageStore<M, H> + RustToCuda + Sync,
    X: EmigrationExit<M, H, G, S> + RustToCuda + Sync,
    D: DispersalSampler<M, H, G> + RustToCuda + Sync,
    C: CoalescenceSampler<M, H, S> + RustToCuda + Sync,
    T: TurnoverRate<M, H> + RustToCuda + Sync,
    N: SpeciationProbability<M, H> + RustToCuda + Sync,
    E: MinSpeciationTrackingEventSampler<M, H, G, S, X, D, C, T, N> + RustToCuda + Sync,
    I: ImmigrationEntry<M> + RustToCuda + Sync,
    A: SingularActiveLineageSampler<M, H, G, S, X, D, C, T, N, E, I> + RustToCuda + Sync,
    ReportSpeciation: Boolean,
    ReportDispersal: Boolean,
>(
    std::marker::PhantomData<(
        M,
        H,
        G,
        S,
        X,
        D,
        C,
        T,
        N,
        E,
        I,
        A,
        ReportSpeciation,
        ReportDispersal,
    )>,
);

#[allow(unused_macros)] // FIXME: use expect
macro_rules! link_kernel {
    ($habitat:ty, $dispersal:ty, $turnover:ty, $speciation:ty) => {
        link_kernel! {
            $habitat, $dispersal, $turnover, $speciation,
            necsim_core::reporter::boolean::False,
            necsim_core::reporter::boolean::False
        }
        link_kernel! {
            $habitat, $dispersal, $turnover, $speciation,
            necsim_core::reporter::boolean::False,
            necsim_core::reporter::boolean::True
        }
        link_kernel! {
            $habitat, $dispersal, $turnover, $speciation,
            necsim_core::reporter::boolean::True,
            necsim_core::reporter::boolean::False
        }
        link_kernel! {
            $habitat, $dispersal, $turnover, $speciation,
            necsim_core::reporter::boolean::True,
            necsim_core::reporter::boolean::True
        }
    };
    (
        $habitat:ty, $dispersal:ty, $turnover:ty, $speciation:ty,
        $report_speciation:ty, $report_dispersal:ty
    ) => {
        rustcoalescence_algorithms_cuda_gpu_kernel::link! { impl simulate<
            necsim_impls_cuda::cogs::maths::NvptxMathsCore,
            $habitat,
            necsim_impls_cuda::cogs::rng::CudaRng<
                necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                necsim_impls_no_std::cogs::rng::wyhash::WyHash<
                    necsim_impls_cuda::cogs::maths::NvptxMathsCore
                >,
            >,
            necsim_impls_no_std::cogs::lineage_store::independent::IndependentLineageStore<
                necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                $habitat,
            >,
            necsim_impls_no_std::cogs::emigration_exit::never::NeverEmigrationExit,
            $dispersal,
            necsim_impls_no_std::cogs::coalescence_sampler::independent::IndependentCoalescenceSampler<
                necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                $habitat,
            >,
            $turnover,
            $speciation,
            necsim_impls_no_std::cogs::event_sampler::independent::IndependentEventSampler<
                necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                $habitat,
                necsim_impls_cuda::cogs::rng::CudaRng<
                    necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                    necsim_impls_no_std::cogs::rng::wyhash::WyHash<
                        necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                    >,
                >,
                necsim_impls_no_std::cogs::emigration_exit::never::NeverEmigrationExit,
                $dispersal,
                $turnover,
                $speciation,
            >,
            necsim_impls_no_std::cogs::immigration_entry::never::NeverImmigrationEntry,
            necsim_impls_no_std::cogs::active_lineage_sampler::independent::IndependentActiveLineageSampler<
                necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                $habitat,
                necsim_impls_cuda::cogs::rng::CudaRng<
                    necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                    necsim_impls_no_std::cogs::rng::wyhash::WyHash<
                        necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                    >,
                >,
                necsim_impls_no_std::cogs::emigration_exit::never::NeverEmigrationExit,
                $dispersal,
                $turnover,
                $speciation,
                necsim_impls_no_std::cogs::active_lineage_sampler::independent::event_time_sampler::exp::ExpEventTimeSampler,
            >,
            $report_speciation,
            $report_dispersal,
        > for SimulationKernelPtx }

        rustcoalescence_algorithms_cuda_gpu_kernel::link! { impl simulate<
            necsim_impls_cuda::cogs::maths::NvptxMathsCore,
            $habitat,
            necsim_impls_cuda::cogs::rng::CudaRng<
                necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                necsim_impls_no_std::cogs::rng::wyhash::WyHash<
                    necsim_impls_cuda::cogs::maths::NvptxMathsCore
                >,
            >,
            necsim_impls_no_std::cogs::lineage_store::independent::IndependentLineageStore<
                necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                $habitat,
            >,
            necsim_impls_no_std::cogs::emigration_exit::never::NeverEmigrationExit,
            necsim_impls_no_std::cogs::dispersal_sampler::trespassing::TrespassingDispersalSampler<
                necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                $habitat,
                necsim_impls_cuda::cogs::rng::CudaRng<
                    necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                    necsim_impls_no_std::cogs::rng::wyhash::WyHash<
                        necsim_impls_cuda::cogs::maths::NvptxMathsCore
                    >,
                >,
                $dispersal,
                necsim_impls_no_std::cogs::dispersal_sampler::trespassing::uniform::UniformAntiTrespassingDispersalSampler<
                    necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                    $habitat,
                    necsim_impls_cuda::cogs::rng::CudaRng<
                        necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                        necsim_impls_no_std::cogs::rng::wyhash::WyHash<
                            necsim_impls_cuda::cogs::maths::NvptxMathsCore
                        >,
                    >,
                >,
            >,
            necsim_impls_no_std::cogs::coalescence_sampler::independent::IndependentCoalescenceSampler<
                necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                $habitat,
            >,
            $turnover,
            $speciation,
            necsim_impls_no_std::cogs::event_sampler::independent::IndependentEventSampler<
                necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                $habitat,
                necsim_impls_cuda::cogs::rng::CudaRng<
                    necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                    necsim_impls_no_std::cogs::rng::wyhash::WyHash<
                        necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                    >,
                >,
                necsim_impls_no_std::cogs::emigration_exit::never::NeverEmigrationExit,
                necsim_impls_no_std::cogs::dispersal_sampler::trespassing::TrespassingDispersalSampler<
                    necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                    $habitat,
                    necsim_impls_cuda::cogs::rng::CudaRng<
                        necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                        necsim_impls_no_std::cogs::rng::wyhash::WyHash<
                            necsim_impls_cuda::cogs::maths::NvptxMathsCore
                        >,
                    >,
                    $dispersal,
                    necsim_impls_no_std::cogs::dispersal_sampler::trespassing::uniform::UniformAntiTrespassingDispersalSampler<
                        necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                        $habitat,
                        necsim_impls_cuda::cogs::rng::CudaRng<
                            necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                            necsim_impls_no_std::cogs::rng::wyhash::WyHash<
                                necsim_impls_cuda::cogs::maths::NvptxMathsCore
                            >,
                        >,
                    >,
                >,
                $turnover,
                $speciation,
            >,
            necsim_impls_no_std::cogs::immigration_entry::never::NeverImmigrationEntry,
            necsim_impls_no_std::cogs::active_lineage_sampler::independent::IndependentActiveLineageSampler<
                necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                $habitat,
                necsim_impls_cuda::cogs::rng::CudaRng<
                    necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                    necsim_impls_no_std::cogs::rng::wyhash::WyHash<
                        necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                    >,
                >,
                necsim_impls_no_std::cogs::emigration_exit::never::NeverEmigrationExit,
                necsim_impls_no_std::cogs::dispersal_sampler::trespassing::TrespassingDispersalSampler<
                    necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                    $habitat,
                    necsim_impls_cuda::cogs::rng::CudaRng<
                        necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                        necsim_impls_no_std::cogs::rng::wyhash::WyHash<
                            necsim_impls_cuda::cogs::maths::NvptxMathsCore
                        >,
                    >,
                    $dispersal,
                    necsim_impls_no_std::cogs::dispersal_sampler::trespassing::uniform::UniformAntiTrespassingDispersalSampler<
                        necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                        $habitat,
                        necsim_impls_cuda::cogs::rng::CudaRng<
                            necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                            necsim_impls_no_std::cogs::rng::wyhash::WyHash<
                                necsim_impls_cuda::cogs::maths::NvptxMathsCore
                            >,
                        >,
                    >,
                >,
                $turnover,
                $speciation,
                necsim_impls_no_std::cogs::active_lineage_sampler::independent::event_time_sampler::r#const::ConstEventTimeSampler,
            >,
            $report_speciation,
            $report_dispersal,
        > for SimulationKernelPtx }
    };
}

#[cfg(feature = "non-spatial-scenario")]
link_kernel!(
    necsim_impls_no_std::cogs::habitat::non_spatial::NonSpatialHabitat<
        necsim_impls_cuda::cogs::maths::NvptxMathsCore
    >,
    necsim_impls_no_std::cogs::dispersal_sampler::non_spatial::NonSpatialDispersalSampler<
        necsim_impls_cuda::cogs::maths::NvptxMathsCore,
        necsim_impls_cuda::cogs::rng::CudaRng<
            necsim_impls_cuda::cogs::maths::NvptxMathsCore,
            necsim_impls_no_std::cogs::rng::wyhash::WyHash<
                necsim_impls_cuda::cogs::maths::NvptxMathsCore
            >,
        >,
    >,
    necsim_impls_no_std::cogs::turnover_rate::uniform::UniformTurnoverRate,
    necsim_impls_no_std::cogs::speciation_probability::uniform::UniformSpeciationProbability
);

#[cfg(feature = "spatially-implicit-scenario")]
link_kernel!(
    necsim_impls_no_std::cogs::habitat::spatially_implicit::SpatiallyImplicitHabitat<
        necsim_impls_cuda::cogs::maths::NvptxMathsCore
    >,
    necsim_impls_no_std::cogs::dispersal_sampler::spatially_implicit::SpatiallyImplicitDispersalSampler<
        necsim_impls_cuda::cogs::maths::NvptxMathsCore,
        necsim_impls_cuda::cogs::rng::CudaRng<
            necsim_impls_cuda::cogs::maths::NvptxMathsCore,
            necsim_impls_no_std::cogs::rng::wyhash::WyHash<
                necsim_impls_cuda::cogs::maths::NvptxMathsCore
            >,
        >,
    >,
    necsim_impls_no_std::cogs::turnover_rate::uniform::UniformTurnoverRate,
    necsim_impls_no_std::cogs::speciation_probability::spatially_implicit::SpatiallyImplicitSpeciationProbability
);

#[cfg(feature = "almost-infinite-normal-dispersal-scenario")]
link_kernel!(
    necsim_impls_no_std::cogs::habitat::almost_infinite::AlmostInfiniteHabitat<
        necsim_impls_cuda::cogs::maths::NvptxMathsCore
    >,
    necsim_impls_no_std::cogs::dispersal_sampler::almost_infinite::normal::AlmostInfiniteNormalDispersalSampler<
        necsim_impls_cuda::cogs::maths::NvptxMathsCore,
        necsim_impls_cuda::cogs::rng::CudaRng<
            necsim_impls_cuda::cogs::maths::NvptxMathsCore,
            necsim_impls_no_std::cogs::rng::wyhash::WyHash<
                necsim_impls_cuda::cogs::maths::NvptxMathsCore
            >,
        >,
    >,
    necsim_impls_no_std::cogs::turnover_rate::uniform::UniformTurnoverRate,
    necsim_impls_no_std::cogs::speciation_probability::uniform::UniformSpeciationProbability
);

#[cfg(feature = "almost-infinite-clark2dt-dispersal-scenario")]
link_kernel!(
    necsim_impls_no_std::cogs::habitat::almost_infinite::AlmostInfiniteHabitat<
        necsim_impls_cuda::cogs::maths::NvptxMathsCore
    >,
    necsim_impls_no_std::cogs::dispersal_sampler::almost_infinite::clark2dt::AlmostInfiniteClark2DtDispersalSampler<
        necsim_impls_cuda::cogs::maths::NvptxMathsCore,
        necsim_impls_cuda::cogs::rng::CudaRng<
            necsim_impls_cuda::cogs::maths::NvptxMathsCore,
            necsim_impls_no_std::cogs::rng::wyhash::WyHash<
                necsim_impls_cuda::cogs::maths::NvptxMathsCore
            >,
        >,
    >,
    necsim_impls_no_std::cogs::turnover_rate::uniform::UniformTurnoverRate,
    necsim_impls_no_std::cogs::speciation_probability::uniform::UniformSpeciationProbability
);

#[cfg(all(
    feature = "almost-infinite-normal-dispersal-scenario",
    feature = "almost-infinite-downscaled-scenario",
))]
link_kernel!(
    necsim_impls_no_std::cogs::habitat::almost_infinite::downscaled::AlmostInfiniteDownscaledHabitat<
        necsim_impls_cuda::cogs::maths::NvptxMathsCore
    >,
    necsim_impls_no_std::cogs::dispersal_sampler::almost_infinite::downscaled::AlmostInfiniteDownscaledDispersalSampler<
        necsim_impls_cuda::cogs::maths::NvptxMathsCore,
        necsim_impls_cuda::cogs::rng::CudaRng<
            necsim_impls_cuda::cogs::maths::NvptxMathsCore,
            necsim_impls_no_std::cogs::rng::wyhash::WyHash<
                necsim_impls_cuda::cogs::maths::NvptxMathsCore
            >,
        >,
        necsim_impls_no_std::cogs::dispersal_sampler::almost_infinite::normal::AlmostInfiniteNormalDispersalSampler<
            necsim_impls_cuda::cogs::maths::NvptxMathsCore,
            necsim_impls_cuda::cogs::rng::CudaRng<
                necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                necsim_impls_no_std::cogs::rng::wyhash::WyHash<
                    necsim_impls_cuda::cogs::maths::NvptxMathsCore
                >,
            >,
        >,
    >,
    necsim_impls_no_std::cogs::turnover_rate::uniform::UniformTurnoverRate,
    necsim_impls_no_std::cogs::speciation_probability::uniform::UniformSpeciationProbability
);

#[cfg(all(
    feature = "almost-infinite-clark2dt-dispersal-scenario",
    feature = "almost-infinite-downscaled-scenario",
))]
link_kernel!(
    necsim_impls_no_std::cogs::habitat::almost_infinite::downscaled::AlmostInfiniteDownscaledHabitat<
        necsim_impls_cuda::cogs::maths::NvptxMathsCore
    >,
    necsim_impls_no_std::cogs::dispersal_sampler::almost_infinite::downscaled::AlmostInfiniteDownscaledDispersalSampler<
        necsim_impls_cuda::cogs::maths::NvptxMathsCore,
        necsim_impls_cuda::cogs::rng::CudaRng<
            necsim_impls_cuda::cogs::maths::NvptxMathsCore,
            necsim_impls_no_std::cogs::rng::wyhash::WyHash<
                necsim_impls_cuda::cogs::maths::NvptxMathsCore
            >,
        >,
        necsim_impls_no_std::cogs::dispersal_sampler::almost_infinite::clark2dt::AlmostInfiniteClark2DtDispersalSampler<
            necsim_impls_cuda::cogs::maths::NvptxMathsCore,
            necsim_impls_cuda::cogs::rng::CudaRng<
                necsim_impls_cuda::cogs::maths::NvptxMathsCore,
                necsim_impls_no_std::cogs::rng::wyhash::WyHash<
                    necsim_impls_cuda::cogs::maths::NvptxMathsCore
                >,
            >,
        >,
    >,
    necsim_impls_no_std::cogs::turnover_rate::uniform::UniformTurnoverRate,
    necsim_impls_no_std::cogs::speciation_probability::uniform::UniformSpeciationProbability
);

#[cfg(feature = "spatially-explicit-uniform-turnover-scenario")]
link_kernel!(
    necsim_impls_no_std::cogs::habitat::in_memory::InMemoryHabitat<
        necsim_impls_cuda::cogs::maths::NvptxMathsCore
    >,
    necsim_impls_no_std::cogs::dispersal_sampler::in_memory::packed_separable_alias::InMemoryPackedSeparableAliasDispersalSampler<
        necsim_impls_cuda::cogs::maths::NvptxMathsCore,
        necsim_impls_no_std::cogs::habitat::in_memory::InMemoryHabitat<
            necsim_impls_cuda::cogs::maths::NvptxMathsCore
        >,
        necsim_impls_cuda::cogs::rng::CudaRng<
            necsim_impls_cuda::cogs::maths::NvptxMathsCore,
            necsim_impls_no_std::cogs::rng::wyhash::WyHash<
                necsim_impls_cuda::cogs::maths::NvptxMathsCore
            >,
        >,
    >,
    necsim_impls_no_std::cogs::turnover_rate::uniform::UniformTurnoverRate,
    necsim_impls_no_std::cogs::speciation_probability::uniform::UniformSpeciationProbability
);

#[cfg(feature = "spatially-explicit-turnover-map-scenario")]
link_kernel!(
    necsim_impls_no_std::cogs::habitat::in_memory::InMemoryHabitat<
        necsim_impls_cuda::cogs::maths::NvptxMathsCore
    >,
    necsim_impls_no_std::cogs::dispersal_sampler::in_memory::packed_separable_alias::InMemoryPackedSeparableAliasDispersalSampler<
        necsim_impls_cuda::cogs::maths::NvptxMathsCore,
        necsim_impls_no_std::cogs::habitat::in_memory::InMemoryHabitat<
            necsim_impls_cuda::cogs::maths::NvptxMathsCore
        >,
        necsim_impls_cuda::cogs::rng::CudaRng<
            necsim_impls_cuda::cogs::maths::NvptxMathsCore,
            necsim_impls_no_std::cogs::rng::wyhash::WyHash<
                necsim_impls_cuda::cogs::maths::NvptxMathsCore
            >,
        >,
    >,
    necsim_impls_no_std::cogs::turnover_rate::in_memory::InMemoryTurnoverRate,
    necsim_impls_no_std::cogs::speciation_probability::uniform::UniformSpeciationProbability
);

#[cfg(feature = "wrapping-noise-scenario")]
link_kernel!(
    necsim_impls_no_std::cogs::habitat::wrapping_noise::WrappingNoiseHabitat<
        necsim_impls_cuda::cogs::maths::NvptxMathsCore
    >,
    necsim_impls_no_std::cogs::dispersal_sampler::wrapping_noise::WrappingNoiseApproximateNormalDispersalSampler<
        necsim_impls_cuda::cogs::maths::NvptxMathsCore,
        necsim_impls_cuda::cogs::rng::CudaRng<
            necsim_impls_cuda::cogs::maths::NvptxMathsCore,
            necsim_impls_no_std::cogs::rng::wyhash::WyHash<
                necsim_impls_cuda::cogs::maths::NvptxMathsCore
            >,
        >,
    >,
    necsim_impls_no_std::cogs::turnover_rate::uniform::UniformTurnoverRate,
    necsim_impls_no_std::cogs::speciation_probability::uniform::UniformSpeciationProbability
);
