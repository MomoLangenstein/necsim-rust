#![deny(clippy::pedantic)]
#![allow(long_running_const_eval)]
#![recursion_limit = "1024"]

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

mod link;
mod patch;

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
