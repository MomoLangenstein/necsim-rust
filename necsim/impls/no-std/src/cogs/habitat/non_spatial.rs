use core::{marker::PhantomData, num::NonZeroU32};

use necsim_core::{
    cogs::{Habitat, MathsCore, RngCore, UniformlySampleableHabitat},
    landscape::{IndexedLocation, LandscapeExtent, Location},
};
use necsim_core_bond::{OffByOneU32, OffByOneU64};

#[expect(clippy::module_name_repetitions)]
#[derive(Debug)]
#[cfg_attr(feature = "cuda", derive(rust_cuda::lend::LendRustToCuda))]
#[cfg_attr(feature = "cuda", cuda(free = "M"))]
pub struct NonSpatialHabitat<M: MathsCore> {
    #[cfg_attr(feature = "cuda", cuda(embed))]
    extent: LandscapeExtent,
    deme: NonZeroU32,
    marker: PhantomData<M>,
}

impl<M: MathsCore> NonSpatialHabitat<M> {
    #[must_use]
    #[debug_ensures(
        ret.get_total_habitat() == old(
            OffByOneU64::from(area.0) * OffByOneU64::from(area.1) * OffByOneU64::from(deme)
        ),
        "creates a habitat with community size area.0 * area.1 * deme"
    )]
    pub fn new(area: (OffByOneU32, OffByOneU32), deme: NonZeroU32) -> Self {
        Self {
            extent: LandscapeExtent::new(Location::new(0, 0), area.0, area.1),
            deme,
            marker: PhantomData::<M>,
        }
    }

    pub(super) fn new_with_offset(
        offset: Location,
        area: (OffByOneU32, OffByOneU32),
        deme: NonZeroU32,
    ) -> Self {
        Self {
            extent: LandscapeExtent::new(offset, area.0, area.1),
            deme,
            marker: PhantomData::<M>,
        }
    }

    #[must_use]
    pub fn get_deme(&self) -> NonZeroU32 {
        self.deme
    }
}

impl<M: MathsCore> Clone for NonSpatialHabitat<M> {
    fn clone(&self) -> Self {
        Self {
            extent: self.extent.clone(),
            deme: self.deme,
            marker: PhantomData::<M>,
        }
    }
}

#[contract_trait]
impl<M: MathsCore> Habitat<M> for NonSpatialHabitat<M> {
    type LocationIterator<'a> = impl Iterator<Item = Location>;

    #[must_use]
    fn is_finite(&self) -> bool {
        true
    }

    #[must_use]
    fn get_extent(&self) -> &LandscapeExtent {
        &self.extent
    }

    #[must_use]
    fn get_total_habitat(&self) -> OffByOneU64 {
        OffByOneU64::from(self.extent.width())
            * OffByOneU64::from(self.extent.height())
            * OffByOneU64::from(self.deme)
    }

    #[must_use]
    fn get_habitat_at_location(&self, _location: &Location) -> u32 {
        self.deme.get()
    }

    #[must_use]
    fn map_indexed_location_to_u64_injective(&self, indexed_location: &IndexedLocation) -> u64 {
        (u64::from(
            indexed_location
                .location()
                .y()
                .wrapping_sub(self.extent.origin().y()),
        ) * u64::from(self.extent.width())
            + u64::from(
                indexed_location
                    .location()
                    .x()
                    .wrapping_sub(self.extent.origin().x()),
            ))
            * u64::from(self.deme.get())
            + u64::from(indexed_location.index())
    }

    #[must_use]
    fn iter_habitable_locations(&self) -> Self::LocationIterator<'_> {
        self.get_extent().iter()
    }
}

#[contract_trait]
impl<M: MathsCore, G: RngCore<M>> UniformlySampleableHabitat<M, G> for NonSpatialHabitat<M> {
    #[must_use]
    #[inline]
    fn sample_habitable_indexed_location(&self, rng: &mut G) -> IndexedLocation {
        use necsim_core::cogs::RngSampler;

        let mut dispersal_target_index = rng.sample_index_u64(self.get_total_habitat());
        #[expect(clippy::cast_possible_truncation)]
        let index = (dispersal_target_index % u64::from(self.deme.get())) as u32;
        dispersal_target_index /= u64::from(self.deme.get());

        #[expect(clippy::cast_possible_truncation)]
        IndexedLocation::new(
            Location::new(
                self.extent
                    .origin()
                    .x()
                    .wrapping_add((dispersal_target_index % self.extent.width().get()) as u32),
                self.extent
                    .origin()
                    .y()
                    .wrapping_add((dispersal_target_index / self.extent.width().get()) as u32),
            ),
            index,
        )
    }
}
