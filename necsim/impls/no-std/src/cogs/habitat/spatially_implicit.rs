use core::num::NonZeroU32;

use necsim_core::{
    cogs::{Habitat, MathsCore, RngCore, UniformlySampleableHabitat},
    landscape::{IndexedLocation, LandscapeExtent, Location},
};
use necsim_core_bond::{OffByOneU32, OffByOneU64};

use crate::cogs::habitat::non_spatial::NonSpatialHabitat;

const SPATIALLY_IMPLICIT_EXTENT: LandscapeExtent =
    LandscapeExtent::new(Location::new(0, 0), OffByOneU32::max(), OffByOneU32::max());

#[expect(clippy::module_name_repetitions)]
#[derive(Debug)]
#[cfg_attr(feature = "cuda", derive(rust_cuda::lend::LendRustToCuda))]
#[cfg_attr(feature = "cuda", cuda(free = "M"))]
pub struct SpatiallyImplicitHabitat<M: MathsCore> {
    #[cfg_attr(feature = "cuda", cuda(embed))]
    local: NonSpatialHabitat<M>,
    #[cfg_attr(feature = "cuda", cuda(embed))]
    meta: NonSpatialHabitat<M>,
}

impl<M: MathsCore> SpatiallyImplicitHabitat<M> {
    #[must_use]
    #[debug_ensures(
        ret.get_total_habitat() == old(
            OffByOneU64::from(local_area.0)
                * OffByOneU64::from(local_area.1)
                * OffByOneU64::from(local_deme)
            + OffByOneU64::from(meta_area.0)
                * OffByOneU64::from(meta_area.1)
                * OffByOneU64::from(meta_deme)
        ),
        "creates a habitat with a combined local and meta community size "
    )]
    pub fn new(
        local_area: (OffByOneU32, OffByOneU32),
        local_deme: NonZeroU32,
        meta_area: (OffByOneU32, OffByOneU32),
        meta_deme: NonZeroU32,
    ) -> Self {
        let local = NonSpatialHabitat::new(local_area, local_deme);
        let meta = NonSpatialHabitat::new_with_offset(
            Location::new(meta_area.0.inv(), meta_area.1.inv()),
            meta_area,
            meta_deme,
        );

        Self { local, meta }
    }

    #[must_use]
    pub fn local(&self) -> &NonSpatialHabitat<M> {
        &self.local
    }

    #[must_use]
    pub fn meta(&self) -> &NonSpatialHabitat<M> {
        &self.meta
    }
}

impl<M: MathsCore> Clone for SpatiallyImplicitHabitat<M> {
    fn clone(&self) -> Self {
        Self {
            local: self.local.clone(),
            meta: self.meta.clone(),
        }
    }
}

#[contract_trait]
impl<M: MathsCore> Habitat<M> for SpatiallyImplicitHabitat<M> {
    type LocationIterator<'a> = impl Iterator<Item = Location> + 'a;

    #[must_use]
    fn is_finite(&self) -> bool {
        true
    }

    #[must_use]
    fn get_extent(&self) -> &LandscapeExtent {
        &SPATIALLY_IMPLICIT_EXTENT
    }

    #[must_use]
    fn get_total_habitat(&self) -> OffByOneU64 {
        self.local.get_total_habitat() + self.meta().get_total_habitat()
    }

    #[must_use]
    fn get_habitat_at_location(&self, location: &Location) -> u32 {
        if self.local.get_extent().contains(location) {
            self.local.get_habitat_at_location(location)
        } else if self.meta().get_extent().contains(location) {
            self.meta.get_habitat_at_location(location)
        } else {
            0_u32
        }
    }

    #[must_use]
    #[inline]
    fn map_indexed_location_to_u64_injective(&self, indexed_location: &IndexedLocation) -> u64 {
        if self
            .local
            .get_extent()
            .contains(indexed_location.location())
        {
            self.local
                .map_indexed_location_to_u64_injective(indexed_location)
        } else {
            self.meta
                .map_indexed_location_to_u64_injective(indexed_location)
        }
    }

    #[must_use]
    fn iter_habitable_locations(&self) -> Self::LocationIterator<'_> {
        self.local
            .iter_habitable_locations()
            .chain(self.meta.iter_habitable_locations())
    }
}

#[contract_trait]
impl<M: MathsCore, G: RngCore<M>> UniformlySampleableHabitat<M, G> for SpatiallyImplicitHabitat<M> {
    #[must_use]
    #[inline]
    fn sample_habitable_indexed_location(&self, rng: &mut G) -> IndexedLocation {
        self.local.sample_habitable_indexed_location(rng)
    }
}
