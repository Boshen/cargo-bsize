# cargo-bsize

Analyze Rust binary size and find practical ways to reduce it.

`cargo-bsize` builds one release binary and reports where its shipped bytes come
from. It combines sections, symbols, debug information, and assembly into a
ranked report designed to guide source-code changes.

## Installation

```bash
cargo install cargo-bsize --locked
```

## Usage

```bash
cargo bsize                 # analyze the workspace's only binary
cargo bsize --bin my-app    # select a binary in a multi-binary workspace
cargo bsize path/to/project # analyze another project
```

Exactly one binary is analyzed. If a workspace contains several, `--bin` is
required. A library-only workspace still reports duplicate dependencies.

Common options:

```bash
cargo bsize --limit=50                 # keep more ranked entries
cargo bsize --baseline path/to/binary  # report what changed
cargo bsize --llvm-ir                  # attribute LLVM IR to generic families
cargo bsize --mono                     # rank generic definitions by monomorphization cost
cargo bsize --remarks                  # list the loops the optimizer unrolled or vectorized
cargo bsize --what-if                  # measure size-focused build settings
cargo bsize --what-if --levers=all     # every lever, or a comma-separated list
```

`--llvm-ir`, `--mono`, `--remarks`, and `--what-if` perform additional builds
and can be slow (`--mono` reads `-Zdump-mono-stats` and `--remarks` reads
`-Cremark` through `-Zremark-dir`, both via `RUSTC_BOOTSTRAP=1`). By
default `--what-if` measures `opt-level="z"` and `panic="abort"`; `--levers`
selects others, among them `lto="fat"`, `codegen-units=1`, `fmt-debug=none`,
`location-detail=none`, `share-generics=yes`, `virtual-function-elimination`,
`force-unwind-tables=no`, `no-vectorize`, `build-std`, `optimize-for-size`,
`panic="immediate-abort"`, and `min-size` (all of them at once). Nightly-only
levers run through `RUSTC_BOOTSTRAP=1` on the pinned toolchain; a lever that
fails to build is reported as skipped. Each lever also lists the functions that
moved most under it. Cargo's `--locked`, `--offline`, and `--frozen` modes are
also supported.

## Report

The report is Markdown, written to standard output: one section per view, each
a ranked table with sizes and shares of the shipped size, names in code spans,
and a contents line linking the sections. Save it as `cargo bsize > report.md`
to read it rendered or hand it to an agent.

The report includes:

- binary sections and shipped size;
- largest functions, data symbols, types (with enum variants and padding),
  crates, and generic families, each function with its definition site;
- duplicate dependencies costed per version, dependency features, duplicate
  functions and read-only data;
- inlined code, panic and formatting overhead, and dynamic dispatch;
- constant data by content: panic locations by file, strings, vtables, lookup
  and jump tables, and the functions carrying them; the pointer slots in data
  and what the loader charges for them;
- assembly patterns, source-line attribution with the source text, and
  retained-size analysis;
- optional baseline, LLVM IR, monomorphization cost, expanded-loop, and
  measured what-if comparisons.

Build artifacts are written to `target/bsize`, leaving the project's normal
release cache untouched. Debug information and symbols are retained for analysis
but excluded from the reported shipped size.

## Prior art

- [cargo-bloat](https://crates.io/crates/cargo-bloat)
- [cargo-llvm-lines](https://github.com/dtolnay/cargo-llvm-lines)

## License

MIT. See [LICENSE](LICENSE).

[Sponsored by Oxc](https://oxc.rs/sponsor).
