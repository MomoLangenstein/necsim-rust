use core::{fmt, marker::PhantomData};

use necsim_core::{
    cogs::{Habitat, MathsCore, RngCore, UniformlySampleableHabitat},
    landscape::{IndexedLocation, LandscapeExtent, Location},
};
use necsim_core_bond::{OffByOneU32, OffByOneU64};

use crate::cogs::lineage_store::coherent::globally::singleton_demes::SingletonDemesHabitat;

pub mod downscaled;

const ALMOST_INFINITE_EXTENT: LandscapeExtent =
    LandscapeExtent::new(Location::new(0, 0), OffByOneU32::max(), OffByOneU32::max());

#[expect(clippy::module_name_repetitions)]
#[cfg_attr(feature = "cuda", derive(rust_cuda::lend::LendRustToCuda))]
#[cfg_attr(feature = "cuda", cuda(free = "M"))]
pub struct AlmostInfiniteHabitat<M: MathsCore> {
    marker: PhantomData<M>,
}

impl<M: MathsCore> fmt::Debug for AlmostInfiniteHabitat<M> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct(stringify!(AlmostInfiniteHabitat)).finish()
    }
}

impl<M: MathsCore> Default for AlmostInfiniteHabitat<M> {
    fn default() -> Self {
        Self {
            marker: PhantomData::<M>,
        }
    }
}

impl<M: MathsCore> Clone for AlmostInfiniteHabitat<M> {
    fn clone(&self) -> Self {
        Self {
            marker: PhantomData::<M>,
        }
    }
}

#[contract_trait]
impl<M: MathsCore> Habitat<M> for AlmostInfiniteHabitat<M> {
    type LocationIterator<'a> = impl Iterator<Item = Location>;

    #[must_use]
    fn is_finite(&self) -> bool {
        false
    }

    #[must_use]
    fn get_extent(&self) -> &LandscapeExtent {
        &ALMOST_INFINITE_EXTENT
    }

    #[must_use]
    fn get_total_habitat(&self) -> OffByOneU64 {
        OffByOneU64::max()
    }

    #[must_use]
    fn get_habitat_at_location(&self, _location: &Location) -> u32 {
        1_u32
    }

    #[must_use]
    fn map_indexed_location_to_u64_injective(&self, indexed_location: &IndexedLocation) -> u64 {
        (u64::from(indexed_location.location().y()) << 32)
            | u64::from(indexed_location.location().x())
    }

    #[must_use]
    fn iter_habitable_locations(&self) -> Self::LocationIterator<'_> {
        self.get_extent().iter()
    }
}

#[contract_trait]
impl<M: MathsCore, G: RngCore<M>> UniformlySampleableHabitat<M, G> for AlmostInfiniteHabitat<M> {
    #[must_use]
    #[inline]
    fn sample_habitable_indexed_location(&self, rng: &mut G) -> IndexedLocation {
        let index = rng.sample_u64();

        IndexedLocation::new(
            Location::new(
                (index & 0xFFFF_FFFF) as u32,
                ((index >> 32) & 0xFFFF_FFFF) as u32,
            ),
            0,
        )
    }
}

impl<M: MathsCore> SingletonDemesHabitat<M> for AlmostInfiniteHabitat<M> {}

impl<M: MathsCore> AlmostInfiniteHabitat<M> {
    #[must_use]
    pub fn clamp_round_dispersal(location: &Location, dx: f64, dy: f64) -> Location {
        const WRAP: i64 = 1 << 32;

        // Discrete dispersal assumes lineage positions are centred on (0.5, 0.5),
        // i.e. |dispersal| >= 0.5 changes the cell
        // (dx and dy must be rounded to nearest int away from 0.0)
        #[expect(clippy::cast_possible_truncation)]
        let (dx, dy): (i64, i64) = (M::round(dx) as i64, M::round(dy) as i64);

        let new_x = (i64::from(location.x()) + dx) % WRAP;
        let new_y = (i64::from(location.y()) + dy) % WRAP;

        #[expect(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        Location::new(
            ((new_x + WRAP) % WRAP) as u32,
            ((new_y + WRAP) % WRAP) as u32,
        )
    }
}
