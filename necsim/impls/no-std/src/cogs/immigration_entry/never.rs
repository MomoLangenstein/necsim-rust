use necsim_core::{
    cogs::{Backup, ImmigrationEntry, MathsCore},
    lineage::MigratingLineage,
};

#[expect(clippy::module_name_repetitions)]
#[derive(Debug, Default)]
#[cfg_attr(feature = "cuda", derive(rust_cuda::lend::LendRustToCuda))]
pub struct NeverImmigrationEntry([u8; 0]);

#[contract_trait]
impl Backup for NeverImmigrationEntry {
    unsafe fn backup_unchecked(&self) -> Self {
        Self([])
    }
}

#[contract_trait]
impl<M: MathsCore> ImmigrationEntry<M> for NeverImmigrationEntry {
    #[must_use]
    #[inline]
    #[debug_ensures(ret.is_none(), "no lineage ever immigrates")]
    fn next_optional_immigration(&mut self) -> Option<MigratingLineage> {
        None
    }

    #[must_use]
    #[inline]
    #[debug_ensures(ret.is_none(), "no lineage ever immigrates")]
    fn peek_next_immigration(&self) -> Option<&MigratingLineage> {
        None
    }
}
