use core::marker::PhantomData;

use necsim_core::cogs::{MathsCore, PrimeableRng, RngCore};

use const_type_layout::TypeGraphLayout;
use rust_cuda::{
    safety::{PortableBitSemantics, StackOnly},
    utils::adapter::RustToCudaWithPortableBitCloneSemantics,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[allow(clippy::module_name_repetitions)] // FIXME: use expect
#[derive(Debug, Clone, rust_cuda::lend::LendRustToCuda)]
#[cuda(free = "M", free = "R")]
pub struct CudaRng<M: MathsCore, R>
where
    R: RngCore<M> + StackOnly + PortableBitSemantics + TypeGraphLayout,
{
    #[cuda(embed)]
    inner: RustToCudaWithPortableBitCloneSemantics<R>,
    marker: PhantomData<M>,
}

impl<M: MathsCore, R: RngCore<M> + StackOnly + PortableBitSemantics + TypeGraphLayout> From<R>
    for CudaRng<M, R>
{
    #[must_use]
    #[inline]
    fn from(rng: R) -> Self {
        Self {
            inner: rng.into(),
            marker: PhantomData::<M>,
        }
    }
}

impl<M: MathsCore, R: RngCore<M> + StackOnly + PortableBitSemantics + TypeGraphLayout> RngCore<M>
    for CudaRng<M, R>
{
    type Seed = <R as RngCore<M>>::Seed;

    #[must_use]
    #[inline]
    fn from_seed(seed: Self::Seed) -> Self {
        Self {
            inner: R::from_seed(seed).into(),
            marker: PhantomData::<M>,
        }
    }

    #[must_use]
    #[inline]
    fn sample_u64(&mut self) -> u64 {
        self.inner.sample_u64()
    }
}

impl<M: MathsCore, R: PrimeableRng<M> + StackOnly + PortableBitSemantics + TypeGraphLayout>
    PrimeableRng<M> for CudaRng<M, R>
{
    #[inline]
    fn prime_with(&mut self, location_index: u64, time_index: u64) {
        self.inner.prime_with(location_index, time_index);
    }
}

impl<M: MathsCore, R: RngCore<M> + StackOnly + PortableBitSemantics + TypeGraphLayout> Serialize
    for CudaRng<M, R>
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.inner.serialize(serializer)
    }
}

impl<'de, M: MathsCore, R: RngCore<M> + StackOnly + PortableBitSemantics + TypeGraphLayout>
    Deserialize<'de> for CudaRng<M, R>
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let inner = R::deserialize(deserializer)?.into();

        Ok(Self {
            inner,
            marker: PhantomData::<M>,
        })
    }
}
