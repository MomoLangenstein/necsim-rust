use core::{
    cmp::Ordering,
    convert::TryFrom,
    fmt,
    hash::{Hash, Hasher},
    ops::Neg,
};

use serde::{Deserialize, Serialize};

use crate::NonNegativeF64;

#[derive(Debug)]
#[expect(clippy::module_name_repetitions)]
pub struct NonPositiveF64Error(f64);

impl fmt::Display for NonPositiveF64Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{} is positive.", self.0)
    }
}

#[expect(clippy::unsafe_derive_deserialize)]
#[derive(Copy, Clone, Serialize, Deserialize, TypeLayout)]
#[repr(transparent)]
#[serde(try_from = "f64", into = "f64")]
pub struct NonPositiveF64(f64);

impl fmt::Display for NonPositiveF64 {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, fmt)
    }
}

impl TryFrom<f64> for NonPositiveF64 {
    type Error = NonPositiveF64Error;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<NonPositiveF64> for f64 {
    fn from(val: NonPositiveF64) -> Self {
        val.get()
    }
}

impl fmt::Debug for NonPositiveF64 {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        struct NonPositiveF64Range(f64);

        impl fmt::Debug for NonPositiveF64Range {
            fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                write!(fmt, "{} <= 0.0", self.0)
            }
        }

        fmt.debug_tuple("NonPositiveF64")
            .field(&NonPositiveF64Range(self.0))
            .finish()
    }
}

impl NonPositiveF64 {
    /// # Errors
    ///
    /// Returns `NonPositiveF64Error` if not `value <= 0.0`
    pub const fn new(value: f64) -> Result<Self, NonPositiveF64Error> {
        if value <= 0.0 {
            Ok(Self(value))
        } else {
            Err(NonPositiveF64Error(value))
        }
    }

    /// # Safety
    ///
    /// Only safe iff `value <= 0.0`
    #[must_use]
    pub const unsafe fn new_unchecked(value: f64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn zero() -> Self {
        Self(0.0_f64)
    }

    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl PartialEq for NonPositiveF64 {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl Eq for NonPositiveF64 {}

impl PartialOrd for NonPositiveF64 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NonPositiveF64 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl Hash for NonPositiveF64 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl PartialEq<f64> for NonPositiveF64 {
    fn eq(&self, other: &f64) -> bool {
        self.0.eq(other)
    }
}

impl PartialOrd<f64> for NonPositiveF64 {
    fn partial_cmp(&self, other: &f64) -> Option<Ordering> {
        self.0.partial_cmp(other)
    }
}

impl Neg for NonPositiveF64 {
    type Output = NonNegativeF64;

    fn neg(self) -> Self::Output {
        unsafe { NonNegativeF64::new_unchecked(-self.0) }
    }
}
