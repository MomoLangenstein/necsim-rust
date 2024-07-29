use serde::{Deserialize, Serialize};
use serde_state::DeserializeState;

use necsim_core::reporter::Reporter;
use necsim_core_bond::PositiveF64;
use necsim_partitioning_core::{
    partition::{Partition, PartitionSize},
    LocalPartition, Partitioning,
};

#[derive(Clone, Serialize, Debug)]
#[expect(clippy::module_name_repetitions)]
pub struct GillespieArguments {
    pub parallelism_mode: ParallelismMode,
}

impl<'de> DeserializeState<'de, PartitionSize> for GillespieArguments {
    fn deserialize_state<D>(
        partition_size: &mut PartitionSize,
        deserializer: D,
    ) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let raw = GillespieArgumentsRaw::deserialize_state(partition_size, deserializer)?;

        let parallelism_mode = match raw.parallelism_mode {
            Some(parallelism_mode) => parallelism_mode,
            None => {
                if partition_size.is_monolithic() {
                    ParallelismMode::Monolithic
                } else {
                    ParallelismMode::Lockstep
                }
            },
        };

        Ok(GillespieArguments { parallelism_mode })
    }
}

#[derive(Default, Debug, DeserializeState)]
#[serde(default, deny_unknown_fields)]
#[serde(rename = "GillespieArguments")]
#[serde(deserialize_state = "PartitionSize")]
struct GillespieArgumentsRaw {
    #[serde(deserialize_state)]
    parallelism_mode: Option<ParallelismMode>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptimisticParallelismMode {
    pub delta_sync: PositiveF64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AveragingParallelismMode {
    pub delta_sync: PositiveF64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ParallelismMode {
    Monolithic,
    Optimistic(OptimisticParallelismMode),
    Lockstep,
    OptimisticLockstep,
    Averaging(AveragingParallelismMode),
}

impl<'de> DeserializeState<'de, PartitionSize> for ParallelismMode {
    fn deserialize_state<D>(
        partition_size: &mut PartitionSize,
        deserializer: D,
    ) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        use serde::de::Error;

        let parallelism_mode = ParallelismMode::deserialize(deserializer)?;

        match parallelism_mode {
            ParallelismMode::Monolithic if !partition_size.is_monolithic() => {
                Err(D::Error::custom(format!(
                    "parallelism_mode {parallelism_mode:?} is incompatible with non-monolithic \
                     partitioning."
                )))
            },
            ParallelismMode::Optimistic(..)
            | ParallelismMode::Lockstep
            | ParallelismMode::OptimisticLockstep
            | ParallelismMode::Averaging(..)
                if partition_size.is_monolithic() =>
            {
                Err(D::Error::custom(format!(
                    "parallelism_mode {parallelism_mode:?} is incompatible with monolithic \
                     partitioning."
                )))
            },
            partition_mode => Ok(partition_mode),
        }
    }
}

#[must_use]
pub fn get_gillespie_logical_partition_size<P: Partitioning>(
    args: &GillespieArguments,
    partitioning: &P,
) -> PartitionSize {
    match &args.parallelism_mode {
        ParallelismMode::Monolithic => PartitionSize::MONOLITHIC,
        ParallelismMode::Optimistic(_)
        | ParallelismMode::Lockstep
        | ParallelismMode::OptimisticLockstep
        | ParallelismMode::Averaging(_) => partitioning.get_size(),
    }
}

#[must_use]
pub fn get_gillespie_logical_partition<'p, R: Reporter, P: LocalPartition<'p, R>>(
    args: &GillespieArguments,
    local_partition: &P,
) -> Partition {
    match &args.parallelism_mode {
        ParallelismMode::Monolithic => Partition::monolithic(),
        ParallelismMode::Optimistic(_)
        | ParallelismMode::Lockstep
        | ParallelismMode::OptimisticLockstep
        | ParallelismMode::Averaging(_) => local_partition.get_partition(),
    }
}
