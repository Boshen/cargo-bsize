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
cargo bsize --format=json             # machine-readable output
cargo bsize --limit=50                 # keep more ranked entries
cargo bsize --baseline path/to/binary  # report what changed
cargo bsize --llvm-ir                  # attribute LLVM IR to generic families
cargo bsize --what-if                  # measure size-focused build settings
```

`--llvm-ir` and `--what-if` perform additional builds and can be slow. Cargo's
`--locked`, `--offline`, and `--frozen` modes are also supported.

## Report

The report includes:

- binary sections and shipped size;
- largest functions, data symbols, types, crates, and generic families;
- duplicate dependencies, functions, and read-only data;
- inlined code, panic and formatting overhead, and dynamic dispatch;
- assembly patterns, source-line attribution, and retained-size analysis;
- optional baseline, LLVM IR, and measured what-if comparisons.

Build artifacts are written to `target/bsize`, leaving the project's normal
release cache untouched. Debug information and symbols are retained for analysis
but excluded from the reported shipped size.

## Prior art

- [cargo-bloat](https://crates.io/crates/cargo-bloat)
- [cargo-llvm-lines](https://github.com/dtolnay/cargo-llvm-lines)

## License

MIT. See [LICENSE](LICENSE).

[Sponsored by Oxc](https://oxc.rs/sponsor).
