# cargo-bsize

Analyze Rust binary size and propose size-reducing changes.

```bash
cargo bsize
```

Builds the project's release binaries and reports what is in them, then reports
duplicate dependencies. `--format=json` emits the same data for machine
consumers.

## Binary sections

```
target/bsize/release/cargo-bsize (macho)
       1.1 MiB  total
     816.0 KiB  shipped, excluding symbols and debug info

     651.2 KiB  58.7%  code
     292.5 KiB  26.4%  symbols
      85.4 KiB   7.7%  read-only data
      39.6 KiB   3.6%  unwind
       3.4 KiB   0.3%  data
      36.4 KiB   3.3%  other (headers, padding, code signature)

     648.8 KiB  58.5%  __TEXT,__text
     292.5 KiB  26.4%  __LINKEDIT
      53.0 KiB   4.8%  __TEXT,__const
      ...
```

The build forces `debug = 2` and `strip = "none"` over the project's release
profile, because later analyses need the symbol table. That inflates the binary,
so the report separates `total` from `shipped`. It goes to its own target
directory, since overriding profile settings would otherwise invalidate the
project's `target/release` cache on every run.

Sizes are file ranges, never virtual sizes — Mach-O's `__PAGEZERO` claims 4 GB
of address space and occupies no file bytes. Sections do not cover a whole file
either, so sectionless segments are reported too; on Mach-O `__LINKEDIT` is
routinely a quarter of the binary and would otherwise vanish. `total`,
`accounted`, and `other` always reconcile.

## Duplicate dependencies

Reports crates that resolve to more than one version. Every extra version is
compiled and linked separately, so each one is avoidable binary size.

```
hashbrown
  0.14.5 — used by rowan v0.17.0
  0.17.1 — used by indexmap v2.14.0

1 duplicate dependency
```

Unlike `cargo tree --duplicates`, the graph is walked as the linker sees it:
dev-dependencies, build-dependencies, and anything reachable only through a
proc-macro are left out, because none of them reach the binary. In practice that
removes the `syn` duplicate almost every project would otherwise be told about.
Each version is listed with the crates that pull it in — the ones to bump or
unify.

# [Sponsored By](https://oxc.rs/sponsor)

<p align="center">
  <a href="https://oxc.rs/sponsor">
    <img src="https://raw.githubusercontent.com/oxc-project/sponsors/main/sponsors.svg" alt="Our sponsors" />
  </a>
</p>
