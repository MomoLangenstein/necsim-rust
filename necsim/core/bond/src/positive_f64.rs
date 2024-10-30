use core::{
    cmp::Ordering,
    convert::TryFrom,
    fmt,
    hash::{Hash, Hasher},
    num::{NonZeroU32, NonZeroU64},
    ops::{Add, Mul},
};

use serde::{Deserialize, Serialize};

use crate::NonNegativeF64;

#[derive(Debug)]
#[expect(clippy::module_name_repetitions)]
pub struct PositiveF64Error(f64);

impl fmt::Display for PositiveF64Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{} is not positive.", self.0)
    }
}

#[expect(clippy::unsafe_derive_deserialize)]
#[derive(Copy, Clone, Serialize, Deserialize, TypeLayout)]
#[repr(transparent)]
#[serde(try_from = "f64", into = "f64")]
pub struct PositiveF64(f64);

impl fmt::Display for PositiveF64 {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, fmt)
    }
}

impl TryFrom<f64> for PositiveF64 {
    type Error = PositiveF64Error;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<PositiveF64> for f64 {
    fn from(val: PositiveF64) -> Self {
        val.get()
    }
}

impl fmt::Debug for PositiveF64 {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        struct PositiveF64Range(f64);

        impl fmt::Debug for PositiveF64Range {
            fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                write!(fmt, "0.0 < {}", self.0)
            }
        }

        fmt.debug_tuple("PositiveF64")
            .field(&PositiveF64Range(self.0))
            .finish()
    }
}

impl PositiveF64 {
    /// # Errors
    ///
    /// Returns `PositiveF64Error` if not `0.0 < value`
    pub const fn new(value: f64) -> Result<Self, PositiveF64Error> {
        if value > 0.0 {
            Ok(Self(value))
        } else {
            Err(PositiveF64Error(value))
        }
    }

    /// # Safety
    ///
    /// Only safe iff `0.0 < value`
    #[must_use]
    pub const unsafe fn new_unchecked(value: f64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn one() -> Self {
        Self(1.0)
    }

    #[must_use]
    pub const fn infinity() -> Self {
        Self(f64::INFINITY)
    }

    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }

    #[must_use]
    #[inline]
    pub const fn max_after(before: NonNegativeF64, value: NonNegativeF64) -> Self {
        if value.get() > before.get() || before.get().is_nan() {
            Self(value.get())
        } else if before.get().is_infinite() {
            Self(f64::INFINITY)
        } else {
            // also catches `value.get().is_nan()`
            // Next `f64` value that is larger than `before`
            Self(f64::from_bits(before.get().to_bits() + 1))
        }
    }
}

impl From<NonZeroU32> for PositiveF64 {
    fn from(value: NonZeroU32) -> Self {
        Self(f64::from(value.get()))
    }
}

impl From<NonZeroU64> for PositiveF64 {
    #[expect(clippy::cast_precision_loss)]
    fn from(value: NonZeroU64) -> Self {
        Self(value.get() as f64)
    }
}

impl PartialEq for PositiveF64 {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl Eq for PositiveF64 {}

impl PartialOrd for PositiveF64 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PositiveF64 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl Hash for PositiveF64 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl PartialEq<NonNegativeF64> for PositiveF64 {
    fn eq(&self, other: &NonNegativeF64) -> bool {
        self.0.eq(&other.get())
    }
}

impl PartialOrd<NonNegativeF64> for PositiveF64 {
    fn partial_cmp(&self, other: &NonNegativeF64) -> Option<Ordering> {
        self.0.partial_cmp(&other.get())
    }
}

impl PartialEq<f64> for PositiveF64 {
    fn eq(&self, other: &f64) -> bool {
        self.0.eq(other)
    }
}

impl PartialOrd<f64> for PositiveF64 {
    fn partial_cmp(&self, other: &f64) -> Option<Ordering> {
        self.0.partial_cmp(other)
    }
}

impl Mul for PositiveF64 {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Self(self.0 * other.0)
    }
}

impl Add<NonNegativeF64> for PositiveF64 {
    type Output = Self;

    fn add(self, other: NonNegativeF64) -> Self {
        Self(self.0 + other.get())
    }
}

impl Add<PositiveF64> for NonNegativeF64 {
    type Output = PositiveF64;

    fn add(self, other: PositiveF64) -> PositiveF64 {
        PositiveF64(self.get() + other.0)
    }
}
