use serde::{Deserialize, Serialize};

#[allow(clippy::module_name_repetitions)] // FIXME: use expect
#[derive(
    Eq, PartialEq, PartialOrd, Ord, Clone, Hash, Debug, Serialize, Deserialize, TypeLayout,
)]
#[cfg_attr(feature = "cuda", derive(rust_cuda::lend::LendRustToCuda))]
#[repr(C)]
#[cfg_attr(feature = "cuda", cuda(ignore))]
#[serde(deny_unknown_fields)]
pub struct Location {
    x: u32,
    y: u32,
}

impl Location {
    #[must_use]
    pub const fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }

    #[must_use]
    pub const fn x(&self) -> u32 {
        self.x
    }

    #[must_use]
    pub const fn y(&self) -> u32 {
        self.y
    }
}

impl From<IndexedLocation> for Location {
    fn from(indexed_location: IndexedLocation) -> Location {
        indexed_location.location
    }
}

#[derive(
    Eq, PartialEq, PartialOrd, Ord, Clone, Hash, Debug, Serialize, Deserialize, TypeLayout,
)]
#[allow(clippy::module_name_repetitions)] // FIXME: use expect
#[cfg_attr(feature = "cuda", derive(rust_cuda::lend::LendRustToCuda))]
#[repr(C)]
#[cfg_attr(feature = "cuda", cuda(ignore))]
#[serde(from = "IndexedLocationRaw", into = "IndexedLocationRaw")]
pub struct IndexedLocation {
    #[cfg_attr(feature = "cuda", cuda(embed))]
    location: Location,
    index: u32,
}

impl IndexedLocation {
    #[must_use]
    pub const fn new(location: Location, index: u32) -> Self {
        Self { location, index }
    }

    #[must_use]
    pub const fn location(&self) -> &Location {
        &self.location
    }

    #[must_use]
    pub const fn index(&self) -> u32 {
        self.index
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename = "IndexedLocation")]
struct IndexedLocationRaw {
    x: u32,
    y: u32,
    #[serde(alias = "i")]
    index: u32,
}

impl From<IndexedLocation> for IndexedLocationRaw {
    fn from(val: IndexedLocation) -> Self {
        Self {
            x: val.location.x,
            y: val.location.y,
            index: val.index,
        }
    }
}

impl From<IndexedLocationRaw> for IndexedLocation {
    fn from(raw: IndexedLocationRaw) -> Self {
        Self {
            location: Location::new(raw.x, raw.y),
            index: raw.index,
        }
    }
}
