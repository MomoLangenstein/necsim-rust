use core::{convert::TryFrom, fmt, num::NonZeroU32};

use serde::{Deserialize, Serialize};

#[derive(Debug)]
#[expect(clippy::module_name_repetitions)]
pub struct PartitionRankOutOfBounds(u32, u32);

impl fmt::Display for PartitionRankOutOfBounds {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(
            fmt,
            "rank {} is is not in partition range [0, {}].",
            self.0, self.1
        )
    }
}

#[expect(clippy::unsafe_derive_deserialize)]
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
#[serde(try_from = "PartitionRaw")]
pub struct Partition {
    rank: u32,
    size: PartitionSize,
}

impl Partition {
    /// Creates a `Partition` from a `rank` and number of partitions.
    ///
    /// # Errors
    ///
    /// Returns `PartitionRankOutOfBounds` if `rank >= size`.
    pub const fn try_new(rank: u32, size: PartitionSize) -> Result<Self, PartitionRankOutOfBounds> {
        if rank < size.get() {
            Ok(Self { rank, size })
        } else {
            Err(PartitionRankOutOfBounds(rank, size.get() - 1))
        }
    }

    /// Creates a `Partition` from a `rank` and number of partitions.
    ///
    /// # Safety
    ///
    /// The number of partitions must be strictly greater than `rank`.
    #[must_use]
    pub const unsafe fn new_unchecked(rank: u32, size: PartitionSize) -> Self {
        Self { rank, size }
    }

    #[must_use]
    pub const fn root(size: PartitionSize) -> Self {
        Self { rank: 0, size }
    }

    #[must_use]
    pub const fn monolithic() -> Self {
        Self::root(PartitionSize::MONOLITHIC)
    }

    #[must_use]
    pub const fn rank(self) -> u32 {
        self.rank
    }

    #[must_use]
    pub const fn size(self) -> PartitionSize {
        self.size
    }

    #[must_use]
    pub const fn is_root(self) -> bool {
        self.rank == 0_u32
    }
}

impl TryFrom<PartitionRaw> for Partition {
    type Error = PartitionRankOutOfBounds;

    fn try_from(raw: PartitionRaw) -> Result<Self, Self::Error> {
        Self::try_new(raw.rank, raw.size)
    }
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename = "Partition")]
struct PartitionRaw {
    rank: u32,
    size: PartitionSize,
}

#[expect(clippy::module_name_repetitions)]
#[expect(clippy::unsafe_derive_deserialize)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PartitionSize(pub NonZeroU32);

impl PartitionSize {
    pub const MONOLITHIC: Self = Self(NonZeroU32::MIN);

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0.get()
    }

    #[must_use]
    pub const fn is_monolithic(self) -> bool {
        self.0.get() == 1
    }

    #[must_use]
    pub fn partitions(self) -> impl ExactSizeIterator<Item = Partition> {
        // Safety: rank is in bounds
        (0..self.get()).map(move |rank| unsafe { Partition::new_unchecked(rank, self) })
    }
}

impl fmt::Display for PartitionSize {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_fmt(format_args!("{}", self.0))
    }
}
