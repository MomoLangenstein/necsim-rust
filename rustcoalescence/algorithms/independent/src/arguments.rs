use std::num::NonZeroU64;

use serde::{Deserialize, Serialize};
use serde_state::DeserializeState;

use necsim_core_bond::{ClosedUnitF64, PositiveF64};
use necsim_partitioning_core::partition::{Partition, PartitionSize};

use necsim_impls_no_std::parallelisation::independent::{DedupCache, EventSlice, RelativeCapacity};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MonolithicParallelismMode {
    pub event_slice: EventSlice,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IsolatedParallelismMode {
    pub event_slice: EventSlice,
    pub partition: Partition,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProbabilisticParallelismMode {
    #[serde(alias = "communication")]
    pub communication_probability: ClosedUnitF64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ParallelismMode {
    Monolithic(MonolithicParallelismMode),
    IsolatedIndividuals(IsolatedParallelismMode),
    IsolatedLandscape(IsolatedParallelismMode),
    Individuals,
    Landscape,
    Probabilistic(ProbabilisticParallelismMode),
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
            ParallelismMode::Monolithic(..)
            | ParallelismMode::IsolatedIndividuals(..)
            | ParallelismMode::IsolatedLandscape(..)
                if !partition_size.is_monolithic() =>
            {
                Err(D::Error::custom(format!(
                    "parallelism_mode {parallelism_mode:?} is incompatible with non-monolithic \
                     partitioning."
                )))
            },
            ParallelismMode::Individuals
            | ParallelismMode::Landscape
            | ParallelismMode::Probabilistic(..)
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

#[derive(Clone, Debug, Serialize)]
#[expect(clippy::module_name_repetitions)]
pub struct IndependentArguments {
    pub delta_t: PositiveF64,
    pub step_slice: NonZeroU64,
    pub dedup_cache: DedupCache,
    pub parallelism_mode: ParallelismMode,
}

impl<'de> DeserializeState<'de, PartitionSize> for IndependentArguments {
    fn deserialize_state<D>(
        partition_size: &mut PartitionSize,
        deserializer: D,
    ) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let raw = IndependentArgumentsRaw::deserialize_state(partition_size, deserializer)?;

        let parallelism_mode = match raw.parallelism_mode {
            Some(parallelism_mode) => parallelism_mode,
            None => {
                if partition_size.is_monolithic() {
                    ParallelismMode::Monolithic(MonolithicParallelismMode {
                        event_slice: EventSlice::Relative(RelativeCapacity {
                            factor: PositiveF64::new(2.0_f64).unwrap(),
                        }),
                    })
                } else {
                    ParallelismMode::Probabilistic(ProbabilisticParallelismMode {
                        communication_probability: ClosedUnitF64::new(0.25_f64).unwrap(),
                    })
                }
            },
        };

        Ok(IndependentArguments {
            delta_t: raw.delta_t,
            step_slice: raw.step_slice,
            dedup_cache: raw.dedup_cache,
            parallelism_mode,
        })
    }
}

#[derive(Debug, DeserializeState)]
#[serde(default, deny_unknown_fields)]
#[serde(deserialize_state = "PartitionSize")]
struct IndependentArgumentsRaw {
    delta_t: PositiveF64,
    step_slice: NonZeroU64,
    dedup_cache: DedupCache,
    #[serde(deserialize_state)]
    parallelism_mode: Option<ParallelismMode>,
}

impl Default for IndependentArgumentsRaw {
    fn default() -> Self {
        Self {
            delta_t: PositiveF64::new(2.0_f64).unwrap(),
            step_slice: NonZeroU64::new(10_u64).unwrap(),
            dedup_cache: DedupCache::Relative(RelativeCapacity {
                factor: PositiveF64::new(1.0_f64).unwrap(),
            }),
            parallelism_mode: None,
        }
    }
}
