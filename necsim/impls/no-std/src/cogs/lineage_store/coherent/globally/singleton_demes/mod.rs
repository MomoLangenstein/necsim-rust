use core::{marker::PhantomData, ops::Index};

use fnv::FnvBuildHasher;
use hashbrown::hash_map::HashMap;
use slab::Slab;

use necsim_core::{
    cogs::{Backup, Habitat, MathsCore},
    landscape::Location,
    lineage::Lineage,
};

use crate::cogs::lineage_reference::in_memory::InMemoryLineageReference;

mod store;

/// Marker trait which declares that all locations have <= 1 habitat
///  i.e. all indexed locations have index 0
#[expect(clippy::module_name_repetitions)]
pub trait SingletonDemesHabitat<M: MathsCore>: Habitat<M> {
    #[must_use]
    #[inline]
    #[debug_requires(self.get_extent().contains(location), "location is inside habitat extent")]
    #[debug_ensures(self.get_habitat_at_location(location) <= 1_u32, "habitat is <= 1")]
    fn is_habitat_at_location(&self, location: &Location) -> bool {
        self.get_habitat_at_location(location) != 0_u32
    }
}

#[expect(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct SingletonDemesLineageStore<M: MathsCore, H: SingletonDemesHabitat<M>> {
    lineages_store: Slab<Lineage>,
    location_to_lineage_reference: HashMap<Location, InMemoryLineageReference, FnvBuildHasher>,
    _marker: PhantomData<(M, H)>,
}

impl<'a, M: MathsCore, H: SingletonDemesHabitat<M>> Index<&'a InMemoryLineageReference>
    for SingletonDemesLineageStore<M, H>
{
    type Output = Lineage;

    #[must_use]
    #[debug_requires(
        self.lineages_store.contains(reference.into()),
        "lineage reference is valid in the lineage store"
    )]
    fn index(&self, reference: &'a InMemoryLineageReference) -> &Self::Output {
        &self.lineages_store[usize::from(reference)]
    }
}

#[contract_trait]
impl<M: MathsCore, H: SingletonDemesHabitat<M>> Backup for SingletonDemesLineageStore<M, H> {
    unsafe fn backup_unchecked(&self) -> Self {
        Self {
            lineages_store: self.lineages_store.clone(),
            location_to_lineage_reference: self
                .location_to_lineage_reference
                .iter()
                .map(|(k, v)| (k.clone(), v.backup_unchecked()))
                .collect(),
            _marker: PhantomData::<(M, H)>,
        }
    }
}
