#![deny(clippy::pedantic)]

#[macro_use]
extern crate contracts;

use std::{fmt, mem::ManuallyDrop, num::NonZeroU32, time::Duration};

use anyhow::Context;
use humantime_serde::re::humantime::format_duration;
use mpi::{
    datatype::PartitionMut,
    environment::Universe,
    topology::{Communicator, Rank, SimpleCommunicator},
    traits::CommunicatorCollectives,
    Count, Tag,
};
use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};
use serde_derive_state::DeserializeState;
use serde_state::{DeserializeState, Deserializer};
use thiserror::Error;

use necsim_core::{
    lineage::MigratingLineage,
    reporter::{
        boolean::{False, True},
        FilteredReporter, Reporter,
    },
};

use necsim_impls_std::event_log::recorder::EventLogConfig;
use necsim_partitioning_core::{
    partition::PartitionSize,
    reporter::{FinalisableReporter, OpaqueFinalisableReporter, ReporterContext},
    Data, Partitioning,
};

mod partition;
mod request;

pub use partition::{MpiLocalPartition, MpiParallelPartition, MpiRootPartition};
use request::{reduce_scope, DataOrRequest};

#[derive(Error, Debug)]
pub enum MpiPartitioningError {
    #[error("MPI has already been initialised.")]
    AlreadyInitialised,
    #[error("MPI must be initialised with at least two partitions.")]
    NoParallelism,
}

#[derive(Error, Debug)]
pub enum MpiLocalPartitionError {
    #[error("MPI partitioning requires an event log.")]
    MissingEventLog,
    #[error("Failed to create the event sub-log.")]
    InvalidEventSubLog,
}

pub struct MpiPartitioning {
    universe: ManuallyDrop<Universe>,
    world: SimpleCommunicator,
    migration_interval: Duration,
    progress_interval: Duration,
}

impl fmt::Debug for MpiPartitioning {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        struct FormattedDuration(Duration);

        impl fmt::Debug for FormattedDuration {
            fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                fmt.write_str(&format_duration(self.0).to_string())
            }
        }

        fmt.debug_struct(stringify!(MpiPartitioning))
            .field("world", &self.get_size().get())
            .field(
                "migration_interval",
                &FormattedDuration(self.migration_interval),
            )
            .field(
                "progress_interval",
                &FormattedDuration(self.progress_interval),
            )
            .finish_non_exhaustive()
    }
}

impl Serialize for MpiPartitioning {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut args = serializer.serialize_struct(stringify!(MpiPartitioning), 3)?;
        args.serialize_field("world", &self.get_size())?;
        args.serialize_field(
            "migration",
            &format_duration(self.migration_interval).to_string(),
        )?;
        args.serialize_field(
            "progress",
            &format_duration(self.progress_interval).to_string(),
        )?;
        args.end()
    }
}

impl<'de> Deserialize<'de> for MpiPartitioning {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut partitioning = Self::initialise().map_err(serde::de::Error::custom)?;

        let raw =
            MpiPartitioningRaw::deserialize_state(&mut partitioning.get_size(), deserializer)?;

        partitioning.set_migration_interval(raw.migration_interval);
        partitioning.set_progress_interval(raw.progress_interval);

        Ok(partitioning)
    }
}

impl MpiPartitioning {
    const MPI_DEFAULT_MIGRATION_INTERVAL: Duration = Duration::from_millis(100_u64);
    const MPI_DEFAULT_PROGRESS_INTERVAL: Duration = Duration::from_millis(100_u64);
    const MPI_MIGRATION_TAG: Tag = 1;
    const MPI_PROGRESS_TAG: Tag = 0;
    const ROOT_RANK: Rank = 0;

    /// # Errors
    ///
    /// Returns `AlreadyInitialised` if MPI was already initialised.
    /// Returns `NoParallelism` if the MPI world only consists of one or less
    ///  partitions.
    pub fn initialise() -> Result<Self, MpiPartitioningError> {
        let universe =
            ManuallyDrop::new(mpi::initialize().ok_or(MpiPartitioningError::AlreadyInitialised)?);
        let world = universe.world();

        if world.size() > 1 {
            Ok(Self {
                universe,
                world,
                migration_interval: Self::MPI_DEFAULT_MIGRATION_INTERVAL,
                progress_interval: Self::MPI_DEFAULT_PROGRESS_INTERVAL,
            })
        } else {
            Err(MpiPartitioningError::NoParallelism)
        }
    }

