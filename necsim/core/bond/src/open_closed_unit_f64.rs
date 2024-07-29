use core::{
    cmp::Ordering,
    convert::TryFrom,
    fmt,
    hash::{Hash, Hasher},
};

use necsim_core_maths::MathsCore;
use serde::{Deserialize, Serialize};

use crate::NonPositiveF64;

#[derive(Debug)]
#[expect(clippy::module_name_repetitions)]
pub struct OpenClosedUnitF64Error(f64);

impl fmt::Display for OpenClosedUnitF64Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{} is not in (0.0, 1.0].", self.0)
    }
}

#[expect(clippy::unsafe_derive_deserialize)]
#[derive(Copy, Clone, Deserialize, Serialize, TypeLayout)]
#[repr(transparent)]
#[serde(try_from = "f64", into = "f64")]
pub struct OpenClosedUnitF64(f64);

impl fmt::Display for OpenClosedUnitF64 {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, fmt)
    }
}

impl TryFrom<f64> for OpenClosedUnitF64 {
    type Error = OpenClosedUnitF64Error;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<OpenClosedUnitF64> for f64 {
    fn from(val: OpenClosedUnitF64) -> Self {
        val.get()
    }
}

impl fmt::Debug for OpenClosedUnitF64 {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        struct OpenClosedUnitF64Range(f64);

        impl fmt::Debug for OpenClosedUnitF64Range {
            fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                write!(fmt, "0.0 < {} <= 1.0", self.0)
            }
        }

        fmt.debug_tuple("OpenClosedUnitF64")
            .field(&OpenClosedUnitF64Range(self.0))
            .finish()
    }
}

impl OpenClosedUnitF64 {
    /// # Errors
    ///
    /// Returns `OpenClosedUnitF64Error` if not `0.0 < value <= 1.0`
    pub const fn new(value: f64) -> Result<Self, OpenClosedUnitF64Error> {
        if value > 0.0 && value <= 1.0 {
            Ok(Self(value))
        } else {
            Err(OpenClosedUnitF64Error(value))
        }
    }

    /// # Safety
    ///
    /// Only safe iff `0.0 < value <= 1.0`
    #[must_use]
    pub const unsafe fn new_unchecked(value: f64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }

    #[must_use]
    pub fn ln<M: MathsCore>(self) -> NonPositiveF64 {
        unsafe { NonPositiveF64::new_unchecked(M::ln(self.0)) }
    }
}

impl PartialEq for OpenClosedUnitF64 {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl Eq for OpenClosedUnitF64 {}

impl PartialOrd for OpenClosedUnitF64 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OpenClosedUnitF64 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl Hash for OpenClosedUnitF64 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl PartialEq<f64> for OpenClosedUnitF64 {
    fn eq(&self, other: &f64) -> bool {
        self.0.eq(other)
    }
}

impl PartialOrd<f64> for OpenClosedUnitF64 {
    fn partial_cmp(&self, other: &f64) -> Option<Ordering> {
        self.0.partial_cmp(other)
    }
}
