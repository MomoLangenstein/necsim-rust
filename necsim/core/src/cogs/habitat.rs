use necsim_core_bond::OffByOneU64;

use crate::landscape::{IndexedLocation, LandscapeExtent, Location};

use super::{MathsCore, RngCore};

#[contract_trait]
pub trait Habitat<M: MathsCore>: crate::cogs::Backup + core::fmt::Debug + Sized {
    type LocationIterator<'a>: Iterator<Item = Location> + 'a
    where
        Self: 'a;

    #[must_use]
    fn is_finite(&self) -> bool;

    #[must_use]
    fn get_extent(&self) -> &LandscapeExtent;

    #[must_use]
    fn is_location_habitable(&self, location: &Location) -> bool {
        self.get_extent().contains(location) && self.get_habitat_at_location(location) > 0_u32
    }

    #[must_use]
    fn is_indexed_location_habitable(&self, indexed_location: &IndexedLocation) -> bool {
        self.get_extent().contains(indexed_location.location())
            && (indexed_location.index()
                < self.get_habitat_at_location(indexed_location.location()))
    }

    #[must_use]
    #[debug_ensures(!self.is_finite() || ret.get() == {
        self.iter_habitable_locations()
            .map(|location| u128::from(self.get_habitat_at_location(&location)))
            .sum()
    }, "total habitat is the sum of all habitat in the extent of the habitat")]
    fn get_total_habitat(&self) -> OffByOneU64;

    #[must_use]
    #[debug_requires(self.get_extent().contains(location), "location is inside habitat extent")]
    fn get_habitat_at_location(&self, location: &Location) -> u32;

    #[must_use]
    #[debug_requires(
        self.get_extent().contains(indexed_location.location()),
        "location is inside habitat extent"
    )]
    #[debug_requires(
        indexed_location.index() < self.get_habitat_at_location(indexed_location.location()),
        "index is within the location's habitat capacity"
    )]
    fn map_indexed_location_to_u64_injective(&self, indexed_location: &IndexedLocation) -> u64;

    #[must_use]
    fn iter_habitable_locations(&self) -> Self::LocationIterator<'_>;
}

#[expect(clippy::module_name_repetitions)]
#[contract_trait]
pub trait UniformlySampleableHabitat<M: MathsCore, G: RngCore<M>>: Habitat<M> {
    #[debug_ensures(
        old(self).get_extent().contains(ret.location()) &&
        ret.index() < old(self).get_habitat_at_location(ret.location()),
        "sampled indexed location is habitable"
    )]
    fn sample_habitable_indexed_location(&self, rng: &mut G) -> IndexedLocation;
}
