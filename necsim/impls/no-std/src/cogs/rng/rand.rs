use core::{fmt, marker::PhantomData};

use necsim_core::cogs::{Backup, MathsCore, RngCore};

use rand_core::{RngCore as RandRngCore, SeedableRng as RandSeedableRng};
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize, Serializer};

#[expect(clippy::module_name_repetitions)]
#[derive(Clone, TypeLayout)]
#[layout(free = "M")]
#[repr(transparent)]
pub struct RandRng<
    M: MathsCore,
    G: RandRngCore + RandSeedableRng + Clone + Serialize + DeserializeOwned,
> {
    inner: G,
    marker: PhantomData<M>,
}

impl<M: MathsCore, G: RandRngCore + RandSeedableRng + Clone + Serialize + DeserializeOwned> From<G>
    for RandRng<M, G>
{
    fn from(inner: G) -> Self {
        Self {
            inner,
            marker: PhantomData::<M>,
        }
    }
}

impl<M: MathsCore, G: RandRngCore + RandSeedableRng + Clone + Serialize + DeserializeOwned>
    RandRng<M, G>
{
    #[must_use]
    pub fn into_inner(self) -> G {
        self.inner
    }
}

impl<M: MathsCore, G: RandRngCore + RandSeedableRng + Clone + Serialize + DeserializeOwned>
    fmt::Debug for RandRng<M, G>
{
    default fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        struct InnerRng(&'static str);

        impl fmt::Debug for InnerRng {
            fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                fmt.write_str(self.0)
            }
        }

        fmt.debug_tuple("RandRng")
            .field(&InnerRng(core::any::type_name::<G>()))
            .finish()
    }
}

impl<
        M: MathsCore,
        G: RandRngCore + RandSeedableRng + Clone + Serialize + DeserializeOwned + fmt::Debug,
    > fmt::Debug for RandRng<M, G>
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_tuple("RandRng").field(&self.inner).finish()
    }
}

#[contract_trait]
impl<M: MathsCore, G: RandRngCore + RandSeedableRng + Clone + Serialize + DeserializeOwned> Backup
    for RandRng<M, G>
{
    unsafe fn backup_unchecked(&self) -> Self {
        self.clone()
    }
}

impl<M: MathsCore, G: RandRngCore + RandSeedableRng + Clone + Serialize + DeserializeOwned>
    Serialize for RandRng<M, G>
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.inner.serialize(serializer)
    }
}

impl<
        'de,
        M: MathsCore,
        G: RandRngCore + RandSeedableRng + Clone + Serialize + DeserializeOwned,
    > Deserialize<'de> for RandRng<M, G>
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let inner = G::deserialize(deserializer)?;

        Ok(Self {
            inner,
            marker: PhantomData::<M>,
        })
    }
}

impl<
        M: MathsCore,
        G: RandRngCore + RandSeedableRng + Send + Clone + Serialize + DeserializeOwned,
    > RngCore<M> for RandRng<M, G>
{
    type Seed = G::Seed;

    #[must_use]
    #[inline]
    fn from_seed(seed: Self::Seed) -> Self {
        Self {
            inner: G::from_seed(seed),
            marker: PhantomData::<M>,
        }
    }

    #[must_use]
    #[inline]
    fn sample_u64(&mut self) -> u64 {
        self.inner.next_u64()
    }
}
