#![deny(clippy::pedantic)]
#![no_std]
#![feature(const_float_bits_conv)]
#![feature(const_float_classify)]
#![feature(const_type_name)]

#[macro_use]
extern crate const_type_layout;

mod closed_open_unit_f64;
mod closed_unit_f64;
mod non_negative_f64;
mod non_positive_f64;
mod non_zero_one_u64;
mod off_by_one_u32;
mod off_by_one_u64;
mod open_closed_unit_f64;
mod positive_f64;

pub use closed_open_unit_f64::ClosedOpenUnitF64;
pub use closed_unit_f64::ClosedUnitF64;
pub use non_negative_f64::NonNegativeF64;
pub use non_positive_f64::NonPositiveF64;
pub use non_zero_one_u64::NonZeroOneU64;
pub use off_by_one_u32::OffByOneU32;
pub use off_by_one_u64::OffByOneU64;
pub use open_closed_unit_f64::OpenClosedUnitF64;
pub use positive_f64::PositiveF64;
