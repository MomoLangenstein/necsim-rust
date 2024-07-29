use std::{
    num::NonZeroU32,
    ops::ControlFlow,
    time::{Duration, Instant},
};

use mpi::{
    collective::{CommunicatorCollectives, SystemOperation},
    datatype::Equivalence,
    environment::Universe,
    point_to_point::{Destination, Source},
    topology::{Communicator, SimpleCommunicator},
};

use necsim_core::lineage::MigratingLineage;
use necsim_core_bond::PositiveF64;

use necsim_partitioning_core::{
    iterator::ImmigrantPopIterator,
    partition::{Partition, PartitionSize},
    MigrationMode,
};

use crate::{
    partition::utils::{reduce_lexicographic_min_time_rank, MpiMigratingLineage},
    request::DataOrRequest,
    MpiPartitioning,
};

pub struct MpiCommonPartition<'p> {
    _universe: Universe,
    world: SimpleCommunicator,
    mpi_local_global_wait: DataOrRequest<'p, (bool, bool), bool>,
    mpi_emigration_buffers: Box<[DataOrRequest<'p, Vec<MigratingLineage>, [MpiMigratingLineage]>]>,
    migration_buffers: Box<[Vec<MigratingLineage>]>,
    last_migration_times: Box<[Instant]>,
    communicated_since_last_barrier: bool,
    migration_interval: Duration,
}

impl<'p> MpiCommonPartition<'p> {
    #[must_use]
    pub(crate) fn new(
        universe: Universe,
        mpi_local_global_wait: DataOrRequest<'p, (bool, bool), bool>,
        mpi_emigration_buffers: Box<
            [DataOrRequest<'p, Vec<MigratingLineage>, [MpiMigratingLineage]>],
        >,
        now: Instant,
        migration_interval: Duration,
    ) -> Self {
        let world = universe.world();

        #[allow(clippy::cast_sign_loss)]
        let world_size = world.size() as usize;

        let mut migration_buffers = Vec::with_capacity(world_size);
        migration_buffers.resize_with(world_size, Vec::new);

        Self {
            _universe: universe,
            world,
            mpi_local_global_wait,
            mpi_emigration_buffers,
            migration_buffers: migration_buffers.into_boxed_slice(),
            last_migration_times: vec![
                now.checked_sub(migration_interval).unwrap_or(now);
                world_size
            ]
            .into_boxed_slice(),
            communicated_since_last_barrier: false,
            migration_interval,
        }
    }

    pub fn world(&self) -> &SimpleCommunicator {
        &self.world
    }

    pub fn has_ongoing_termination_vote(&self) -> bool {
        self.mpi_local_global_wait.get_data().is_none()
    }

    #[must_use]
    pub fn get_partition(&self) -> Partition {
        #[allow(clippy::cast_sign_loss)]
        let rank = self.world.rank() as u32;
        #[allow(clippy::cast_sign_loss)]
        let size = unsafe { NonZeroU32::new_unchecked(self.world.size() as u32) };

        unsafe { Partition::new_unchecked(rank, PartitionSize(size)) }
    }

    #[must_use]
    pub fn migrate_individuals<'a, E: Iterator<Item = (u32, MigratingLineage)>>(
        &'a mut self,
        emigrants: &mut E,
        emigration_mode: MigrationMode,
        immigration_mode: MigrationMode,
    ) -> ImmigrantPopIterator<'a>
    where
        'p: 'a,
    {
        for (partition, emigrant) in emigrants {
            self.migration_buffers[partition as usize].push(emigrant);
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

            let immigration_buffer = &mut self.migration_buffers[self_rank_index];

            let any_process = self.world.any_process();

            // Probe MPI to receive immigrating lineages
            while let Some((msg, status)) =
                any_process.immediate_matched_probe_with_tag(MpiPartitioning::MPI_MIGRATION_TAG)
            {
                #[allow(clippy::cast_sign_loss)]
                let number_immigrants =
                    status.count(MpiMigratingLineage::equivalent_datatype()) as usize;

                let receive_start = immigration_buffer.len();

                immigration_buffer.reserve(number_immigrants);

                let immigration_slice = MpiMigratingLineage::from_mut_uninit_slice(
                    &mut immigration_buffer.spare_capacity_mut()[..number_immigrants],
                );

                msg.matched_receive_into(immigration_slice);

                // Safety: The uninitialised `number_immigrants` items were just initialised
                //         in the `matched_receive_into` call
                unsafe {
                    immigration_buffer.set_len(receive_start + number_immigrants);
                }
            }
        }

        // Send outgoing emigrating lineages
        for partition in self.get_partition().size().partitions() {
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
                // Check if the prior send request has finished
                //  and clear the buffer if it has finished
                if let Some(emigration_buffer) =
                    self.mpi_emigration_buffers[rank_index].test_for_data_mut()
                {
                    emigration_buffer.clear();
                }

                let emigration_buffer = &mut self.migration_buffers[rank_index];

                if !emigration_buffer.is_empty() {
                    #[allow(clippy::cast_possible_wrap)]
                    let receiver_process = self.world.process_at_rank(partition.rank() as i32);

                    let mut last_migration_time = self.last_migration_times[rank_index];
                    let mut communicated_since_last_barrier = self.communicated_since_last_barrier;

                    // Send a new non-empty request iff the prior one has finished
                    self.mpi_emigration_buffers[rank_index].request_if_data(
                        |mpi_emigration_buffer, scope| {
                            last_migration_time = now;

                            // MPI cannot terminate in this round since this partition gave up work
                            communicated_since_last_barrier = true;

                            // Any prior send request has already finished
                            mpi_emigration_buffer.clear();

                            std::mem::swap(emigration_buffer, mpi_emigration_buffer);

                            receiver_process.immediate_synchronous_send_with_tag(
                                scope,
                                MpiMigratingLineage::from_slice(mpi_emigration_buffer),
                                MpiPartitioning::MPI_MIGRATION_TAG,
                            )
                        },
                    );

                    self.last_migration_times[rank_index] = last_migration_time;
                    self.communicated_since_last_barrier = communicated_since_last_barrier;
                }
            }
        }

