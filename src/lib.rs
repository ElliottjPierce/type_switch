#![doc = include_str!("../README.md")]
#![no_std]

mod bools;

pub use bools::*;

pub(crate) trait Sealed {}
