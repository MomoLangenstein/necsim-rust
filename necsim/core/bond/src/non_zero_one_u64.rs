use core::{convert::TryFrom, fmt, num::NonZeroU64};

use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug)]
#[expect(clippy::module_name_repetitions)]
pub struct NonZeroOneU64Error(u64);

impl fmt::Display for NonZeroOneU64Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{} is zero or one.", self.0)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, TypeLayout)]
#[repr(transparent)]
#[serde(try_from = "u64", into = "u64")]
pub struct NonZeroOneU64(NonZeroU64);

impl fmt::Display for NonZeroOneU64 {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, fmt)
    }
}

impl NonZeroOneU64 {
    /// # Errors
    ///
    /// Returns `NonZeroOneU64Error` if not `1 < value`
    pub const fn new(value: u64) -> Result<Self, NonZeroOneU64Error> {
        match NonZeroU64::new(value) {
            Some(inner) if value > 1 => Ok(Self(inner)),
            _ => Err(NonZeroOneU64Error(value)),
        }
    }

    #[must_use]
    /// Creates a non-zero, non-one u64 without checking the value.
    ///
    /// # Safety
    ///
    /// The value must not be zero or one.
    pub const unsafe fn new_unchecked(value: u64) -> Self {
        Self(NonZeroU64::new_unchecked(value))
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl TryFrom<u64> for NonZeroOneU64 {
    type Error = NonZeroOneU64Error;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<NonZeroOneU64> for u64 {
    fn from(val: NonZeroOneU64) -> Self {
        val.get()
    }
}

impl<'de> Deserialize<'de> for NonZeroOneU64 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(u64::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}
