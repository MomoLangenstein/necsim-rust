use alloc::collections::BinaryHeap;
use core::cmp::Reverse;

use necsim_core::{
    cogs::{Backup, ImmigrationEntry, MathsCore},
    lineage::MigratingLineage,
};

#[expect(clippy::module_name_repetitions)]
#[derive(Debug, Default)]
pub struct BufferedImmigrationEntry {
    immigrants: BinaryHeap<Reverse<MigratingLineage>>,
}

#[contract_trait]
impl Backup for BufferedImmigrationEntry {
    unsafe fn backup_unchecked(&self) -> Self {
        Self {
            immigrants: self
                .immigrants
                .iter()
                .map(|migrating_lineage| Reverse(migrating_lineage.0.backup_unchecked()))
                .collect(),
        }
    }
}

#[contract_trait]
impl<M: MathsCore> ImmigrationEntry<M> for BufferedImmigrationEntry {
    #[must_use]
    fn next_optional_immigration(&mut self) -> Option<MigratingLineage> {
        self.immigrants.pop().map(|immigrant| immigrant.0)
    }

    #[must_use]
    fn peek_next_immigration(&self) -> Option<&MigratingLineage> {
        self.immigrants.peek().map(|immigrant| &immigrant.0)
    }
}

impl BufferedImmigrationEntry {
    pub fn reset(&mut self) {
        self.immigrants.clear();
    }

    pub fn push(&mut self, immigrant: MigratingLineage) {
        self.immigrants.push(Reverse(immigrant));
    }

    pub fn extend(&mut self, immigrants: impl Iterator<Item = MigratingLineage>) {
        self.immigrants.extend(immigrants.map(Reverse));
    }
}
