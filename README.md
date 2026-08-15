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
       1.2 MiB  total
     912.0 KiB  shipped, excluding symbols and debug info

     745.9 KiB  81.8%  code
     319.8 KiB      -  symbols (not shipped)
      89.7 KiB   9.8%  read-only data
      41.2 KiB   4.5%  unwind
       3.4 KiB   0.4%  data
      31.7 KiB   3.5%  overhead (headers, padding, code signature)

     743.5 KiB  81.5%  __TEXT,__text
     319.8 KiB      -  __LINKEDIT
      54.5 KiB   6.0%  __TEXT,__const
      ...
```

Percentages are shares of `shipped`, not of the file on disk. The build forces
symbols and debug info in, so measuring against `total` would understate every
figure by whatever those weigh — here 26%. Rows that get stripped before release
show `-` rather than a share of a denominator they are not part of.

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
     752.8 KiB  82.5%  code in 1359 named symbols
      55.6 KiB   6.1%  read-only data in 34 named symbols
      29.9 KiB   3.3%  in those sections, named by no symbol

largest functions
      34.5 KiB   3.8%  <cargo_bsize::CargoBsize<std::io::stdio::Stdout>>::analyze
      17.9 KiB   2.0%  cargo_bsize::duplicates::find

largest data symbols
    ≤ 22.8 KiB   2.5%  zmij::STATIC_DATA
    ≤ 11.4 KiB   1.3%  core::unicode::unicode_data::conversions::LOWERCASE_LUT

by crate, where the code is defined
     169.5 KiB  18.6%  core (419 symbols)
      87.0 KiB   9.5%  cargo_bsize (14 symbols)

generic families
      30.9 KiB   3.4%  core::ptr::drop_glue (167×)
      27.2 KiB   3.0%  core::slice::sort::stable::quicksort::quicksort (14×)

by crate, which one caused the instantiation
  (generic code from the list above, re-attributed — not additional)
     178.2 KiB  19.5%  cargo_bsize (357 symbols)
      27.1 KiB   3.0%  cargo_metadata (62 symbols)
```

A name appearing more than once carries a `(2×)` suffix: the same item emitted
twice, its sizes summed. oxlint's package has both a lib and a bin crate, so it
ships two copies of `register_lsp_methods::<Backend>` at 28.3 KiB each.

### Why the grouped views matter more than the ranked one

Optimized Rust binaries are flat. In oxlint the largest single function is 0.5%
of the code and the twenty largest together are 4.9% — eliminating all of them
outright would barely move the number. The mass is in patterns repeated
thousands of times, so the useful question is not "which function is biggest"
but "which pattern costs the most across all its instances".

Grouping by trait method answers that. Every impl of one method sums into a
single row:

```
by trait method, every impl combined
     859.4 KiB   6.4%  <oxc_linter::rule::Rule>::run (596 impls)
     681.2 KiB   5.1%  <serde_core::de::DeserializeSeed>::deserialize (369 impls)
     489.7 KiB   3.7%  <tower_service::Service>::call (265 impls)
     439.8 KiB   3.3%  <oxc_linter::rule::Rule>::from_configuration (320 impls)
     243.2 KiB   1.8%  <oxc_linter::rule::Rule>::run_once (95 impls)
```

One trait accounts for 1.5 MiB there, and `from_configuration` costs ~1.4 KiB
per rule just to parse configuration — a fact no per-symbol ranking can show,
because no individual rule is large.

Some code belongs to no crate, module, or trait the way a named function does,
so it is matched on shape instead:

```
by pattern
  (a symbol can match several, so these do not sum to the total)
       1.8 MiB  13.9%  closures (2484 symbols)
       1.3 MiB   9.8%  serde (1544 symbols)
     356.6 KiB   2.7%  formatting (1431 symbols)
     284.7 KiB   2.1%  drop glue (1088 symbols)
```

Closures lead because a method generic over a closure type gets a fresh
instantiation per call site, which is the pattern
[Tighten Rust's Belt](https://dl.acm.org/doi/10.1145/3519941.3535075) singles
out. They are invisible to every other rollup.

Generic families carry both a unit cost and an estimate of what collapsing them
onto one dynamically-dispatched copy would return:

```
generic families
  (recoverable = the total less its largest instance)
     284.0 KiB   2.1%  core::ptr::drop_glue (1086×, 267 B each, ~280.8 KiB recoverable)
      63.2 KiB   0.5%  <oxc_linter::context::LintContext>::create_fix (125×, 517 B each, ~58.8 KiB recoverable)
```

`recoverable` is an upper bound: dynamic dispatch is not free and the surviving
copy may grow. The two orderings disagree usefully — ranking by total favours
whatever is instantiated most, ranking by unit cost favours whatever is
expensive each time.

That last block is the part `cargo bloat` cannot give you. v0 symbol mangling
records the instantiating crate whenever a generic is monomorphized outside the
crate that defined it, so `serde`'s code can be charged to whichever of your
crates caused it. It covers cross-crate instantiations only — when a crate
instantiates its own generic, the mangling omits the suffix.

**Why code and data are ranked separately.** Mach-O symbols carry no size, so
sizes come from the distance to the next symbol. That is sound in code, which is
densely named — oxlint names 14,668 symbols across 11.3 MiB of `__text` — but
not in the constant sections, where roughly a hundred names cover a megabyte and
each one absorbs the anonymous data that follows it. Unmeasured sizes are marked
`≤` and kept out of the rollups; without that, `httparse::TOKEN_MAP` reads as
149 KiB when the declaration is `[bool; 256]`.

One further limit: under `lto = "fat"` an inlined function has no symbol at all
and its bytes land on whatever inlined it, so this shows where code ended up
rather than where it was written.

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
