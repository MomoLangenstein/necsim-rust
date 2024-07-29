#![deny(clippy::pedantic)]
#![feature(never_type)]

#[allow(unused_imports)] // FIXME: use expect
#[macro_use]
extern crate log;

use std::marker::PhantomData;

use necsim_core::cogs::{
    DispersalSampler, LineageStore, MathsCore, RngCore, SpeciationProbability, TurnoverRate,
    UniformlySampleableHabitat,
};
use necsim_core_bond::OpenClosedUnitF64 as PositiveUnitF64;
use necsim_partitioning_core::partition::Partition;

use necsim_impls_no_std::{
    cogs::origin_sampler::{pre_sampler::OriginPreSampler, TrustedOriginSampler},
    decomposition::Decomposition,
};

#[cfg(any(
    feature = "almost-infinite-normal-dispersal",
    feature = "almost-infinite-clark2dt-dispersal",
))]
pub mod almost_infinite;
#[cfg(feature = "non-spatial")]
pub mod non_spatial;
#[cfg(any(
    feature = "spatially-explicit-uniform-turnover",
    feature = "spatially-explicit-turnover-map"
))]
pub mod spatially_explicit;
#[cfg(feature = "spatially-implicit")]
pub mod spatially_implicit;
#[cfg(feature = "wrapping-noise")]
pub mod wrapping_noise;

pub trait ScenarioParameters {
    type Arguments;
    type Error;
}

pub trait Scenario<M: MathsCore, G: RngCore<M>>: Sized + Send + ScenarioParameters {
    type Habitat: Send + Clone + UniformlySampleableHabitat<M, G>;
    type OriginSampler<'h, I: Iterator<Item = u64>>: TrustedOriginSampler<
        'h,
        M,
        Habitat = Self::Habitat,
    >
    where
        M: 'h,
        G: 'h,
        Self: 'h;
    type OriginSamplerAuxiliary: Send + Clone;
    type Decomposition: Decomposition<M, Self::Habitat>;
    type DecompositionAuxiliary: Send + Clone;
    type LineageStore<L: LineageStore<M, Self::Habitat>>: LineageStore<M, Self::Habitat>;
    type DispersalSampler: Send + Clone + DispersalSampler<M, Self::Habitat, G>;
    type TurnoverRate: Send + Clone + TurnoverRate<M, Self::Habitat>;
    type SpeciationProbability: Send + Clone + SpeciationProbability<M, Self::Habitat>;

    /// # Errors
    ///
    /// Returns a `Self::Error` if creating the scenario failed
    fn new(
        args: Self::Arguments,
        speciation_probability_per_generation: PositiveUnitF64,
    ) -> Result<ScenarioCogs<M, G, Self>, Self::Error>;

    fn sample_habitat<'h, I: Iterator<Item = u64>>(
        habitat: &'h Self::Habitat,
        pre_sampler: OriginPreSampler<M, I>,
        auxiliary: Self::OriginSamplerAuxiliary,
    ) -> Self::OriginSampler<'h, I>
    where
        G: 'h;

    fn decompose(
        habitat: &Self::Habitat,
        subdomain: Partition,
        auxiliary: Self::DecompositionAuxiliary,
    ) -> Self::Decomposition;
}

#[non_exhaustive]
pub struct ScenarioCogs<M: MathsCore, G: RngCore<M>, O: Scenario<M, G>> {
    pub habitat: O::Habitat,
    pub dispersal_sampler: O::DispersalSampler,
    pub turnover_rate: O::TurnoverRate,
    pub speciation_probability: O::SpeciationProbability,
    pub origin_sampler_auxiliary: O::OriginSamplerAuxiliary,
    pub decomposition_auxiliary: O::DecompositionAuxiliary,
    _marker: PhantomData<(M, G, O)>,
}

impl<M: MathsCore, G: RngCore<M>, O: Scenario<M, G>> Clone for ScenarioCogs<M, G, O> {
    fn clone(&self) -> Self {
        Self {
            habitat: self.habitat.clone(),
            dispersal_sampler: self.dispersal_sampler.clone(),
            turnover_rate: self.turnover_rate.clone(),
            speciation_probability: self.speciation_probability.clone(),
            origin_sampler_auxiliary: self.origin_sampler_auxiliary.clone(),
            decomposition_auxiliary: self.decomposition_auxiliary.clone(),
            _marker: PhantomData::<(M, G, O)>,
        }
    }
}
