use std::{
    fmt,
    marker::PhantomData,
    ops::ControlFlow,
    sync::{
        mpsc::{Receiver, SendError, SyncSender, TrySendError},
        Arc, Barrier,
    },
    task::Poll,
    time::{Duration, Instant},
};

use necsim_core::{
    impl_report,
    lineage::MigratingLineage,
    reporter::{
        boolean::{Boolean, False},
        Reporter,
    },
};
use necsim_core_bond::PositiveF64;

use necsim_impls_std::event_log::recorder::EventLogRecorder;
use necsim_partitioning_core::{partition::Partition, LocalPartition, MigrationMode};

use crate::vote::{AsyncVote, Vote};

#[allow(clippy::module_name_repetitions)]
pub struct ThreadsLocalPartition<R: Reporter> {
    partition: Partition,
    vote_any: Vote<bool>,
    vote_min_time: Vote<(PositiveF64, u32)>,
    vote_termination: AsyncVote<ControlFlow<(), ()>>,
    emigration_buffers: Box<[Vec<MigratingLineage>]>,
    emigration_channels: Box<[SyncSender<Vec<MigratingLineage>>]>,
    immigration_buffers: Vec<Vec<MigratingLineage>>,
    immigration_channel: Receiver<Vec<MigratingLineage>>,
    last_migration_times: Box<[Instant]>,
    communicated_since_last_termination_vote: bool,
    migration_interval: Duration,
    recorder: EventLogRecorder,
    local_remaining: u64,
    progress_channel: SyncSender<(u64, u32)>,
    last_report_time: Instant,
    progress_interval: Duration,
    sync_barrier: Arc<Barrier>,
    _marker: PhantomData<R>,
}

impl<R: Reporter> fmt::Debug for ThreadsLocalPartition<R> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct(stringify!(ThreadsLocalPartition)).finish()
    }
}

impl<R: Reporter> ThreadsLocalPartition<R> {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub(crate) fn new(
        partition: Partition,
        vote_any: &Vote<bool>,
        vote_min_time: &Vote<(PositiveF64, u32)>,
        vote_termination: &AsyncVote<ControlFlow<(), ()>>,
        emigration_channels: &[SyncSender<Vec<MigratingLineage>>],
        immigration_channel: Receiver<Vec<MigratingLineage>>,
        migration_interval: Duration,
        mut recorder: EventLogRecorder,
        progress_channel: SyncSender<(u64, u32)>,
        progress_interval: Duration,
        sync_barrier: &Arc<Barrier>,
    ) -> Self {
        recorder.set_event_filter(R::ReportSpeciation::VALUE, R::ReportDispersal::VALUE);

        let partition_size = partition.size().get() as usize;

        let mut emigration_buffers = Vec::with_capacity(partition_size);
        emigration_buffers.resize_with(partition_size, Vec::new);

        let now = Instant::now();

        Self {
            partition,
            vote_any: vote_any.clone(),
            vote_min_time: vote_min_time.clone(),
            vote_termination: vote_termination.clone(),
            emigration_buffers: emigration_buffers.into_boxed_slice(),
            emigration_channels: Vec::from(emigration_channels).into_boxed_slice(),
            immigration_buffers: Vec::new(),
            immigration_channel,
            last_migration_times: vec![
                now.checked_sub(migration_interval).unwrap_or(now);
                partition_size
            ]
            .into_boxed_slice(),
            communicated_since_last_termination_vote: false,
            migration_interval,
            recorder,
            local_remaining: 0,
            progress_channel,
            last_report_time: now.checked_sub(progress_interval).unwrap_or(now),
            progress_interval,
            sync_barrier: sync_barrier.clone(),
            _marker: PhantomData::<R>,
        }
    }
}

impl<'p, R: Reporter> LocalPartition<'p, R> for ThreadsLocalPartition<R> {
    type ImmigrantIterator<'a> = ImmigrantPopIterator<'a> where 'p: 'a, R: 'a;
    type IsLive = False;
    type Reporter = Self;

    fn get_reporter(&mut self) -> &mut Self::Reporter {
        self
    }

    fn get_partition(&self) -> Partition {
        self.partition
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
        for (partition, emigrant) in emigrants {
            self.emigration_buffers[partition as usize].push(emigrant);
        }

        let self_rank_index = self.get_partition().rank() as usize;

        let now = Instant::now();

        // Receive incoming immigrating lineages
        if match immigration_mode {
            MigrationMode::Force => true,
            MigrationMode::Default => {
                now.duration_since(self.last_migration_times[self_rank_index])
                    >= self.migration_interval
            },
            MigrationMode::Hold => false,
        } {
            self.last_migration_times[self_rank_index] = now;

            self.immigration_buffers
                .extend(self.immigration_channel.try_iter());
        }

        // Send outgoing emigrating lineages
        for partition in self.partition.size().partitions() {
            let rank_index = partition.rank() as usize;

            if rank_index != self_rank_index
                && match emigration_mode {
                    MigrationMode::Force => true,
                    MigrationMode::Default => {
                        now.duration_since(self.last_migration_times[rank_index])
                            >= self.migration_interval
                    },
                    MigrationMode::Hold => false,
                }
            {
                let emigration_buffer = &mut self.emigration_buffers[rank_index];

                if !emigration_buffer.is_empty() {
                    let emigration_buffer_message = std::mem::take(emigration_buffer);

                    // Send a new non-empty request iff there is capacity
                    match self.emigration_channels[rank_index].try_send(emigration_buffer_message) {
                        Ok(()) => {
                            self.last_migration_times[rank_index] = now;

                            // we cannot terminate in this round since this partition gave up work
                            self.communicated_since_last_termination_vote = true;
                        },
                        Err(TrySendError::Full(emigration_buffer_message)) => {
                            *emigration_buffer = emigration_buffer_message;
                        },
                        Err(TrySendError::Disconnected(_)) => {
                            panic!("threads partitioning migration channel disconnected")
                        },
                    }
                }
            }
        }

        ImmigrantPopIterator::new(&mut self.immigration_buffers)
    }

