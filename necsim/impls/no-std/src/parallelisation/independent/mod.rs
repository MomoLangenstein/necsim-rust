use core::num::NonZeroUsize;

use serde::{Deserialize, Serialize};

use necsim_core_bond::PositiveF64;

use crate::cache::DirectMappedCache;

mod reporter;

pub mod individuals;
pub mod landscape;
pub mod monolithic;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AbsoluteCapacity {
    pub capacity: NonZeroUsize,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelativeCapacity {
    pub factor: PositiveF64,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum DedupCache {
    Absolute(AbsoluteCapacity),
    Relative(RelativeCapacity),
    None,
}

impl DedupCache {
    #[must_use]
    pub fn construct<T: core::hash::Hash + PartialEq>(
        self,
        workload: usize,
    ) -> DirectMappedCache<T> {
        DirectMappedCache::with_capacity(match self {
            DedupCache::Absolute(AbsoluteCapacity { capacity }) => capacity.get(),
            DedupCache::Relative(RelativeCapacity { factor }) => {
                #[expect(
                    clippy::cast_precision_loss,
                    clippy::cast_sign_loss,
                    clippy::cast_possible_truncation
                )]
                let capacity = ((workload as f64) * factor.get()) as usize;

                capacity
            },
            DedupCache::None => 0_usize,
        })
    }
}

#[expect(clippy::unsafe_derive_deserialize)]
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum EventSlice {
    Absolute(AbsoluteCapacity),
    Relative(RelativeCapacity),
}

impl EventSlice {
    #[must_use]
    pub fn capacity(self, workload: usize) -> NonZeroUsize {
        match self {
            EventSlice::Absolute(AbsoluteCapacity { capacity }) => capacity,
            EventSlice::Relative(RelativeCapacity { factor }) => {
                #[expect(
                    clippy::cast_precision_loss,
                    clippy::cast_sign_loss,
                    clippy::cast_possible_truncation
                )]
                let capacity = ((workload as f64) * factor.get()) as usize;

                // Safety: max(c, 1) >= 1
                unsafe { NonZeroUsize::new_unchecked(capacity.max(1_usize)) }
            },
        }
    }
}
