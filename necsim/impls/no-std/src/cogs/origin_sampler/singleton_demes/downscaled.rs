use core::{fmt, iter::Iterator};

use necsim_core::{
    cogs::MathsCore,
    landscape::{IndexedLocation, Location},
    lineage::Lineage,
};

use crate::cogs::{
    habitat::almost_infinite::{
        downscaled::AlmostInfiniteDownscaledHabitat, AlmostInfiniteHabitat,
    },
    origin_sampler::{pre_sampler::OriginPreSampler, TrustedOriginSampler, UntrustedOriginSampler},
};

pub struct AlmostInfiniteDownscaledOriginSampler<
    'h,
    M: MathsCore,
    O: UntrustedOriginSampler<'h, M, Habitat = AlmostInfiniteHabitat<M>>,
> {
    sampler: O,
    habitat: &'h AlmostInfiniteDownscaledHabitat<M>,
}

impl<'h, M: MathsCore, O: UntrustedOriginSampler<'h, M, Habitat = AlmostInfiniteHabitat<M>>>
    AlmostInfiniteDownscaledOriginSampler<'h, M, O>
{
    #[must_use]
    pub fn new(sampler: O, habitat: &'h AlmostInfiniteDownscaledHabitat<M>) -> Self {
        Self { sampler, habitat }
    }
}

impl<'h, M: MathsCore, O: UntrustedOriginSampler<'h, M, Habitat = AlmostInfiniteHabitat<M>>>
    fmt::Debug for AlmostInfiniteDownscaledOriginSampler<'h, M, O>
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct(stringify!(AlmostInfiniteDownscaledOriginSampler))
            .field("sampler", &self.sampler)
            .field("habitat", &self.habitat)
            .finish()
    }
}

#[contract_trait]
impl<'h, M: MathsCore, O: UntrustedOriginSampler<'h, M, Habitat = AlmostInfiniteHabitat<M>>>
    UntrustedOriginSampler<'h, M> for AlmostInfiniteDownscaledOriginSampler<'h, M, O>
{
    type Habitat = AlmostInfiniteDownscaledHabitat<M>;
    type PreSampler = O::PreSampler;

    fn habitat(&self) -> &'h Self::Habitat {
        self.habitat
    }

    fn into_pre_sampler(self) -> OriginPreSampler<M, Self::PreSampler> {
        self.sampler.into_pre_sampler()
    }

    fn full_upper_bound_size_hint(&self) -> u64 {
        self.sampler.full_upper_bound_size_hint()
    }
}

unsafe impl<'h, M: MathsCore, O: TrustedOriginSampler<'h, M, Habitat = AlmostInfiniteHabitat<M>>>
    TrustedOriginSampler<'h, M> for AlmostInfiniteDownscaledOriginSampler<'h, M, O>
{
}

impl<'h, M: MathsCore, O: UntrustedOriginSampler<'h, M, Habitat = AlmostInfiniteHabitat<M>>>
    Iterator for AlmostInfiniteDownscaledOriginSampler<'h, M, O>
{
    type Item = Lineage;

    fn next(&mut self) -> Option<Self::Item> {
        let location = self.sampler.next()?.indexed_location;

        let index_x = location.location().x() % (self.habitat.downscale_x() as u32);
        let index_y = location.location().y() % (self.habitat.downscale_y() as u32);

        Some(Lineage::new(
            IndexedLocation::new(
                Location::new(
                    location.location().x() - index_x,
                    location.location().y() - index_y,
                ),
                index_y * (self.habitat.downscale_x() as u32) + index_x,
            ),
            self.habitat,
        ))
    }
}
