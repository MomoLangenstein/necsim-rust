#![deny(clippy::pedantic)]
#![no_std]
#![feature(const_type_name)]
#![cfg_attr(target_os = "cuda", feature(asm_experimental_arch))]
#![cfg_attr(target_os = "cuda", feature(const_float_bits_conv))]
#![allow(internal_features)]
#![feature(core_intrinsics)]
#![allow(incomplete_features)]
#![feature(specialization)]

extern crate alloc;

#[macro_use]
extern crate const_type_layout;

#[cfg_attr(target_os = "cuda", macro_use)]
extern crate contracts;

pub mod cogs;
pub mod event_buffer;
pub mod value_buffer;

mod utils;
