use core::{fmt, iter::Iterator};

use necsim_core::{
    cogs::MathsCore,
    landscape::{IndexedLocation, LandscapeExtent, Location, LocationIterator},
    lineage::Lineage,
};
use necsim_core_bond::OffByOneU32;

use crate::cogs::{
    lineage_store::coherent::globally::singleton_demes::SingletonDemesHabitat,
    origin_sampler::{pre_sampler::OriginPreSampler, TrustedOriginSampler, UntrustedOriginSampler},
};

pub struct SingletonDemesCircleOriginSampler<
    'h,
    M: MathsCore,
    H: SingletonDemesHabitat<M>,
    I: Iterator<Item = u64>,
> {
    pre_sampler: OriginPreSampler<M, I>,
    last_index: u64,
    location_iterator: LocationIterator,
    centre: Location,
    radius_squared: u64,
    upper_bound_size_hint: u64,
    habitat: &'h H,
}

impl<'h, M: MathsCore, H: SingletonDemesHabitat<M>, I: Iterator<Item = u64>> fmt::Debug
    for SingletonDemesCircleOriginSampler<'h, M, H, I>
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct(stringify!(SingletonDemesCircleOriginSampler))
            .field("pre_sampler", &self.pre_sampler)
            .field("last_index", &self.last_index)
            .field("location_iterator", &self.location_iterator)
            .field("centre", &self.centre)
            .field("radius_squared", &self.radius_squared)
            .field("upper_bound_size_hint", &self.upper_bound_size_hint)
            .field("habitat", &self.habitat)
            .finish()
    }
}

impl<'h, M: MathsCore, H: SingletonDemesHabitat<M>, I: Iterator<Item = u64>>
    SingletonDemesCircleOriginSampler<'h, M, H, I>
{
    #[must_use]
    pub fn new(
        pre_sampler: OriginPreSampler<M, I>,
        habitat: &'h H,
        centre: Location,
        radius: u16,
    ) -> Self {
        // Safety: safe since lower and upper bound are both safe
        //  a) radius = 0 --> 0*2 + 1 = 1 --> ok
        //  b) radius = u16::MAX --> u16::MAX*2 + 1 <= u32::MAX --> ok
        let diameter = unsafe { OffByOneU32::new_unchecked(u64::from(radius) * 2 + 1) };

        let sample_extent = LandscapeExtent::new(
            Location::new(
                centre.x().wrapping_sub(u32::from(radius)),
                centre.y().wrapping_sub(u32::from(radius)),
            ),
            diameter,
            diameter,
        );

        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let upper_bound_size_hint = M::ceil(
            f64::from(radius)
                * f64::from(radius)
                * core::f64::consts::PI
                * pre_sampler.get_sample_proportion().get(),
        ) as u64;

        Self {
            pre_sampler,
            last_index: 0_u64,
            location_iterator: sample_extent.iter(),
            centre,
            radius_squared: u64::from(radius) * u64::from(radius),
            upper_bound_size_hint,
            habitat,
        }
    }
}

#[contract_trait]
impl<'h, M: MathsCore, H: SingletonDemesHabitat<M>, I: Iterator<Item = u64>>
    UntrustedOriginSampler<'h, M> for SingletonDemesCircleOriginSampler<'h, M, H, I>
{
    type Habitat = H;
    type PreSampler = I;

    fn habitat(&self) -> &'h Self::Habitat {
        self.habitat
    }

    fn into_pre_sampler(self) -> OriginPreSampler<M, Self::PreSampler> {
        self.pre_sampler
    }

    fn full_upper_bound_size_hint(&self) -> u64 {
        self.upper_bound_size_hint
    }
}

unsafe impl<'h, M: MathsCore, H: SingletonDemesHabitat<M>, I: Iterator<Item = u64>>
    TrustedOriginSampler<'h, M> for SingletonDemesCircleOriginSampler<'h, M, H, I>
{
}

impl<'h, M: MathsCore, H: SingletonDemesHabitat<M>, I: Iterator<Item = u64>> Iterator
    for SingletonDemesCircleOriginSampler<'h, M, H, I>
{
    type Item = Lineage;

    fn next(&mut self) -> Option<Self::Item> {
        let next_index = self.pre_sampler.next()?;
        let mut index_difference = next_index - self.last_index;
        self.last_index = next_index + 1;

        for next_location in &mut self.location_iterator {
            let dx = u32::min(
                next_location.x().wrapping_sub(self.centre.x()),
                self.centre.x().wrapping_sub(next_location.x()),
            );
            let dy = u32::min(
                next_location.y().wrapping_sub(self.centre.y()),
                self.centre.y().wrapping_sub(next_location.y()),
            );

            let (dx, dy) = (u64::from(dx), u64::from(dy));
            let distance_squared = (dx * dx) + (dy * dy);

            if distance_squared <= self.radius_squared
                && self.habitat.get_habitat_at_location(&next_location) > 0_u32
            {
                if index_difference == 0 {
                    return Some(Lineage::new(
                        IndexedLocation::new(next_location, 0),
                        self.habitat,
                    ));
                }

                index_difference -= 1;
            }
        }

        None
    }
}
