use alloc::vec::Vec;

use necsim_core::lineage::MigratingLineage;

#[expect(clippy::module_name_repetitions)]
pub struct ImmigrantPopIterator<'i> {
    immigrants: &'i mut Vec<MigratingLineage>,
}

impl<'i> ImmigrantPopIterator<'i> {
    pub fn new(immigrants: &'i mut Vec<MigratingLineage>) -> Self {
        Self { immigrants }
    }
}

impl<'i> Iterator for ImmigrantPopIterator<'i> {
    type Item = MigratingLineage;

    fn next(&mut self) -> Option<Self::Item> {
        self.immigrants.pop()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.immigrants.len(), Some(self.immigrants.len()))
    }
}
