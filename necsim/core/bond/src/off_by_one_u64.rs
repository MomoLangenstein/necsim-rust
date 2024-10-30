use core::{
    convert::TryFrom,
    fmt,
    num::{NonZeroU32, NonZeroU64},
    ops::{Add, Mul},
};

use serde::{Deserialize, Deserializer, Serialize};

use crate::{ClosedUnitF64, OffByOneU32};

#[derive(Debug)]
#[expect(clippy::module_name_repetitions)]
pub struct OffByOneU64Error(u128);

impl fmt::Display for OffByOneU64Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{} is not in {{1, .., 2^64}}.", self.0)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, TypeLayout)]
#[repr(transparent)]
#[serde(try_from = "u128", into = "u128")]
pub struct OffByOneU64(u64);

impl fmt::Display for OffByOneU64 {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.get(), fmt)
    }
}

impl OffByOneU64 {
    /// # Errors
    ///
    /// Returns `OffByOneU64Error` if not `1 <= value <= 2^64`
    pub const fn new(value: u128) -> Result<Self, OffByOneU64Error> {
        // match u64::try_from(value.wrapping_sub(1)) {
        //     Ok(value) => Ok(Self(value)),
        //     Err(_) => Err(OffByOneU64Error(value)),
        // }
        match value.wrapping_sub(1) {
            #[expect(clippy::cast_possible_truncation)]
            value if value < (u64::MAX as u128) => Ok(Self(value as u64)),
            _ => Err(OffByOneU64Error(value)),
        }
    }

    #[must_use]
    /// Creates a off-by-one u64 without checking the value.
    ///
    /// # Safety
    ///
    /// The value must be in {1, .., 2^64}.
    pub const unsafe fn new_unchecked(value: u128) -> Self {
        #[expect(clippy::cast_possible_truncation)]
        Self(value.wrapping_sub(1) as u64)
    }

    #[must_use]
    pub const fn get(self) -> u128 {
        // u128::from(self)
        (self.0 as u128) + 1_u128
    }

    #[must_use]
    pub const fn sub_one(self) -> u64 {
        self.0
    }

    #[must_use]
    pub const fn add_incl(self, other: u64) -> u64 {
        other.wrapping_add(self.0)
    }

    #[must_use]
    pub const fn add_excl(self, other: u64) -> u64 {
        other.wrapping_add(self.0).wrapping_add(1)
    }

    #[must_use]
    pub const fn one() -> Self {
        Self(0)
    }

    #[must_use]
    pub const fn max() -> Self {
        Self(u64::MAX)
    }

    #[must_use]
    pub const fn inv(self) -> u64 {
        u64::MAX - self.0
    }
}

impl TryFrom<u128> for OffByOneU64 {
    type Error = OffByOneU64Error;

    fn try_from(value: u128) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<NonZeroU32> for OffByOneU64 {
    fn from(val: NonZeroU32) -> Self {
        Self(u64::from(val.get()) - 1)
    }
}

impl From<OffByOneU32> for OffByOneU64 {
    fn from(val: OffByOneU32) -> Self {
        Self(val.get() - 1)
    }
}

impl From<NonZeroU64> for OffByOneU64 {
    fn from(val: NonZeroU64) -> Self {
        Self(val.get() - 1)
    }
}

impl From<OffByOneU64> for NonZeroU64 {
    fn from(val: OffByOneU64) -> Self {
        // Safety: always at least 1, max case undefined behaviour
        unsafe { NonZeroU64::new_unchecked(val.0 + 1) }
    }
}

impl From<OffByOneU64> for f64 {
    #[expect(clippy::cast_precision_loss)]
    fn from(val: OffByOneU64) -> Self {
        (val.0 as f64) + 1.0_f64
    }
}

impl From<OffByOneU64> for u128 {
    fn from(val: OffByOneU64) -> Self {
        u128::from(val.0) + 1_u128
    }
}

impl<'de> Deserialize<'de> for OffByOneU64 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(u128::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl Add for OffByOneU64 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self((self.0 + 1) + (other.0 + 1) - 1)
    }
}

impl Mul for OffByOneU64 {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Self((self.0 + 1) * (other.0 + 1) - 1)
    }
}

impl Mul<ClosedUnitF64> for OffByOneU64 {
    type Output = Self;

    fn mul(self, other: ClosedUnitF64) -> Self::Output {
        #[expect(clippy::cast_possible_truncation)]
        #[expect(clippy::cast_sign_loss)]
        #[expect(clippy::cast_precision_loss)]
        Self(((((self.get() as f64) * other.get()) as u128) - 1) as u64)
    }
}
