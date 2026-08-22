# cargo-bsize

Analyze Rust binary size.

## Install

```sh
cargo binstall cargo-bsize
```

## Use

```sh
cargo bsize
cargo bsize --cdylib rolldown_binding path/to/rolldown
cargo bsize --staticlib my_ffi_lib path/to/project
```

A `staticlib` is an archive of unlinked objects, so it is first linked into a
shared library with `cc` (every member kept, dead code stripped, symbols the
host provides left unresolved) and that is what gets analyzed.

The output is a Markdown report that can be fed into an agent. It shows:

- Binary size by object-file section
- Linker input, archive-member, and exported-root provenance
- Code size by Rust function
- Static data size
- Rust type layout
- Code size by crate
- Code size by generic monomorphization
- Duplicate Cargo dependencies
- Duplicate function bodies
- Duplicate static data
- Inlined Rust code
- LLVM inlining, instruction-growth, and stack-frame decisions
- Panic-path overhead
- Rust formatting overhead
- Trait-object dynamic dispatch
- Assembly instruction patterns
- Rust source-line attribution
- Function retained size
- And more.

## Prior art

- [cargo-bloat](https://crates.io/crates/cargo-bloat)
- [cargo-llvm-lines](https://github.com/dtolnay/cargo-llvm-lines)
- [cargo-show-asm](https://crates.io/crates/cargo-show-asm)

## [Sponsors](https://github.com/sponsors/boshen)

<p align="center">
  <a href="https://github.com/sponsors/boshen">
    <img src="https://raw.githubusercontent.com/oxc-project/sponsors/main/sponsors.svg" alt="Sponsors" />
  </a>
</p>
