use core::ops::Index;

use super::{Habitat, LineageReference, MathsCore};
use crate::{
    landscape::{IndexedLocation, Location},
    lineage::{GlobalLineageReference, Lineage},
};

#[contract_trait]
pub trait LineageStore<M: MathsCore, H: Habitat<M>>:
    crate::cogs::Backup + Sized + core::fmt::Debug
{
    type LocalLineageReference: LineageReference<M, H>;

    #[must_use]
    fn with_capacity(habitat: &H, capacity: usize) -> Self;

    #[must_use]
    fn get_lineage_for_local_reference(
        &self,
        reference: &Self::LocalLineageReference,
    ) -> Option<&Lineage>;
}

#[expect(clippy::module_name_repetitions)]
#[contract_trait]
pub trait LocallyCoherentLineageStore<M: MathsCore, H: Habitat<M>>:
    LineageStore<M, H> + for<'a> Index<&'a Self::LocalLineageReference, Output = Lineage>
{
    #[must_use]
    #[debug_requires(
        habitat.is_indexed_location_habitable(indexed_location),
        "indexed location is habitable"
    )]
    fn get_global_lineage_reference_at_indexed_location(
        &self,
        indexed_location: &IndexedLocation,
        habitat: &H,
    ) -> Option<&GlobalLineageReference>;

    #[debug_requires(
        habitat.is_indexed_location_habitable(&lineage.indexed_location),
        "indexed location is habitable"
    )]
    #[debug_ensures(self.get_lineage_for_local_reference(
        &ret
    ).is_some(), "lineage was activated")]
    #[debug_ensures(
        self[&ret].indexed_location == old(lineage.indexed_location.clone()),
        "lineage was added to indexed_location"
    )]
    #[debug_ensures(
        self.get_global_lineage_reference_at_indexed_location(
            &old(lineage.indexed_location.clone()), old(habitat)
        ) == Some(&self[&ret].global_reference),
        "lineage is now indexed at indexed_location"
    )]
    fn insert_lineage_locally_coherent(
        &mut self,
        lineage: Lineage,
        habitat: &H,
    ) -> Self::LocalLineageReference;

    #[must_use]
    #[debug_requires(self.get_lineage_for_local_reference(
        &reference
    ).is_some(), "lineage is active")]
    #[debug_ensures(
        old(habitat).is_indexed_location_habitable(&ret.indexed_location),
        "prior indexed location is habitable"
    )]
    #[debug_ensures(self.get_lineage_for_local_reference(
        &old(unsafe { crate::cogs::Backup::backup_unchecked(&reference) })
    ).is_none(), "lineage was deactivated")]
    #[debug_ensures(
        ret == old(self[&reference].clone()),
        "returns the individual corresponding to reference"
    )]
    #[debug_ensures(self.get_global_lineage_reference_at_indexed_location(
        &ret.indexed_location, old(habitat)
    ).is_none(), "lineage is no longer indexed at its prior IndexedLocation")]
    fn extract_lineage_locally_coherent(
        &mut self,
        reference: Self::LocalLineageReference,
        habitat: &H,
    ) -> Lineage;
}

#[expect(clippy::module_name_repetitions)]
#[contract_trait]
pub trait GloballyCoherentLineageStore<M: MathsCore, H: Habitat<M>>:
    LocallyCoherentLineageStore<M, H>
{
    type LocationIterator<'a>: Iterator<Item = Location>
    where
        M: 'a,
        Self: 'a;

    #[must_use]
    fn iter_active_locations(&self, habitat: &H) -> Self::LocationIterator<'_>;

    #[must_use]
    #[debug_requires(
        habitat.is_location_habitable(location),
        "location is habitable"
    )]
    fn get_local_lineage_references_at_location_unordered(
        &self,
        location: &Location,
        habitat: &H,
    ) -> &[Self::LocalLineageReference];

    #[debug_ensures(
        self.get_local_lineage_references_at_location_unordered(
            &old(lineage.indexed_location.location().clone()), old(habitat)
        ).last() == Some(&ret),
        "lineage is now indexed unordered at indexed_location.location()"
    )]
    #[debug_ensures(
        old(self.get_local_lineage_references_at_location_unordered(
            lineage.indexed_location.location(), old(habitat)
        ).len() + 1) == self.get_local_lineage_references_at_location_unordered(
            &old(lineage.indexed_location.location().clone()), old(habitat)
        ).len(),
        "unordered active lineage index at given location has grown by 1"
    )]
    fn insert_lineage_globally_coherent(
        &mut self,
        lineage: Lineage,
        habitat: &H,
    ) -> Self::LocalLineageReference {
        self.insert_lineage_locally_coherent(lineage, habitat)
    }

    #[must_use]
    #[debug_ensures(
        self.get_local_lineage_references_at_location_unordered(
            ret.indexed_location.location(),
            old(habitat),
        ).len() + 1 == old(self.get_local_lineage_references_at_location_unordered(
            self[&reference].indexed_location.location(),
            old(habitat),
        ).len()), "unordered active lineage index at returned location has shrunk by 1")]
    fn extract_lineage_globally_coherent(
        &mut self,
        reference: Self::LocalLineageReference,
        habitat: &H,
    ) -> Lineage {
        self.extract_lineage_locally_coherent(reference, habitat)
    }
}
