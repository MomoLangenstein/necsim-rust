use core::marker::PhantomData;

use necsim_core::{
    cogs::{Habitat, MathsCore},
    lineage::Lineage,
};

use crate::{
    cogs::origin_sampler::{
        pre_sampler::OriginPreSampler, TrustedOriginSampler, UntrustedOriginSampler,
    },
    decomposition::Decomposition,
};

#[expect(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct DecompositionOriginSampler<
    'd,
    M: MathsCore,
    O: UntrustedOriginSampler<'d, M>,
    D: Decomposition<M, O::Habitat>,
> {
    origin_sampler: O,
    decomposition: &'d D,
    _marker: PhantomData<M>,
}

impl<'d, M: MathsCore, O: UntrustedOriginSampler<'d, M>, D: Decomposition<M, O::Habitat>>
    DecompositionOriginSampler<'d, M, O, D>
{
    #[must_use]
    pub fn new(origin_sampler: O, decomposition: &'d D) -> Self {
        Self {
            origin_sampler,
            decomposition,
            _marker: PhantomData::<M>,
        }
    }
}

#[contract_trait]
impl<'d, M: MathsCore, O: UntrustedOriginSampler<'d, M>, D: Decomposition<M, O::Habitat>>
    UntrustedOriginSampler<'d, M> for DecompositionOriginSampler<'d, M, O, D>
{
    type Habitat = O::Habitat;
    type PreSampler = O::PreSampler;

    fn habitat(&self) -> &'d Self::Habitat {
        self.origin_sampler.habitat()
    }

    fn into_pre_sampler(self) -> OriginPreSampler<M, Self::PreSampler> {
        self.origin_sampler.into_pre_sampler()
    }

    fn full_upper_bound_size_hint(&self) -> u64 {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        {
            ((self.origin_sampler.full_upper_bound_size_hint() as f64)
                / f64::from(self.decomposition.get_subdomain().size().get())) as u64
        }
    }
}

unsafe impl<'d, M: MathsCore, O: TrustedOriginSampler<'d, M>, D: Decomposition<M, O::Habitat>>
    TrustedOriginSampler<'d, M> for DecompositionOriginSampler<'d, M, O, D>
{
}

impl<'d, M: MathsCore, O: UntrustedOriginSampler<'d, M>, D: Decomposition<M, O::Habitat>> Iterator
    for DecompositionOriginSampler<'d, M, O, D>
{
    type Item = Lineage;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(lineage) = self.origin_sampler.next() {
            // Forward any out-of-habitat or out-of-deme lineages
            //  (but only on the root subdomain -> no duplication)
            if !self
                .origin_sampler
                .habitat()
                .is_indexed_location_habitable(&lineage.indexed_location)
            {
                if self.decomposition.get_subdomain().is_root() {
                    return Some(lineage);
                }

                continue;
            }

            if self.decomposition.map_location_to_subdomain_rank(
                lineage.indexed_location.location(),
                self.origin_sampler.habitat(),
            ) == self.decomposition.get_subdomain().rank()
            {
                return Some(lineage);
            }
        }

        None
    }
}
