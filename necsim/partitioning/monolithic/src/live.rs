use std::{fmt, ops::ControlFlow};

use necsim_core::{
    lineage::MigratingLineage,
    reporter::{boolean::True, FilteredReporter, Reporter},
};
use necsim_core_bond::PositiveF64;

use necsim_partitioning_core::{
    iterator::ImmigrantPopIterator, partition::Partition, LocalPartition, MigrationMode,
};

#[expect(clippy::module_name_repetitions)]
pub struct LiveMonolithicLocalPartition<R: Reporter> {
    reporter: FilteredReporter<R, True, True, True>,
    loopback: Vec<MigratingLineage>,
}

impl<R: Reporter> fmt::Debug for LiveMonolithicLocalPartition<R> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        struct LoopbackLen(usize);

        impl fmt::Debug for LoopbackLen {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "Vec<MigratingLineage; {}>", self.0)
            }
        }

        fmt.debug_struct(stringify!(LiveMonolithicLocalPartition))
            .field("reporter", &self.reporter)
            .field("loopback", &LoopbackLen(self.loopback.len()))
            .finish()
    }
}

impl<'p, R: Reporter> LocalPartition<'p, R> for LiveMonolithicLocalPartition<R> {
    type ImmigrantIterator<'a> = ImmigrantPopIterator<'a> where 'p: 'a, R: 'a;
    type IsLive = True;
    type Reporter = FilteredReporter<R, True, True, True>;

    fn get_reporter(&mut self) -> &mut Self::Reporter {
        &mut self.reporter
    }

    fn get_partition(&self) -> Partition {
        Partition::monolithic()
    }

    fn migrate_individuals<'a, E: Iterator<Item = (u32, MigratingLineage)>>(
        &'a mut self,
        emigrants: &mut E,
        _emigration_mode: MigrationMode,
        _immigration_mode: MigrationMode,
    ) -> Self::ImmigrantIterator<'a>
    where
        'p: 'a,
    {
        for (_, emigrant) in emigrants {
            self.loopback.push(emigrant);
        }

        ImmigrantPopIterator::new(&mut self.loopback)
    }

    fn reduce_vote_any(&mut self, vote: bool) -> bool {
        vote
    }

    fn reduce_vote_min_time(
        &mut self,
        local_time: PositiveF64,
    ) -> Result<PositiveF64, PositiveF64> {
        Ok(local_time)
    }

    fn wait_for_termination(&mut self) -> ControlFlow<(), ()> {
        if self.loopback.is_empty() {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }

    fn report_progress_sync(&mut self, remaining: u64) {
        self.reporter.report_progress(&remaining.into());
    }
}

impl<R: Reporter> LiveMonolithicLocalPartition<R> {
    pub(crate) fn from_reporter(reporter: FilteredReporter<R, True, True, True>) -> Self {
        Self {
            reporter,
            loopback: Vec::new(),
        }
    }

    pub(crate) fn into_reporter(self) -> FilteredReporter<R, True, True, True> {
        self.reporter
    }
}