    fn reduce_vote_any(&mut self, vote: bool) -> bool {
        self.vote_any.vote(|acc| match acc {
            None => vote,
            Some(acc) => *acc || vote,
        })
    }

    fn reduce_vote_min_time(
        &mut self,
        local_time: PositiveF64,
    ) -> Result<PositiveF64, PositiveF64> {
        let vote = (local_time, self.partition.rank());

        let result = self.vote_min_time.vote(|acc| match acc {
            None => vote,
            Some(acc) => vote.min(*acc),
        });

        if result.1 == self.partition.rank() {
            Ok(result.0)
        } else {
            Err(result.0)
        }
    }

    fn wait_for_termination(&mut self) -> ControlFlow<(), ()> {
        let mut local_wait = ControlFlow::Break(());

        // This partition can only terminate once all emigrations have been processed
        for buffer in &self.emigration_buffers {
            if !buffer.is_empty() {
                local_wait = ControlFlow::Continue(());
                break;
            }
        }
        // This partition can only terminate once all immigrations have been processed
        if !self.immigration_buffers.is_empty() {
            local_wait = ControlFlow::Continue(());
        }

        // Continue early if no vote is ongoing
        if local_wait.is_continue() && !self.vote_termination.is_ongoing() {
            return ControlFlow::Continue(());
        }

        // This partition can only terminate if there was no communication since the
        //  last vote
        if self.communicated_since_last_termination_vote {
            local_wait = ControlFlow::Continue(());
        }

        // Participate in an async poll, only blocks on a barrier once all votes are in
        let async_vote = self.vote_termination.vote(
            |global_wait| {
                self.communicated_since_last_termination_vote = false;

                match global_wait {
                    Some(ControlFlow::Continue(())) => ControlFlow::Continue(()),
                    Some(ControlFlow::Break(())) | None => local_wait,
                }
            },
            self.partition.rank(),
        );

        match async_vote {
            Poll::Pending => {
                // This partition doesn't have any work right now but we have
                //  to do another round of voting, so let's yield to not busy
                //  wait - we'll be woken up if more work comes in
                std::thread::yield_now();
                ControlFlow::Continue(())
            },
            Poll::Ready(result) => result,
        }
    }

    fn report_progress_sync(&mut self, remaining: u64) {
        if let Err(SendError(_)) = self
            .progress_channel
            .send((remaining, self.partition.rank()))
        {
            panic!("threads partitioning sync progress channel disconnected");
        }

        self.sync_barrier.wait();
    }
}

impl<R: Reporter> Reporter for ThreadsLocalPartition<R> {
    impl_report!(speciation(&mut self, speciation: MaybeUsed<R::ReportSpeciation>) {
        self.recorder.record_speciation(speciation);
    });

    impl_report!(dispersal(&mut self, dispersal: MaybeUsed<R::ReportDispersal>) {
        self.recorder.record_dispersal(dispersal);
    });

    impl_report!(progress(&mut self, remaining: MaybeUsed<R::ReportProgress>) {
        if self.local_remaining == *remaining {
            return;
        }

        // Only send progress if there is no ongoing termination vote
        if !self.vote_termination.is_ongoing() {
            let now = Instant::now();

            if now.duration_since(self.last_report_time) >= self.progress_interval {
                match self.progress_channel.try_send((*remaining, self.partition.rank())) {
                    Ok(()) => {
                        self.last_report_time = now;
                        self.local_remaining = *remaining;
                    },
                    Err(TrySendError::Full(_)) => (),
                    Err(TrySendError::Disconnected(_)) =>  {
                        panic!("threads partitioning progress channel disconnected")
                    },
                }
            }
        }
    });
}

pub struct ImmigrantPopIterator<'i> {
    immigrants: &'i mut Vec<Vec<MigratingLineage>>,
}

impl<'i> ImmigrantPopIterator<'i> {
    fn new(immigrants: &'i mut Vec<Vec<MigratingLineage>>) -> Self {
        Self { immigrants }
    }
}

impl<'i> Iterator for ImmigrantPopIterator<'i> {
    type Item = MigratingLineage;

    fn next(&mut self) -> Option<Self::Item> {
        let mut next_immigrants = self.immigrants.last_mut()?;

        loop {
            if let Some(next) = next_immigrants.pop() {
                return Some(next);
            }

            self.immigrants.pop();
            next_immigrants = self.immigrants.last_mut()?;
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.immigrants.iter().map(Vec::len).sum();
        (len, Some(len))
    }
}
