use core::{marker::PhantomData, num::NonZeroU64};

use necsim_core::{
    cogs::{DispersalSampler, Habitat, MathsCore, RngCore, SeparableDispersalSampler},
    landscape::Location,
};
use necsim_core_bond::ClosedUnitF64;

use crate::cogs::habitat::non_spatial::NonSpatialHabitat;

#[expect(clippy::module_name_repetitions)]
#[derive(Debug)]
#[cfg_attr(feature = "cuda", derive(rust_cuda::lend::LendRustToCuda))]
#[cfg_attr(feature = "cuda", cuda(free = "M", free = "G"))]
pub struct NonSpatialDispersalSampler<M: MathsCore, G: RngCore<M>> {
    marker: PhantomData<(M, G)>,
}

impl<M: MathsCore, G: RngCore<M>> Default for NonSpatialDispersalSampler<M, G> {
    #[must_use]
    fn default() -> Self {
        Self {
            marker: PhantomData::<(M, G)>,
        }
    }
}

impl<M: MathsCore, G: RngCore<M>> Clone for NonSpatialDispersalSampler<M, G> {
    fn clone(&self) -> Self {
        Self {
            marker: PhantomData::<(M, G)>,
        }
    }
}

#[contract_trait]
impl<M: MathsCore, G: RngCore<M>> DispersalSampler<M, NonSpatialHabitat<M>, G>
    for NonSpatialDispersalSampler<M, G>
{
    #[must_use]
    #[inline]
    fn sample_dispersal_from_location(
        &self,
        _location: &Location,
        habitat: &NonSpatialHabitat<M>,
        rng: &mut G,
    ) -> Location {
        use necsim_core::cogs::RngSampler;

        let dispersal_target_index = rng.sample_index_u64(habitat.get_extent().area());

        #[expect(clippy::cast_possible_truncation)]
        Location::new(
            habitat
                .get_extent()
                .origin()
                .x()
                .wrapping_add((dispersal_target_index % habitat.get_extent().width().get()) as u32),
            habitat
                .get_extent()
                .origin()
                .y()
                .wrapping_add((dispersal_target_index / habitat.get_extent().width().get()) as u32),
        )
    }
}

#[contract_trait]
impl<M: MathsCore, G: RngCore<M>> SeparableDispersalSampler<M, NonSpatialHabitat<M>, G>
    for NonSpatialDispersalSampler<M, G>
{
    #[must_use]
    #[debug_requires((
        u64::from(habitat.get_extent().width()) * u64::from(habitat.get_extent().height())
    ) > 1_u64, "a different, non-self dispersal, target location exists")]
    fn sample_non_self_dispersal_from_location(
        &self,
        location: &Location,
        habitat: &NonSpatialHabitat<M>,
        rng: &mut G,
    ) -> Location {
        use necsim_core::cogs::RngSampler;

        let habitat_index_max = habitat.get_extent().area();
        let current_location_index =
            u64::from(location.y()) * habitat.get_extent().width().get() + u64::from(location.x());

        let dispersal_target_index = {
            // Safety: by PRE, `habitat_index_max` > 1
            let dispersal_target_index = rng.sample_index_u64(
                unsafe { NonZeroU64::new_unchecked(habitat_index_max.sub_one()) }.into(),
            );

            if dispersal_target_index >= current_location_index {
                dispersal_target_index + 1
            } else {
                dispersal_target_index
            }
        };

        #[expect(clippy::cast_possible_truncation)]
        Location::new(
            habitat
                .get_extent()
                .origin()
                .x()
                .wrapping_add((dispersal_target_index % habitat.get_extent().width().get()) as u32),
            habitat
                .get_extent()
                .origin()
                .y()
                .wrapping_add((dispersal_target_index / habitat.get_extent().width().get()) as u32),
        )
    }

    #[must_use]
    fn get_self_dispersal_probability_at_location(
        &self,
        _location: &Location,
        habitat: &NonSpatialHabitat<M>,
    ) -> ClosedUnitF64 {
        let self_dispersal = 1.0_f64
            / (f64::from(habitat.get_extent().width()) * f64::from(habitat.get_extent().height()));

        // Safety: Since the method is only called for a valid location,
        //          width >= 1 and height >= 1
        //         => 1.0/(width*height) in [0.0; 1.0]
        unsafe { ClosedUnitF64::new_unchecked(self_dispersal) }
    }
}
