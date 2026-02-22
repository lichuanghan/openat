//! Common utilities for openat
//!
//! Provides shared error types, macros, and helper functions

pub mod error;
pub mod macros;
pub mod utils;

pub use error::{CommonError, CommonResult};
