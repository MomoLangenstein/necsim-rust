use core::{fmt, iter::Iterator};

use necsim_core::{cogs::MathsCore, lineage::Lineage};

use crate::cogs::{
    lineage_store::coherent::globally::singleton_demes::SingletonDemesHabitat,
    origin_sampler::{pre_sampler::OriginPreSampler, TrustedOriginSampler, UntrustedOriginSampler},
};

pub mod circle;
pub mod downscaled;
pub mod rectangle;

#[expect(clippy::module_name_repetitions)]
pub enum SingletonDemesOriginSampler<
    'h,
    M: MathsCore,
    H: SingletonDemesHabitat<M>,
    I: Iterator<Item = u64>,
> {
    Circle(circle::SingletonDemesCircleOriginSampler<'h, M, H, I>),
    Rectangle(rectangle::SingletonDemesRectangleOriginSampler<'h, M, H, I>),
}

impl<'h, M: MathsCore, H: SingletonDemesHabitat<M>, I: Iterator<Item = u64>> fmt::Debug
    for SingletonDemesOriginSampler<'h, M, H, I>
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Circle(circle) => circle.fmt(fmt),
            Self::Rectangle(rectangle) => rectangle.fmt(fmt),
        }
    }
}

#[contract_trait]
impl<'h, M: MathsCore, H: SingletonDemesHabitat<M>, I: Iterator<Item = u64>>
    UntrustedOriginSampler<'h, M> for SingletonDemesOriginSampler<'h, M, H, I>
{
    type Habitat = H;
    type PreSampler = I;

    fn habitat(&self) -> &'h Self::Habitat {
        match self {
            Self::Circle(circle) => circle.habitat(),
            Self::Rectangle(rectangle) => rectangle.habitat(),
        }
    }

    fn into_pre_sampler(self) -> OriginPreSampler<M, Self::PreSampler> {
        match self {
            Self::Circle(circle) => circle.into_pre_sampler(),
            Self::Rectangle(rectangle) => rectangle.into_pre_sampler(),
        }
    }

    fn full_upper_bound_size_hint(&self) -> u64 {
        match self {
            Self::Circle(circle) => circle.full_upper_bound_size_hint(),
            Self::Rectangle(rectangle) => rectangle.full_upper_bound_size_hint(),
        }
    }
}

unsafe impl<'h, M: MathsCore, H: SingletonDemesHabitat<M>, I: Iterator<Item = u64>>
    TrustedOriginSampler<'h, M> for SingletonDemesOriginSampler<'h, M, H, I>
{
}

impl<'h, M: MathsCore, H: SingletonDemesHabitat<M>, I: Iterator<Item = u64>> Iterator
    for SingletonDemesOriginSampler<'h, M, H, I>
{
    type Item = Lineage;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Circle(circle) => circle.next(),
            Self::Rectangle(rectangle) => rectangle.next(),
        }
    }
}
