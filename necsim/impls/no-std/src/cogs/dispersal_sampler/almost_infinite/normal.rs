use core::marker::PhantomData;

use necsim_core::{
    cogs::{DispersalSampler, MathsCore, RngCore, SeparableDispersalSampler},
    landscape::Location,
};
use necsim_core_bond::{ClosedUnitF64, NonNegativeF64};

use crate::cogs::habitat::almost_infinite::AlmostInfiniteHabitat;

#[derive(Debug)]
#[cfg_attr(feature = "cuda", derive(rust_cuda::lend::LendRustToCuda))]
#[cfg_attr(feature = "cuda", cuda(free = "M", free = "G"))]
pub struct AlmostInfiniteNormalDispersalSampler<M: MathsCore, G: RngCore<M>> {
    sigma: NonNegativeF64,
    self_dispersal: ClosedUnitF64,
    marker: PhantomData<(M, G)>,
}

impl<M: MathsCore, G: RngCore<M>> AlmostInfiniteNormalDispersalSampler<M, G> {
    #[must_use]
    pub fn new(sigma: NonNegativeF64) -> Self {
        let self_dispersal_1d = if sigma > 0.0_f64 {
            let probability = libm::erf(0.5 / (sigma.get() * core::f64::consts::SQRT_2));

            // Safety: For non-negative values x (as both sigma and sqrt(2.0) are),
            //         erf(0.5 / x) in [0.0; 1.0]
            unsafe { ClosedUnitF64::new_unchecked(probability) }
        } else {
            ClosedUnitF64::one()
        };

        Self {
            sigma,
            self_dispersal: self_dispersal_1d * self_dispersal_1d,
            marker: PhantomData::<(M, G)>,
        }
    }
}

impl<M: MathsCore, G: RngCore<M>> Clone for AlmostInfiniteNormalDispersalSampler<M, G> {
    fn clone(&self) -> Self {
        Self {
            sigma: self.sigma,
            self_dispersal: self.self_dispersal,
            marker: PhantomData::<(M, G)>,
        }
    }
}

#[contract_trait]
impl<M: MathsCore, G: RngCore<M>> DispersalSampler<M, AlmostInfiniteHabitat<M>, G>
    for AlmostInfiniteNormalDispersalSampler<M, G>
{
    #[must_use]
    fn sample_dispersal_from_location(
        &self,
        location: &Location,
        _habitat: &AlmostInfiniteHabitat<M>,
        rng: &mut G,
    ) -> Location {
        use necsim_core::cogs::RngSampler;

        let (dx, dy): (f64, f64) = rng.sample_2d_normal(0.0_f64, self.sigma);

        AlmostInfiniteHabitat::<M>::clamp_round_dispersal(location, dx, dy)
    }
}

#[contract_trait]
impl<M: MathsCore, G: RngCore<M>> SeparableDispersalSampler<M, AlmostInfiniteHabitat<M>, G>
    for AlmostInfiniteNormalDispersalSampler<M, G>
{
    #[must_use]
    fn sample_non_self_dispersal_from_location(
        &self,
        location: &Location,
        habitat: &AlmostInfiniteHabitat<M>,
        rng: &mut G,
    ) -> Location {
        let mut target_location = self.sample_dispersal_from_location(location, habitat, rng);

        // For now, we just use rejection sampling here
        while &target_location == location {
            target_location = self.sample_dispersal_from_location(location, habitat, rng);
        }

        target_location
    }

    #[must_use]
    fn get_self_dispersal_probability_at_location(
        &self,
        _location: &Location,
        _habitat: &AlmostInfiniteHabitat<M>,
    ) -> ClosedUnitF64 {
        self.self_dispersal
    }
}
