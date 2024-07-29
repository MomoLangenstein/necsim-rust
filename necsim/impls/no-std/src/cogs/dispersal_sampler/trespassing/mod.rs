use core::marker::PhantomData;

use necsim_core::{
    cogs::{Backup, DispersalSampler, Habitat, MathsCore, RngCore, SeparableDispersalSampler},
    landscape::Location,
};
use necsim_core_bond::ClosedUnitF64;

pub mod uniform;

#[contract_trait]
pub trait AntiTrespassingDispersalSampler<M: MathsCore, H: Habitat<M>, G: RngCore<M>>:
    Backup + core::fmt::Debug
{
    #[must_use]
    #[debug_requires(!habitat.is_location_habitable(location), "location is inhabitable")]
    #[debug_ensures(old(habitat).is_location_habitable(&ret), "target is habitable")]
    fn sample_anti_trespassing_dispersal_from_location(
        &self,
        location: &Location,
        habitat: &H,
        rng: &mut G,
    ) -> Location;
}

#[expect(clippy::module_name_repetitions)]
#[derive(Debug)]
#[cfg_attr(feature = "cuda", derive(rust_cuda::lend::LendRustToCuda))]
#[cfg_attr(feature = "cuda", cuda(free = "M"))]
pub struct TrespassingDispersalSampler<
    M: MathsCore,
    H: Habitat<M>,
    G: RngCore<M>,
    D: DispersalSampler<M, H, G>,
    T: AntiTrespassingDispersalSampler<M, H, G>,
> {
    #[cfg_attr(feature = "cuda", cuda(embed))]
    legal_dispersal_sampler: D,
    #[cfg_attr(feature = "cuda", cuda(embed))]
    trespassing_dispersal_sampler: T,
    marker: PhantomData<(M, H, G)>,
}

impl<
        M: MathsCore,
        H: Habitat<M>,
        G: RngCore<M>,
        D: DispersalSampler<M, H, G>,
        T: AntiTrespassingDispersalSampler<M, H, G>,
    > TrespassingDispersalSampler<M, H, G, D, T>
{
    #[must_use]
    pub fn new(dispersal_sampler: D, anti_trespassing: T) -> Self {
        Self {
            legal_dispersal_sampler: dispersal_sampler,
            trespassing_dispersal_sampler: anti_trespassing,
            marker: PhantomData::<(M, H, G)>,
        }
    }
}

#[contract_trait]
impl<
        M: MathsCore,
        H: Habitat<M>,
        G: RngCore<M>,
        D: DispersalSampler<M, H, G>,
        T: AntiTrespassingDispersalSampler<M, H, G>,
    > Backup for TrespassingDispersalSampler<M, H, G, D, T>
{
    unsafe fn backup_unchecked(&self) -> Self {
        Self {
            legal_dispersal_sampler: self.legal_dispersal_sampler.backup_unchecked(),
            trespassing_dispersal_sampler: self.trespassing_dispersal_sampler.backup_unchecked(),
            marker: PhantomData::<(M, H, G)>,
        }
    }
}

impl<
        M: MathsCore,
        H: Habitat<M>,
        G: RngCore<M>,
        D: DispersalSampler<M, H, G>,
        T: AntiTrespassingDispersalSampler<M, H, G>,
    > DispersalSampler<M, H, G> for TrespassingDispersalSampler<M, H, G, D, T>
{
    #[must_use]
    #[inline]
    #[debug_ensures(old(habitat).is_location_habitable(&ret), "target is habitable")]
    fn sample_dispersal_from_location(
        &self,
        location: &Location,
        habitat: &H,
        rng: &mut G,
    ) -> Location {
        // Explicitly circumvent the precondition that
        //  `habitat.is_location_habitable(location)`
        self.__contracts_impl_sample_dispersal_from_location(location, habitat, rng)
    }

    #[must_use]
    #[inline]
    fn __contracts_impl_sample_dispersal_from_location(
        &self,
        location: &Location,
        habitat: &H,
        rng: &mut G,
    ) -> Location {
        if habitat.is_location_habitable(location) {
            // Re-establish the precondition that `habitat.is_location_habitable(location)`
            self.legal_dispersal_sampler
                .sample_dispersal_from_location(location, habitat, rng)
        } else {
            // Establish the precondition that `!habitat.is_location_habitable(location)`
            self.trespassing_dispersal_sampler
                .sample_anti_trespassing_dispersal_from_location(location, habitat, rng)
        }
    }
}

impl<
        M: MathsCore,
        H: Habitat<M>,
        G: RngCore<M>,
        D: SeparableDispersalSampler<M, H, G>,
        T: AntiTrespassingDispersalSampler<M, H, G>,
    > SeparableDispersalSampler<M, H, G> for TrespassingDispersalSampler<M, H, G, D, T>
{
    #[must_use]
    #[inline]
    #[debug_ensures(old(habitat).is_location_habitable(&ret), "target is habitable")]
    #[debug_ensures(&ret != location, "disperses to a different location")]
    fn sample_non_self_dispersal_from_location(
        &self,
        location: &Location,
        habitat: &H,
        rng: &mut G,
    ) -> Location {
        // Explicitly circumvent the precondition that
        //  `habitat.is_location_habitable(location)`
        self.__contracts_impl_sample_non_self_dispersal_from_location(location, habitat, rng)
    }

    #[must_use]
    #[inline]
    fn __contracts_impl_sample_non_self_dispersal_from_location(
        &self,
        location: &Location,
        habitat: &H,
        rng: &mut G,
    ) -> Location {
        if habitat.is_location_habitable(location) {
            // Re-establish the precondition that `habitat.is_location_habitable(location)`
            self.legal_dispersal_sampler
                .sample_non_self_dispersal_from_location(location, habitat, rng)
        } else {
            // Establish the precondition that `!habitat.is_location_habitable(location)`
            // Since the origin location is inhabitable but the target location is
            //  habitable, the target location must be different from the origin
            //  location
            self.trespassing_dispersal_sampler
                .sample_anti_trespassing_dispersal_from_location(location, habitat, rng)
        }
    }

    #[must_use]
    #[inline]
    fn get_self_dispersal_probability_at_location(
        &self,
        location: &Location,
        habitat: &H,
    ) -> ClosedUnitF64 {
        // Explicitly circumvent the precondition that
        //  `habitat.is_location_habitable(location)`
        self.__contracts_impl_get_self_dispersal_probability_at_location(location, habitat)
    }

    #[must_use]
    #[inline]
    fn __contracts_impl_get_self_dispersal_probability_at_location(
        &self,
        location: &Location,
        habitat: &H,
    ) -> ClosedUnitF64 {
        if habitat.is_location_habitable(location) {
            // Re-establish the precondition that `habitat.is_location_habitable(location)`
            self.legal_dispersal_sampler
                .get_self_dispersal_probability_at_location(location, habitat)
        } else {
            // The `AntiTrespassingDispersalSampler` always jumps from an inhabitable
            //  location to a habitable location, i.e. there is never any self-dispersal
            //  when starting at an inhabitable location
            ClosedUnitF64::zero()
        }
    }
}
