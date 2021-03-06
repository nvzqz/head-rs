//! Common types with inline headers, brought to you by
//! [@NikolaiVazquez](https://twitter.com/NikolaiVazquez).
//!
//! ## License
//!
//! This project is released under either the
//! [MIT License](https://github.com/nvzqz/head-rs/blob/main/LICENSE-MIT) or
//! [Apache License (Version 2.0)](https://github.com/nvzqz/head-rs/blob/main/LICENSE-APACHE),
//! at your choosing.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod slice;

pub use slice::HeaderSlice;
