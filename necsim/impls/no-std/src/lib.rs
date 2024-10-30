#![deny(clippy::pedantic)]
#![no_std]
#![feature(iter_advance_by)]
#![feature(extract_if)]
#![feature(const_type_name)]
#![feature(negative_impls)]
#![feature(impl_trait_in_assoc_type)]
#![feature(inline_const_pat)]
#![allow(incomplete_features)]
#![feature(specialization)]

extern crate alloc;

#[macro_use]
extern crate const_type_layout;

#[macro_use]
extern crate contracts;

#[macro_use]
extern crate log;

pub mod alias;
pub mod array2d;
pub mod cache;
pub mod cogs;
pub mod decomposition;
pub mod parallelisation;
