use necsim_core::{
    cogs::{Habitat, MathsCore},
    lineage::Lineage,
};

pub mod decomposition;
pub mod in_memory;
pub mod non_spatial;
pub mod pre_sampler;
pub mod resuming;
pub mod singleton_demes;
pub mod spatially_implicit;

use pre_sampler::OriginPreSampler;

#[contract_trait]
#[expect(clippy::module_name_repetitions)]
/// `Lineage`s produced by the sampler's iterator must have
/// * unique global references
pub trait UntrustedOriginSampler<'h, M: MathsCore>:
    core::fmt::Debug + core::iter::Iterator<Item = Lineage>
{
    type Habitat: 'h + Habitat<M>;
    type PreSampler: Iterator<Item = u64>;

    fn habitat(&self) -> &'h Self::Habitat;

    fn into_pre_sampler(self) -> OriginPreSampler<M, Self::PreSampler>
    where
        Self: Sized;

    fn full_upper_bound_size_hint(&self) -> u64;
}

/// # Safety
/// `Lineage`s produced by the sampler's iterator must have
/// * unique global references
/// * unique indexed locations
/// * valid indexed locations (i.e. inside habitable demes)
#[expect(clippy::module_name_repetitions)]
pub unsafe trait TrustedOriginSampler<'h, M: MathsCore>:
    UntrustedOriginSampler<'h, M>
{
}
