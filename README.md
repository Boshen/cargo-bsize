# cargo-bsize

Analyze Rust binary size.

## Install

```sh
cargo binstall cargo-bsize
```

## Use

```sh
cargo bsize
```

The output is a Markdown report that can be fed into an agent. It shows:

- Binary size by object-file section
- Code size by Rust function
- Static data size
- Rust type layout
- Code size by crate
- Code size by generic monomorphization
- Duplicate Cargo dependencies
- Duplicate function bodies
- Duplicate static data
- Inlined Rust code
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
