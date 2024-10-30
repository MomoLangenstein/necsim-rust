use core::{marker::PhantomData, ops::Index};

use fnv::FnvBuildHasher;
use hashbrown::hash_map::HashMap;
use slab::Slab;

use necsim_core::{
    cogs::{Backup, Habitat, MathsCore},
    landscape::IndexedLocation,
    lineage::Lineage,
};

use crate::cogs::lineage_reference::in_memory::InMemoryLineageReference;

mod store;

#[expect(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct ClassicalLineageStore<M: MathsCore, H: Habitat<M>> {
    lineages_store: Slab<Lineage>,
    indexed_location_to_lineage_reference:
        HashMap<IndexedLocation, InMemoryLineageReference, FnvBuildHasher>,
    _marker: PhantomData<(M, H)>,
}

impl<'a, M: MathsCore, H: Habitat<M>> Index<&'a InMemoryLineageReference>
    for ClassicalLineageStore<M, H>
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
impl<M: MathsCore, H: Habitat<M>> Backup for ClassicalLineageStore<M, H> {
    unsafe fn backup_unchecked(&self) -> Self {
        Self {
            lineages_store: self.lineages_store.clone(),
            indexed_location_to_lineage_reference: self
                .indexed_location_to_lineage_reference
                .iter()
                .map(|(k, v)| (k.clone(), v.backup_unchecked()))
                .collect(),
            _marker: PhantomData::<(M, H)>,
        }
    }
}