    pub fn set_migration_interval(&mut self, migration_interval: Duration) {
        self.migration_interval = migration_interval;
    }

    pub fn set_progress_interval(&mut self, progress_interval: Duration) {
        self.progress_interval = progress_interval;
    }

    #[debug_ensures(
        self.get_size().is_monolithic() -> ret,
        "monolithic partition is always root"
    )]
    #[must_use]
    pub fn peek_is_root(&self) -> bool {
        self.world.rank() == MpiPartitioning::ROOT_RANK
    }
}

impl Partitioning for MpiPartitioning {
    type Auxiliary = Option<EventLogConfig>;
    type FinalisableReporter<R: Reporter> = FinalisableMpiReporter<R>;
    type LocalPartition<'p, R: Reporter> = MpiLocalPartition<'p, R>;

    fn get_size(&self) -> PartitionSize {
        #[expect(clippy::cast_sign_loss)]
        let size = unsafe { NonZeroU32::new_unchecked(self.world.size() as u32) };

        PartitionSize(size)
    }

    /// # Errors
    ///
    /// Returns `MissingEventLog` if the local partition is non-monolithic and
    ///  the `event_log` is `None`.
    /// Returns `InvalidEventSubLog` if creating a sub-`event_log` failed.
    fn with_local_partition<
        R: Reporter,
        P: ReporterContext<Reporter = R>,
        A: Data,
        Q: Data + serde::Serialize + serde::de::DeserializeOwned,
    >(
        self,
        reporter_context: P,
        event_log: Self::Auxiliary,
        args: A,
        inner: for<'p> fn(&mut Self::LocalPartition<'p, R>, A) -> Q,
        fold: fn(Q, Q) -> Q,
    ) -> anyhow::Result<(Q, Self::FinalisableReporter<R>)> {
        let Some(event_log) = event_log else {
            anyhow::bail!(MpiLocalPartitionError::MissingEventLog)
        };

        let partition_event_log = event_log
            .new_child_log(&self.world.rank().to_string())
            .and_then(EventLogConfig::create)
            .context(MpiLocalPartitionError::InvalidEventSubLog)?;

        let mut mpi_local_global_wait = (false, false);
        let mut mpi_local_remaining = 0_u64;

        #[expect(clippy::cast_sign_loss)]
        let world_size = self.world.size() as usize;

        let mut mpi_emigration_buffers: Vec<Vec<MigratingLineage>> = Vec::with_capacity(world_size);
        mpi_emigration_buffers.resize_with(world_size, Vec::new);

        mpi::request::scope(|scope| {
            let scope = reduce_scope(scope);

            let mpi_local_global_wait = DataOrRequest::new(&mut mpi_local_global_wait, scope);
            let mpi_local_remaining = DataOrRequest::new(&mut mpi_local_remaining, scope);
            let mpi_emigration_buffers = mpi_emigration_buffers
                .iter_mut()
                .map(|buffer| DataOrRequest::new(buffer, scope))
                .collect::<Vec<_>>()
                .into_boxed_slice();

            let mut local_partition = if self.world.rank() == MpiPartitioning::ROOT_RANK {
                MpiLocalPartition::Root(Box::new(MpiRootPartition::new(
                    ManuallyDrop::into_inner(self.universe),
                    mpi_local_global_wait,
                    mpi_emigration_buffers,
                    reporter_context.try_build()?,
                    partition_event_log,
                    self.migration_interval,
                    self.progress_interval,
                )))
            } else {
                MpiLocalPartition::Parallel(Box::new(MpiParallelPartition::new(
                    ManuallyDrop::into_inner(self.universe),
                    mpi_local_global_wait,
                    mpi_local_remaining,
                    mpi_emigration_buffers,
                    partition_event_log,
                    self.migration_interval,
                    self.progress_interval,
                )))
            };

            let local_result = inner(&mut local_partition, args);

            let result = reduce_partitioning_data(&self.world, local_result, fold)?;

            Ok((result, local_partition.into_reporter()))
        })
    }
}

