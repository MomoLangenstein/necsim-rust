use std::{cmp::Ordering, fmt};

use necsim_core_bond::PositiveF64;
use serde::{Deserialize, Serialize};

pub mod recorder;
pub mod replay;

#[derive(Serialize, Deserialize, PartialEq)]
#[expect(clippy::module_name_repetitions)]
pub struct EventLogHeader {
    min_time: PositiveF64,
    max_time: PositiveF64,

    length: usize,

    with_speciation: bool,
    with_dispersal: bool,
}

impl fmt::Debug for EventLogHeader {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct(stringify!(EventLogHeader))
            .field("min_time", &self.min_time)
            .field("max_time", &self.max_time)
            .field("length", &self.length)
            .finish_non_exhaustive()
    }
}

impl EventLogHeader {
    #[must_use]
    pub fn new(
        min_time: PositiveF64,
        max_time: PositiveF64,
        length: usize,
        with_speciation: bool,
        with_dispersal: bool,
    ) -> Self {
        Self {
            min_time,
            max_time,
            length,
            with_speciation,
            with_dispersal,
        }
    }

    #[must_use]
    pub fn min_time(&self) -> PositiveF64 {
        self.min_time
    }

    #[must_use]
    pub fn max_time(&self) -> PositiveF64 {
        self.max_time
    }

    #[must_use]
    pub fn length(&self) -> usize {
        self.length
    }

    #[must_use]
    pub fn with_speciation(&self) -> bool {
        self.with_speciation
    }

    #[must_use]
    pub fn with_dispersal(&self) -> bool {
        self.with_dispersal
    }
}

impl Eq for EventLogHeader {}

impl PartialOrd for EventLogHeader {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.max_time < other.min_time {
            Some(Ordering::Less)
        } else if self.min_time > other.max_time {
            Some(Ordering::Greater)
        } else {
            None
        }
    }
}
