use core::{fmt, iter::ExactSizeIterator};

use necsim_core::{
    cogs::{Habitat, MathsCore},
    lineage::Lineage,
};

use crate::cogs::origin_sampler::{pre_sampler::OriginPreSampler, TrustedOriginSampler};

use super::UntrustedOriginSampler;

#[expect(clippy::module_name_repetitions)]
pub struct ResumingOriginSampler<
    'h,
    M: MathsCore,
    H: Habitat<M>,
    L: ExactSizeIterator<Item = Lineage>,
    I: Iterator<Item = u64>,
> {
    lineage_iterator: L,
    pre_sampler: OriginPreSampler<M, I>,
    last_index: u64,
    habitat: &'h H,
}

impl<
        'h,
        M: MathsCore,
        H: Habitat<M>,
        L: ExactSizeIterator<Item = Lineage>,
        I: Iterator<Item = u64>,
    > fmt::Debug for ResumingOriginSampler<'h, M, H, L, I>
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct(stringify!(ResumingOriginSampler))
            .field("pre_sampler", &self.pre_sampler)
            .field("last_index", &self.last_index)
            .field("habitat", &self.habitat)
            .finish()
    }
}

impl<
        'h,
        M: MathsCore,
        H: Habitat<M>,
        L: ExactSizeIterator<Item = Lineage>,
        I: Iterator<Item = u64>,
    > ResumingOriginSampler<'h, M, H, L, I>
{
    #[must_use]
    pub fn new(habitat: &'h H, pre_sampler: OriginPreSampler<M, I>, lineage_iterator: L) -> Self {
        Self {
            lineage_iterator,
            pre_sampler,
            last_index: 0_u64,
            habitat,
        }
    }
}

#[contract_trait]
impl<
        'h,
        M: MathsCore,
        H: Habitat<M>,
        L: ExactSizeIterator<Item = Lineage>,
        I: Iterator<Item = u64>,
    > UntrustedOriginSampler<'h, M> for ResumingOriginSampler<'h, M, H, L, I>
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
        #[expect(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
        )]
        let upper_bound_size_hint = M::ceil(
            (self.lineage_iterator.len() as f64) * self.pre_sampler.get_sample_proportion().get(),
        ) as u64;

        upper_bound_size_hint
    }
}

impl<
        'h,
        M: MathsCore,
        H: Habitat<M>,
        L: ExactSizeIterator<Item = Lineage>,
        I: Iterator<Item = u64>,
    > !TrustedOriginSampler<'h, M> for ResumingOriginSampler<'h, M, H, L, I>
{
}

impl<
        'h,
        M: MathsCore,
        H: Habitat<M>,
        L: ExactSizeIterator<Item = Lineage>,
        I: Iterator<Item = u64>,
    > Iterator for ResumingOriginSampler<'h, M, H, L, I>
{
    type Item = Lineage;

    fn next(&mut self) -> Option<Self::Item> {
        let next_index = self.pre_sampler.next()?;
        let index_difference = next_index - self.last_index;
        self.last_index = next_index + 1;

        for _ in 0..index_difference {
            self.lineage_iterator.next()?;
        }

        self.lineage_iterator.next()
    }
}
