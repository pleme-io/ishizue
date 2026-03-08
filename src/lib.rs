//! Ishizue (礎) — core utilities and foundation library for Rust-based Neovim plugins
//!
//! Part of the blnvim-ng distribution — a Rust-native Neovim plugin suite.
//! Built with [`nvim-oxi`](https://github.com/noib3/nvim-oxi) for zero-cost
//! Neovim API bindings.

use nvim_oxi as oxi;

#[oxi::plugin]
fn ishizue() -> oxi::Result<()> {
    Ok(())
}
