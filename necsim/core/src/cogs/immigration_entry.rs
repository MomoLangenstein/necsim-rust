use crate::lineage::MigratingLineage;

use super::MathsCore;

#[contract_trait]
pub trait ImmigrationEntry<M: MathsCore>: crate::cogs::Backup + core::fmt::Debug {
    #[must_use]
    fn next_optional_immigration(&mut self) -> Option<MigratingLineage>;

    #[must_use]
    fn peek_next_immigration(&self) -> Option<&MigratingLineage>;
}
