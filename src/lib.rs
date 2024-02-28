#![cfg_attr(feature = "wasm", no_std)]
#![cfg_attr(feature = "wasm", allow(incomplete_features))]
// #![deny(missing_docs)]
#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

#[cfg_attr(feature = "wasm", macro_use)]
#[cfg(feature = "wasm")]
extern crate alloc;
#[cfg(feature = "wasm")]
extern crate no_std_compat as std;

/// The actual Api to interact with the hub.
#[cfg(any(feature = "online", feature = "tokio", feature = "wasm"))]
pub mod api;
mod cache;
pub mod types;

pub use cache::{Cache, Repo, RepoType};
