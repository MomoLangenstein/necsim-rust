use std::{fmt, ops::ControlFlow};

use anyhow::Result;

use necsim_core::{
    impl_report,
    lineage::MigratingLineage,
    reporter::{
        boolean::{Boolean, False, True},
        FilteredReporter, Reporter,
    },
};
use necsim_core_bond::PositiveF64;

use necsim_impls_std::event_log::recorder::EventLogRecorder;

use necsim_partitioning_core::{
    iterator::ImmigrantPopIterator, partition::Partition, LocalPartition, MigrationMode,
};

#[expect(clippy::module_name_repetitions)]
pub struct RecordedMonolithicLocalPartition<R: Reporter> {
    reporter: FilteredReporter<R, False, False, True>,
    recorder: EventLogRecorder,
    loopback: Vec<MigratingLineage>,
}

impl<R: Reporter> fmt::Debug for RecordedMonolithicLocalPartition<R> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        struct LoopbackLen(usize);

        impl fmt::Debug for LoopbackLen {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "Vec<MigratingLineage; {}>", self.0)
            }
        }

        fmt.debug_struct(stringify!(RecordedMonolithicLocalPartition))
            .field("reporter", &self.reporter)
            .field("recorder", &self.recorder)
            .field("loopback", &LoopbackLen(self.loopback.len()))
            .finish()
    }
}

impl<'p, R: Reporter> LocalPartition<'p, R> for RecordedMonolithicLocalPartition<R> {
    type ImmigrantIterator<'a> = ImmigrantPopIterator<'a> where 'p: 'a, R: 'a;
    type IsLive = False;
    type Reporter = Self;

    fn get_reporter(&mut self) -> &mut Self::Reporter {
        self
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

impl<R: Reporter> RecordedMonolithicLocalPartition<R> {
    pub(crate) fn from_reporter_and_recorder(
        reporter: FilteredReporter<R, False, False, True>,
        mut recorder: EventLogRecorder,
    ) -> Self {
        recorder.set_event_filter(R::ReportSpeciation::VALUE, R::ReportDispersal::VALUE);

        Self {
            reporter,
            recorder,
            loopback: Vec::new(),
        }
    }

    pub(crate) fn into_reporter(self) -> FilteredReporter<R, False, False, True> {
        self.reporter
    }
}

impl<R: Reporter> Reporter for RecordedMonolithicLocalPartition<R> {
    impl_report!(speciation(&mut self, speciation: MaybeUsed<R::ReportSpeciation>) {
        self.recorder.record_speciation(speciation);
    });

    impl_report!(dispersal(&mut self, dispersal: MaybeUsed<R::ReportDispersal>) {
        self.recorder.record_dispersal(dispersal);
    });

    impl_report!(progress(&mut self, progress: MaybeUsed<R::ReportProgress>) {
        self.reporter.report_progress(progress.into());
    });
}
