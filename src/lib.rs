//! # cargo-bsize
//!
//! Analyze Rust binary size and propose size-reducing changes.

use bpaf::Bpaf;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Analyze Rust binary size and propose size-reducing changes.
///
/// Parsed from `argv` by `bpaf`. The doc comments on individual fields become
/// the `--help` text users see, so keep them user-facing.
///
/// The "batteries" `cargo_helper` strips the leading `bsize` argument when
/// invoked as `cargo bsize`, so this struct sees the same shape either way.
/// See <https://docs.rs/bpaf/latest/bpaf/batteries/fn.cargo_helper.html>.
#[derive(Debug, Clone, Bpaf)]
#[bpaf(options("bsize"), version(VERSION))]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "no flags yet; fields land here as they are added"
)]
pub struct CargoBsizeOptions {}
