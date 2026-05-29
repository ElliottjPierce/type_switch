#![doc = include_str!("../README.md")]

mod bools;

pub use bools::*;

pub(crate) trait Sealed {}
