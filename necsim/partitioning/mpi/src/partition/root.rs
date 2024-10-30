use std::{
    fmt,
    num::Wrapping,
    ops::ControlFlow,
    time::{Duration, Instant},
};

use mpi::{
    collective::Root, environment::Universe, point_to_point::Source, topology::Communicator,
};

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

use crate::{partition::utils::MpiMigratingLineage, request::DataOrRequest, MpiPartitioning};

use super::common::MpiCommonPartition;

pub struct MpiRootPartition<'p, R: Reporter> {
    common: MpiCommonPartition<'p>,
    all_remaining: Box<[u64]>,
    last_report_time: Instant,
    reporter: FilteredReporter<R, False, False, True>,
    recorder: EventLogRecorder,
    progress_interval: Duration,
}

impl<'p, R: Reporter> fmt::Debug for MpiRootPartition<'p, R> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct(stringify!(MpiRootPartition)).finish()
    }
}

impl<'p, R: Reporter> MpiRootPartition<'p, R> {
    #[must_use]
    pub(crate) fn new(
        universe: Universe,
        mpi_local_global_wait: DataOrRequest<'p, (bool, bool), bool>,
        mpi_emigration_buffers: Box<
            [DataOrRequest<'p, Vec<MigratingLineage>, [MpiMigratingLineage]>],
        >,
        reporter: FilteredReporter<R, False, False, True>,
        mut recorder: EventLogRecorder,
        migration_interval: Duration,
        progress_interval: Duration,
    ) -> Self {
        recorder.set_event_filter(R::ReportSpeciation::VALUE, R::ReportDispersal::VALUE);

        let now = Instant::now();

        let common = MpiCommonPartition::new(
            universe,
            mpi_local_global_wait,
            mpi_emigration_buffers,
            now,
            migration_interval,
        );

        let all_remaining =
            vec![0; common.get_partition().size().get() as usize].into_boxed_slice();

        Self {
            common,
            all_remaining,
            last_report_time: now.checked_sub(progress_interval).unwrap_or(now),
            reporter,
            recorder,
            progress_interval,
        }
    }
}

impl<'p, R: Reporter> LocalPartition<'p, R> for MpiRootPartition<'p, R> {
    type ImmigrantIterator<'a> = ImmigrantPopIterator<'a> where 'p: 'a, R: 'a;
    type IsLive = False;
    type Reporter = Self;

    fn get_reporter(&mut self) -> &mut Self::Reporter {
        self
    }

    fn get_partition(&self) -> Partition {
        self.common.get_partition()
    }

    fn migrate_individuals<'a, E: Iterator<Item = (u32, MigratingLineage)>>(
        &'a mut self,
        emigrants: &mut E,
        emigration_mode: MigrationMode,
        immigration_mode: MigrationMode,
    ) -> Self::ImmigrantIterator<'a>
    where
        'p: 'a,
    {
        self.common
            .migrate_individuals(emigrants, emigration_mode, immigration_mode)
    }

    fn reduce_vote_any(&mut self, vote: bool) -> bool {
        self.common.reduce_vote_any(vote)
    }

    fn reduce_vote_min_time(
        &mut self,
        local_time: PositiveF64,
    ) -> Result<PositiveF64, PositiveF64> {
        self.common.reduce_vote_min_time(local_time)
    }

    fn wait_for_termination(&mut self) -> ControlFlow<(), ()> {
        let result = self.common.wait_for_termination();

        if !self.common.has_ongoing_termination_vote() {
            // Check for pending progress updates from other partitions
            let remaining = self.all_remaining[self.get_partition().rank() as usize];
            self.report_progress(&remaining.into());
        }

        result
    }

    fn report_progress_sync(&mut self, remaining: u64) {
        let root_process = self
            .common
            .world()
            .process_at_rank(MpiPartitioning::ROOT_RANK);

        root_process.gather_into_root(&remaining, &mut self.all_remaining[..]);

        self.reporter.report_progress(
            &self
                .all_remaining
                .iter()
                .copied()
                .map(Wrapping)
                .sum::<Wrapping<u64>>()
                .0
                .into(),
        );
    }
}

impl<'p, R: Reporter> Reporter for MpiRootPartition<'p, R> {
    impl_report!(speciation(&mut self, speciation: MaybeUsed<R::ReportSpeciation>) {
        self.recorder.record_speciation(speciation);
    });

    impl_report!(dispersal(&mut self, dispersal: MaybeUsed<R::ReportDispersal>) {
        self.recorder.record_dispersal(dispersal);
    });

    impl_report!(progress(&mut self, remaining: MaybeUsed<R::ReportProgress>) {
        let now = Instant::now();

        if now.duration_since(self.last_report_time) >= self.progress_interval {
            self.last_report_time = now;

            self.all_remaining[MpiPartitioning::ROOT_RANK as usize] = *remaining;

            let any_process = self.common.world().any_process();

            while let Some((msg, _)) =
                any_process.immediate_matched_probe_with_tag(MpiPartitioning::MPI_PROGRESS_TAG)
            {
                let remaining_status: (u64, _) = msg.matched_receive();

                #[expect(clippy::cast_sign_loss)]
                { self.all_remaining[remaining_status.1.source_rank() as usize] = remaining_status.0; }
            }

            self.reporter.report_progress(
                &self.all_remaining
                    .iter()
                    .copied()
                    .map(Wrapping)
                    .sum::<Wrapping<u64>>()
                    .0.into()
            );
        }
    });
}

impl<'p, R: Reporter> MpiRootPartition<'p, R> {
    pub(crate) fn into_reporter(self) -> FilteredReporter<R, False, False, True> {
        self.reporter
    }
}
