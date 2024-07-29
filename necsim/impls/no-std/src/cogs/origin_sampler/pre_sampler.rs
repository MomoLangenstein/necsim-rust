use core::{
    fmt,
    iter::Empty,
    marker::PhantomData,
    ops::{Deref, DerefMut, RangeFrom},
};

use necsim_core::cogs::MathsCore;
use necsim_core_bond::ClosedUnitF64;
use necsim_partitioning_core::partition::Partition;

const INV_PHI: f64 = 6.180_339_887_498_949e-1_f64;

#[expect(clippy::module_name_repetitions)]
pub struct OriginPreSampler<M: MathsCore, I: Iterator<Item = u64>> {
    inner: I,
    proportion: ClosedUnitF64,
    _marker: PhantomData<M>,
}

impl<M: MathsCore, I: Iterator<Item = u64>> OriginPreSampler<M, I> {
    pub fn get_sample_proportion(&self) -> ClosedUnitF64 {
        self.proportion
    }
}

impl<M: MathsCore, I: Iterator<Item = u64>> Deref for OriginPreSampler<M, I> {
    type Target = I;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<M: MathsCore, I: Iterator<Item = u64>> DerefMut for OriginPreSampler<M, I> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<M: MathsCore, I: Iterator<Item = u64>> fmt::Debug for OriginPreSampler<M, I> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct(stringify!(OriginPreSampler))
            .field("proportion", &self.proportion)
            .finish_non_exhaustive()
    }
}

impl<M: MathsCore> OriginPreSampler<M, RangeFrom<u64>> {
    #[must_use]
    pub fn all() -> Self {
        Self {
            inner: 0..,
            proportion: ClosedUnitF64::one(),
            _marker: PhantomData::<M>,
        }
    }
}

impl<M: MathsCore> OriginPreSampler<M, Empty<u64>> {
    #[must_use]
    pub fn none() -> Self {
        Self {
            inner: core::iter::empty(),
            proportion: ClosedUnitF64::zero(),
            _marker: PhantomData::<M>,
        }
    }
}

impl<M: MathsCore, I: Iterator<Item = u64>> OriginPreSampler<M, I> {
    #[must_use]
    pub fn percentage(
        mut self,
        percentage: ClosedUnitF64,
    ) -> OriginPreSampler<M, impl Iterator<Item = u64>> {
        let inv_geometric_sample_rate = M::ln(1.0_f64 - percentage.get()).recip();

        OriginPreSampler {
            proportion: self.proportion * percentage,
            inner: core::iter::repeat(()).scan(0.5_f64, move |quasi_random, ()| {
                if percentage <= 0.0_f64 {
                    return None;
                }

                if percentage >= 1.0_f64 {
                    return self.next();
                }

                // q = (q + INV_PHI) % 1  where q >= 0
                *quasi_random += INV_PHI;
                *quasi_random -= M::floor(*quasi_random);

                #[expect(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                let skip = M::floor(M::ln(*quasi_random) * inv_geometric_sample_rate) as usize;

                self.nth(skip)
            }),
            _marker: PhantomData::<M>,
        }
    }

    pub fn partition(
        mut self,
        partition: Partition,
    ) -> OriginPreSampler<M, impl Iterator<Item = u64>> {
        let _ = self.advance_by(partition.rank() as usize);

        OriginPreSampler {
            proportion: self.proportion / partition.size().0,
            inner: self.inner.step_by(partition.size().get() as usize),
            _marker: PhantomData::<M>,
        }
    }
}
