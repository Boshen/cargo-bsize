# cargo-bsize

Analyze Rust binary size and propose size-reducing changes.

```bash
cargo bsize                # workspaces with one binary
cargo bsize --bin oxlint   # pick one when there are several
```

Builds one release binary and reports what is in it, then reports duplicate
dependencies. `--format=json` emits the same data for machine consumers.

Exactly one binary is analyzed. When a workspace has several, `--bin` is
required and the error lists the candidates; only that target is built. This is
not just faster — building the whole workspace at once lets cargo unify features
across members, which inflates the result. Building `oxlint` alone reports
16.9 MiB where building all of oxc's 14 binaries together reports 17.7 MiB for
the same target. A library-only workspace reports duplicates and skips this
section.

## Binary sections

```
target/bsize/release/cargo-bsize (macho)
       1.1 MiB  total
     832.0 KiB  shipped, excluding symbols and debug info

     663.6 KiB  58.9%  code
     295.3 KiB  26.2%  symbols
      85.6 KiB   7.6%  read-only data
      40.1 KiB   3.6%  unwind
       3.4 KiB   0.3%  data
      39.4 KiB   3.5%  overhead (headers, padding, code signature)

     661.2 KiB  58.7%  __TEXT,__text
     295.3 KiB  26.2%  __LINKEDIT
      53.0 KiB   4.7%  __TEXT,__const
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

## Symbols

The section report says the mass is code; this says which code. Symbols in the
code and read-only data sections are ranked individually, then rolled up by
crate, by generic family, and by the crate that caused each instantiation.

```
     808.7 KiB  65.7%  attributed to named symbols

largest symbols
      35.8 KiB   2.9%  cargo_bsize::main
      22.8 KiB   1.9%  zmij::STATIC_DATA

by crate
     193.3 KiB  15.7%  core (446 symbols)
      85.1 KiB   6.9%  cargo_bsize (13 symbols)

generic families
      30.8 KiB   2.5%  core::ptr::drop_glue (168×)
      27.2 KiB   2.2%  core::slice::sort::stable::quicksort::quicksort (14×)

generic code charged to the crate that instantiated it
     152.3 KiB  12.4%  cargo_bsize (358 symbols)
      27.1 KiB   2.2%  cargo_metadata (62 symbols)
```

That last block is the part `cargo bloat` cannot give you. v0 symbol mangling
records the instantiating crate whenever a generic is monomorphized outside the
crate that defined it, so `serde`'s code can be charged to whichever of your
crates caused it. It covers cross-crate instantiations only — when a crate
instantiates its own generic, the mangling omits the suffix.

Two limits worth knowing. Under `lto = "fat"` an inlined function has no symbol
at all and its bytes land on whatever inlined it, so this shows where code ended
up rather than where it was written. And since Mach-O symbols carry no size,
sizes come from address deltas, which charge inter-function padding to the
preceding symbol. The `attributed` line shows how much of the section the
symbols account for.

`--limit` sets how many entries each list keeps (default 20).

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