#[derive(DeserializeState)]
#[serde(rename = "MpiPartitioning")]
#[serde(deny_unknown_fields)]
#[serde(deserialize_state = "PartitionSize")]
#[serde(default)]
struct MpiPartitioningRaw {
    #[serde(deserialize_state_with = "deserialize_state_mpi_world")]
    world: Option<PartitionSize>,
    #[serde(alias = "migration")]
    #[serde(with = "humantime_serde")]
    migration_interval: Duration,
    #[serde(alias = "progress")]
    #[serde(with = "humantime_serde")]
    progress_interval: Duration,
}

impl Default for MpiPartitioningRaw {
    fn default() -> Self {
        Self {
            world: None,
            migration_interval: MpiPartitioning::MPI_DEFAULT_MIGRATION_INTERVAL,
            progress_interval: MpiPartitioning::MPI_DEFAULT_PROGRESS_INTERVAL,
        }
    }
}

fn deserialize_state_mpi_world<'de, D: Deserializer<'de>>(
    mpi_world: &mut PartitionSize,
    deserializer: D,
) -> Result<Option<PartitionSize>, D::Error> {
    let maybe_world = Option::<PartitionSize>::deserialize(deserializer)?;

    match maybe_world {
        None => Ok(None),
        Some(world) if world == *mpi_world => Ok(Some(world)),
        Some(_) => Err(serde::de::Error::custom(format!(
            "mismatch with MPI world size of {mpi_world}"
        ))),
    }
}

fn reduce_partitioning_data<T: serde::Serialize + serde::de::DeserializeOwned>(
    world: &SimpleCommunicator,
    data: T,
    fold: fn(T, T) -> T,
) -> anyhow::Result<T> {
    let local_ser =
        postcard::to_stdvec(&data).context("MPI local partition result failed to serialize")?;
    std::mem::drop(data);
    let local_ser_len = Count::try_from(local_ser.len())
        .context("MPI local partition result is too big to share")?;

    #[expect(clippy::cast_sign_loss)]
    let mut counts = vec![0 as Count; world.size() as usize];
    world.all_gather_into(&local_ser_len, &mut counts);

    let offsets = counts
        .iter()
        .scan(0 as Count, |acc, &x| {
            let tmp = *acc;
            if let Some(a) = (*acc).checked_add(x) {
                *acc = a;
            } else {
                return Some(Err(anyhow::anyhow!(
                    "MPI combined local partition results are too big to share"
                )));
            }
            Some(Ok(tmp))
        })
        .collect::<Result<Vec<_>, _>>()?;

    #[expect(clippy::cast_sign_loss)]
    let mut all_sers = vec![0_u8; counts.iter().copied().sum::<Count>() as usize];
    world.all_gather_varcount_into(
        local_ser.as_slice(),
        &mut PartitionMut::new(all_sers.as_mut_slice(), counts.as_slice(), offsets),
    );

    let folded: Option<T> = counts
        .iter()
        .scan(0_usize, |acc, &x| {
            let pre = *acc;
            #[expect(clippy::cast_sign_loss)]
            {
                *acc += x as usize;
            }
            let post = *acc;

            let de: anyhow::Result<T> = postcard::from_bytes(&all_sers[pre..post])
                .context("MPI data failed to deserialize");

            Some(de)
        })
        .try_fold(None, |acc, x| match (acc, x) {
            (_, Err(err)) => Err(err),
            (Some(acc), Ok(x)) => Ok(Some(fold(acc, x))),
            (None, Ok(x)) => Ok(Some(x)),
        })?;
    let folded = folded.expect("at least one MPI partitioning result");

    Ok(folded)
}

pub enum FinalisableMpiReporter<R: Reporter> {
    Root(OpaqueFinalisableReporter<FilteredReporter<R, False, False, True>>),
    Parallel,
}

impl<R: Reporter> FinalisableReporter for FinalisableMpiReporter<R> {
    fn finalise(self) {
        if let Self::Root(reporter) = self {
            reporter.finalise();
        }
    }
}
