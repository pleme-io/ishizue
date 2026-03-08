//! Ishizue (礎) — core utilities and foundation library for Rust-based Neovim plugins.
//!
//! Part of the blnvim-ng distribution — a Rust-native Neovim plugin suite.
//! Built with [`nvim-oxi`](https://github.com/noib3/nvim-oxi) for zero-cost
//! Neovim API bindings.
//!
//! # Modules
//!
//! - [`path`] — path manipulation utilities (normalize, join, expand `~`, relative paths)
//! - [`job`] — subprocess runner (spawn, capture stdout/stderr, kill)
//! - [`debounce`] — debounce, throttle, once-cell, and once-flag timing primitives
//! - [`strings`] — truncate, pad, split, trim string helpers

pub mod debounce;
pub mod job;
pub mod path;
pub mod strings;

/// Re-export the tane SDK for downstream consumers.
pub use tane;
