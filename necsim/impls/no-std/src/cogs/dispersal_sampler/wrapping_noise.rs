use necsim_core::{
    cogs::{DispersalSampler, Habitat, MathsCore, RngCore, RngSampler, SeparableDispersalSampler},
    landscape::Location,
};
use necsim_core_bond::{ClosedUnitF64, NonNegativeF64};

use crate::cogs::{
    dispersal_sampler::almost_infinite::normal::AlmostInfiniteNormalDispersalSampler,
    habitat::wrapping_noise::WrappingNoiseHabitat,
};

#[expect(clippy::module_name_repetitions)]
#[derive(Debug)]
#[cfg_attr(feature = "cuda", derive(rust_cuda::lend::LendRustToCuda))]
#[cfg_attr(feature = "cuda", cuda(free = "M"))]
pub struct WrappingNoiseApproximateNormalDispersalSampler<M: MathsCore, G: RngCore<M>> {
    #[cfg_attr(feature = "cuda", cuda(embed))]
    inner: AlmostInfiniteNormalDispersalSampler<M, G>,
}

impl<M: MathsCore, G: RngCore<M>> WrappingNoiseApproximateNormalDispersalSampler<M, G> {
    #[must_use]
    pub fn new(sigma: NonNegativeF64) -> Self {
        Self {
            inner: AlmostInfiniteNormalDispersalSampler::new(sigma),
        }
    }
}

impl<M: MathsCore, G: RngCore<M>> Clone for WrappingNoiseApproximateNormalDispersalSampler<M, G> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

#[contract_trait]
impl<M: MathsCore, G: RngCore<M>> DispersalSampler<M, WrappingNoiseHabitat<M>, G>
    for WrappingNoiseApproximateNormalDispersalSampler<M, G>
{
    #[must_use]
    #[inline]
    fn sample_dispersal_from_location(
        &self,
        location: &Location,
        habitat: &WrappingNoiseHabitat<M>,
        rng: &mut G,
    ) -> Location {
        // This awkward deferral to seperable dispersal sampling is required to
        //  keep both consistent and approximate normal dispersal where some
        //  targets are rejected.
        // If seperable dispersal is not required, this can be implemented as a
        //  direct rejection sampling loop instead.
        if rng.sample_event(self.get_self_dispersal_probability_at_location(location, habitat)) {
            location.clone()
        } else {
            self.sample_non_self_dispersal_from_location(location, habitat, rng)
        }
    }
}

#[contract_trait]
impl<M: MathsCore, G: RngCore<M>> SeparableDispersalSampler<M, WrappingNoiseHabitat<M>, G>
    for WrappingNoiseApproximateNormalDispersalSampler<M, G>
{
    #[must_use]
    fn sample_non_self_dispersal_from_location(
        &self,
        location: &Location,
        habitat: &WrappingNoiseHabitat<M>,
        rng: &mut G,
    ) -> Location {
        let mut target =
            self.inner
                .sample_dispersal_from_location(location, habitat.get_inner(), rng);

        // Rejection sample the normal dispersal kernel
        while habitat.get_habitat_at_location(&target) == 0 || &target == location {
            target = self
                .inner
                .sample_dispersal_from_location(location, habitat.get_inner(), rng);
        }

        target
    }

    #[must_use]
    fn get_self_dispersal_probability_at_location(
        &self,
        location: &Location,
        habitat: &WrappingNoiseHabitat<M>,
    ) -> ClosedUnitF64 {
        // By PRE, the location is habitable, i.e. self-dispersal is possible

        let p_self_dispersal = self
            .inner
            .get_self_dispersal_probability_at_location(location, habitat.get_inner());
        let p_out_dispersal = p_self_dispersal.one_minus() * habitat.coverage();

        // Safety:
        // - p_self_dispersal and p_out_dispersal are both in [0, 1]
        // - a / (a + [0, 1]) = [0, 1]
        unsafe {
            ClosedUnitF64::new_unchecked(
                p_self_dispersal.get() / (p_self_dispersal.get() + p_out_dispersal.get()),
            )
        }
    }
}
