use core::{
    cmp::Ordering,
    convert::TryFrom,
    fmt,
    hash::{Hash, Hasher},
    num::NonZeroU32,
    ops::{Div, Mul},
};

use serde::{Deserialize, Serialize};

use crate::{ClosedOpenUnitF64, OpenClosedUnitF64};

#[derive(Debug)]
#[expect(clippy::module_name_repetitions)]
pub struct ClosedUnitF64Error(f64);

impl fmt::Display for ClosedUnitF64Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{} is not in [0.0, 1.0].", self.0)
    }
}

#[expect(clippy::unsafe_derive_deserialize)]
#[derive(Copy, Clone, Serialize, Deserialize, TypeLayout)]
#[repr(transparent)]
#[serde(try_from = "f64", into = "f64")]
pub struct ClosedUnitF64(f64);

impl fmt::Display for ClosedUnitF64 {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, fmt)
    }
}

impl TryFrom<f64> for ClosedUnitF64 {
    type Error = ClosedUnitF64Error;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ClosedUnitF64> for f64 {
    fn from(val: ClosedUnitF64) -> Self {
        val.get()
    }
}

impl fmt::Debug for ClosedUnitF64 {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        struct ClosedUnitF64Range(f64);

        impl fmt::Debug for ClosedUnitF64Range {
            fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                write!(fmt, "0.0 <= {} <= 1.0", self.0)
            }
        }

        fmt.debug_tuple("ClosedUnitF64")
            .field(&ClosedUnitF64Range(self.0))
            .finish()
    }
}

impl ClosedUnitF64 {
    /// # Errors
    ///
    /// Returns `ClosedUnitF64Error` if not `0.0 <= value <= 1.0`
    pub const fn new(value: f64) -> Result<Self, ClosedUnitF64Error> {
        if value >= 0.0_f64 && value <= 1.0_f64 {
            Ok(Self(value))
        } else {
            Err(ClosedUnitF64Error(value))
        }
    }

    /// # Safety
    ///
    /// Only safe iff `0.0 <= value <= 1.0`
    #[must_use]
    pub const unsafe fn new_unchecked(value: f64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn zero() -> Self {
        Self(0.0_f64)
    }

    #[must_use]
    pub const fn half() -> Self {
        Self(0.5_f64)
    }

    #[must_use]
    pub const fn one() -> Self {
        Self(1.0_f64)
    }

    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }

    #[must_use]
    pub const fn one_minus(self) -> Self {
        Self(1.0_f64 - self.0)
    }
}

impl From<OpenClosedUnitF64> for ClosedUnitF64 {
    fn from(value: OpenClosedUnitF64) -> Self {
        Self(value.get())
    }
}

impl From<ClosedOpenUnitF64> for ClosedUnitF64 {
    fn from(value: ClosedOpenUnitF64) -> Self {
        Self(value.get())
    }
}

impl PartialEq for ClosedUnitF64 {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl Eq for ClosedUnitF64 {}

impl PartialOrd for ClosedUnitF64 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ClosedUnitF64 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl Hash for ClosedUnitF64 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl PartialEq<f64> for ClosedUnitF64 {
    fn eq(&self, other: &f64) -> bool {
        self.0.eq(other)
    }
}

impl PartialOrd<f64> for ClosedUnitF64 {
    fn partial_cmp(&self, other: &f64) -> Option<Ordering> {
        self.0.partial_cmp(other)
    }
}

impl Mul for ClosedUnitF64 {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Self(self.0 * other.0)
    }
}

impl Div<NonZeroU32> for ClosedUnitF64 {
    type Output = Self;

    fn div(self, rhs: NonZeroU32) -> Self::Output {
        Self(self.0 / f64::from(rhs.get()))
    }
}

impl Mul<ClosedOpenUnitF64> for ClosedUnitF64 {
    type Output = ClosedOpenUnitF64;

    fn mul(self, other: ClosedOpenUnitF64) -> ClosedOpenUnitF64 {
        unsafe { ClosedOpenUnitF64::new_unchecked(self.0 * other.get()) }
    }
}

impl Mul<OpenClosedUnitF64> for ClosedUnitF64 {
    type Output = OpenClosedUnitF64;

    fn mul(self, other: OpenClosedUnitF64) -> OpenClosedUnitF64 {
        unsafe { OpenClosedUnitF64::new_unchecked(self.0 * other.get()) }
    }
}
