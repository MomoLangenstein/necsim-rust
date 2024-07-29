use necsim_core::{
    cogs::{Backup, Habitat, MathsCore},
    landscape::Location,
};
use necsim_partitioning_core::partition::Partition;

use crate::decomposition::Decomposition;

#[expect(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct ModuloDecomposition {
    subdomain: Partition,
}

impl ModuloDecomposition {
    #[must_use]
    pub fn new(subdomain: Partition) -> Self {
        Self { subdomain }
    }
}

#[contract_trait]
impl Backup for ModuloDecomposition {
    unsafe fn backup_unchecked(&self) -> Self {
        Self {
            subdomain: self.subdomain,
        }
    }
}

#[contract_trait]
impl<M: MathsCore, H: Habitat<M>> Decomposition<M, H> for ModuloDecomposition {
    fn get_subdomain(&self) -> Partition {
        self.subdomain
    }

    fn map_location_to_subdomain_rank(&self, location: &Location, habitat: &H) -> u32 {
        let extent = habitat.get_extent();

        let location_index = u64::from(location.y() - extent.origin().y())
            * u64::from(extent.width())
            + u64::from(location.x() - extent.origin().x());

        #[expect(clippy::cast_possible_truncation)]
        {
            (location_index % u64::from(self.subdomain.size().get())) as u32
        }
    }
}
