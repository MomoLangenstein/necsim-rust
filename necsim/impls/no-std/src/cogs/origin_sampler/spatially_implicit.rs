use core::{fmt, iter::Iterator};

use necsim_core::{cogs::MathsCore, lineage::Lineage};

use crate::cogs::{
    habitat::spatially_implicit::SpatiallyImplicitHabitat,
    origin_sampler::{non_spatial::NonSpatialOriginSampler, pre_sampler::OriginPreSampler},
};

use super::{TrustedOriginSampler, UntrustedOriginSampler};

#[expect(clippy::module_name_repetitions)]
pub struct SpatiallyImplicitOriginSampler<'h, M: MathsCore, I: Iterator<Item = u64>> {
    local_iterator: NonSpatialOriginSampler<'h, M, I>,
    habitat: &'h SpatiallyImplicitHabitat<M>,
}

impl<'h, M: MathsCore, I: Iterator<Item = u64>> fmt::Debug
    for SpatiallyImplicitOriginSampler<'h, M, I>
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct(stringify!(SpatiallyImplicitOriginSampler))
            .field("local_iterator", &self.local_iterator)
            .field("habitat", &self.habitat)
            .finish()
    }
}

impl<'h, M: MathsCore, I: Iterator<Item = u64>> SpatiallyImplicitOriginSampler<'h, M, I> {
    #[must_use]
    pub fn new(
        pre_sampler: OriginPreSampler<M, I>,
        habitat: &'h SpatiallyImplicitHabitat<M>,
    ) -> Self {
        Self {
            local_iterator: NonSpatialOriginSampler::new(pre_sampler, habitat.local()),
            habitat,
        }
    }
}

#[contract_trait]
impl<'h, M: MathsCore, I: Iterator<Item = u64>> UntrustedOriginSampler<'h, M>
    for SpatiallyImplicitOriginSampler<'h, M, I>
{
    type Habitat = SpatiallyImplicitHabitat<M>;
    type PreSampler = I;

    fn habitat(&self) -> &'h Self::Habitat {
        self.habitat
    }

    fn into_pre_sampler(self) -> OriginPreSampler<M, Self::PreSampler> {
        self.local_iterator.into_pre_sampler()
    }

    fn full_upper_bound_size_hint(&self) -> u64 {
        self.local_iterator.full_upper_bound_size_hint()
    }
}

unsafe impl<'h, M: MathsCore, I: Iterator<Item = u64>> TrustedOriginSampler<'h, M>
    for SpatiallyImplicitOriginSampler<'h, M, I>
{
}

impl<'h, M: MathsCore, I: Iterator<Item = u64>> Iterator
    for SpatiallyImplicitOriginSampler<'h, M, I>
{
    type Item = Lineage;

    fn next(&mut self) -> Option<Self::Item> {
        self.local_iterator.next()
    }
}