        ImmigrantPopIterator::new(&mut self.migration_buffers[self_rank_index])
    }

    #[must_use]
    pub fn reduce_vote_any(&mut self, vote: bool) -> bool {
        let mut global_vote = vote;

        self.world
            .all_reduce_into(&vote, &mut global_vote, SystemOperation::logical_or());

        global_vote
    }

    pub fn reduce_vote_min_time(
        &mut self,
        local_time: PositiveF64,
    ) -> Result<PositiveF64, PositiveF64> {
        let local_partition_rank = self.get_partition().rank();

        let (global_min_time, global_min_rank) =
            reduce_lexicographic_min_time_rank(&self.world, local_time, local_partition_rank);

        if global_min_rank == local_partition_rank {
            Ok(local_time)
        } else {
            Err(global_min_time)
        }
    }

    #[must_use]
    pub fn wait_for_termination(&mut self) -> ControlFlow<(), ()> {
        // This partition can only terminate once all migrations have been processed
        for buffer in &self.migration_buffers {
            if !buffer.is_empty() {
                return ControlFlow::Continue(());
            }
        }

        // This partition can only terminate if all emigrations have been
        //  sent and acknowledged (request finished + empty buffers)
        for buffer in &self.mpi_emigration_buffers {
            if !buffer.get_data().map_or(false, Vec::is_empty) {
                return ControlFlow::Continue(());
            }
        }

        let world = &self.world;
        let mut communicated_since_last_barrier = self.communicated_since_last_barrier;

        // Create a new termination attempt if the last one failed
        self.mpi_local_global_wait
            .request_if_data(|(local_wait, global_wait), scope| {
                *local_wait = communicated_since_last_barrier;
                communicated_since_last_barrier = false;
                *global_wait = false;

                world.immediate_all_reduce_into(
                    scope,
                    local_wait,
                    global_wait,
                    SystemOperation::logical_or(),
                )
            });

        self.communicated_since_last_barrier = communicated_since_last_barrier;

        // Wait if voting is ongoing or at least one partition voted to wait
        let should_wait = if let Some((_local_wait, global_wait)) =
            self.mpi_local_global_wait.test_for_data_mut()
        {
            *global_wait
        } else {
            // This partition doesn't have any work right now but we have
            //  to do another round of voting, so let's yield to not busy
            //  wait - we'll be woken up if more work comes in
            std::thread::yield_now();

            true
        };

        if should_wait {
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(())
        }
    }
}
