#![deny(clippy::pedantic)]
#![no_std]

extern crate alloc;

use core::ops::ControlFlow;

use necsim_core::{
    lineage::MigratingLineage,
    reporter::{boolean::Boolean, Reporter},
};
use necsim_core_bond::PositiveF64;

pub mod iterator;
pub mod partition;
pub mod reporter;

use partition::{Partition, PartitionSize};
use reporter::{FinalisableReporter, ReporterContext};

pub trait Partitioning: Sized {
    type LocalPartition<'p, R: Reporter>: LocalPartition<'p, R>;
    type FinalisableReporter<R: Reporter>: FinalisableReporter;
    type Auxiliary;

    fn get_size(&self) -> PartitionSize;

    #[expect(clippy::missing_errors_doc)]
    fn with_local_partition<
        R: Reporter,
        P: ReporterContext<Reporter = R>,
        A: Data,
        Q: Data + serde::Serialize + serde::de::DeserializeOwned,
    >(
        self,
        reporter_context: P,
        auxiliary: Self::Auxiliary,
        args: A,
        inner: for<'p> fn(&mut Self::LocalPartition<'p, R>, A) -> Q,
        fold: fn(Q, Q) -> Q,
    ) -> anyhow::Result<(Q, Self::FinalisableReporter<R>)>;
}

pub trait Data: Send + Clone {}
impl<T: Send + Clone> Data for T {}

#[derive(Copy, Clone)]
pub enum MigrationMode {
    Force,
    Default,
    Hold,
}

pub trait LocalPartition<'p, R: Reporter>: Sized {
    type Reporter: Reporter;
    type IsLive: Boolean;
    type ImmigrantIterator<'a>: Iterator<Item = MigratingLineage>
    where
        Self: 'a,
        'p: 'a;

    fn get_reporter(&mut self) -> &mut Self::Reporter;

    fn get_partition(&self) -> Partition;

    fn migrate_individuals<'a, E: Iterator<Item = (u32, MigratingLineage)>>(
        &'a mut self,
        emigrants: &mut E,
        emigration_mode: MigrationMode,
        immigration_mode: MigrationMode,
    ) -> Self::ImmigrantIterator<'a>
    where
        'p: 'a;

    fn reduce_vote_any(&mut self, vote: bool) -> bool;

    #[expect(clippy::missing_errors_doc)]
    fn reduce_vote_min_time(&mut self, local_time: PositiveF64)
        -> Result<PositiveF64, PositiveF64>;

    fn wait_for_termination(&mut self) -> ControlFlow<(), ()>;

    fn report_progress_sync(&mut self, remaining: u64);
}
