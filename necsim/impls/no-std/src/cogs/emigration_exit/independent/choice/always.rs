use necsim_core::{
    cogs::{Backup, Habitat, MathsCore},
    landscape::IndexedLocation,
};
use necsim_core_bond::PositiveF64;

use super::EmigrationChoice;

#[expect(clippy::module_name_repetitions)]
#[derive(Debug, Default)]
pub struct AlwaysEmigrationChoice([u8; 0]);

#[contract_trait]
impl Backup for AlwaysEmigrationChoice {
    unsafe fn backup_unchecked(&self) -> Self {
        Self([])
    }
}

#[contract_trait]
impl<M: MathsCore, H: Habitat<M>> EmigrationChoice<M, H> for AlwaysEmigrationChoice {
    fn should_lineage_emigrate(
        &self,
        _indexed_location: &IndexedLocation,
        _time: PositiveF64,
        _habitat: &H,
    ) -> bool {
        true
    }
}
