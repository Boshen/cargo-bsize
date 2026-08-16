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
      22.8 KiB   2.5%  zmij::STATIC_DATA
    ≤ 11.4 KiB   1.3%  core::unicode::unicode_data::conversions::LOWERCASE_LUT

largest types
  (in-memory layout size; a large type drives the moves, copies, and drop glue above)
       728 B  alloc::collections::btree::node::InternalNode<String, Value>
       112 B  serde_json::value::Value

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

Rolling those methods up one step further — by trait, not trait method — makes
that concrete and gathers what the per-method and per-crate views scatter. An
AST visitor is a single `Visit` impl spread over ~200 `visit_*` methods, each
attributed to the rule crate that wrote it, so no other view adds it into one
number:

```
by trait, every method of every impl combined
       1.5 MiB  11.8%  oxc_linter::rule::Rule (1094 methods)
     680.5 KiB   5.1%  serde_core::de::DeserializeSeed (368 methods)
     489.8 KiB   3.7%  tower_service::Service (267 methods)
     324.2 KiB   2.4%  oxc_ast_visit::generated::visit_js::VisitJs (635 methods)
     131.3 KiB   1.0%  oxc_ast_visit::generated::visit::Visit (203 methods)
```

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
each one absorbs the anonymous data that follows it. Sizes inferred this way are
marked `≤` and kept out of the rollups; without that, `httparse::TOKEN_MAP` reads
as 149 KiB when the declaration is `[bool; 256]`.

DWARF fixes most of that: it records the real `DW_AT_byte_size` of each static's
type, so a data symbol whose type it names gets an exact size and loses the `≤`.
Types are resolved by their global `.debug_info` offset, so a lookup table whose
element is a primitive shared across compilation units (a `DW_FORM_ref_addr`)
sizes correctly — `httparse::TOKEN_MAP` reads its true 256 bytes, not `≤149 KiB`.
The same walk ranks the **largest types** — the `-Zprint-type-sizes` insight
without a nightly compiler — because a large type is what drives the moves,
copies, and drop glue the code views measure. Both need debug info; a `≤` remains
only where DWARF is absent, which for a release binary is mostly `core`/`std`
statics, since std ships with little of it.

## Panic, format, and unwind overhead

The code views show where panic and format calls are; the read-only data those
calls lean on, and the unwind tables, are counted nowhere else. A panic site
loads a `core::panic::Location`; a `tracing` log site emits a static
`__CALLSITE` metadata record. Consolidating them names a lever a per-symbol view
cannot.

```
panic, format, and unwind overhead
  (infrastructure the code views only hint at; the levers below remove it)
      88.8 KiB   0.7%  unwind and exception tables
      15.8 KiB   0.1%  tracing metadata (143 symbols)

  levers: panic="abort" drops the unwind tables; -Zbuild-std with panic_immediate_abort strips the panic locations; disabling tracing removes the callsite metadata
```

The rollup is what matters — this data is many small records, and the
individual symbols are already in the data-symbol view. The exact sizes matter
too: read as `≤` upper bounds, oxlint's tracing metadata looked like a quarter
megabyte; sized exactly from DWARF it is 16 KiB. The classification is by name,
so it is a floor — anonymous panic-location constants have no symbol to match
and land in the "named by no symbol" bucket instead.

## Duplicate read-only data

The data analog of the assembly view's identical function bodies: two constants
with the same bytes under different names cost twice. Reading the actual bytes of
the constant sections — the one place the tool looks past sizes and names — and
hashing each named region finds them.

```
duplicate read-only data
  (byte-identical constants under different names; the linker's --icf or sharing a const collapses them)
       8.0 KiB   0.1%  recoverable from 3 groups (7 symbols)
       6.0 KiB   0.0%  hashbrown_0_14::…::TABLE ≡ hashbrown_0_15::…::TABLE ≡ 2 more (4 symbols, 2.0 KiB each)
```

The usual sources are a crate linked at several versions, each embedding the same
table, and a package's lib and bin crates shipping the same static twice. Exact
DWARF sizes are what make it work: without them two identical tables are compared
over different gap-inferred extents and never line up.

## Dynamic dispatch

A small proxy for what the binary spends on indirection: the named vtables
(`…::vtable`) and the function-pointer coercion and drop shims (`{shim:…}`). It
is a floor — the vtables themselves, the arrays of method pointers, are anonymous
in Rust and cannot be attributed by name. A lever in either direction: fewer
trait objects, or _more_ `dyn` to collapse a generic family the generic-families
view flags.

## By derive

Every impl of a derivable trait is generated, not written, so grouping them by
the derive shows what `#[derive(…)]` actually costs — a total the trait-method
view hints at but does not roll up by derive.

```
by derive, every impl combined
  (what #[derive(…)] costs across the binary)
      77.8 KiB   6.2%  Deserialize (83 impls)
      37.6 KiB   3.0%  Debug (106 impls)
       7.3 KiB   0.6%  Clone (12 impls)
```

Alongside it, the fraction of code the compiler split off as cold — the
`.text.unlikely` for panic and error paths. ELF only; Mach-O keeps cold code in
`__text`, so it reads zero there.

## Inlined code

Symbols only name functions that survived. Under `lto = "fat"` a function
inlined into all its callers has no symbol, and its bytes are counted against
whoever inlined it — **43.7% of cargo-bsize's own shipped binary**, across
91,137 instances. Debug info is the only place that code is named:

```
     454.4 KiB  43.7%  in 91137 inlined instances, charged to their callers

largest inlined functions
       8.7 KiB   0.8%  <u8 as core::slice::cmp::SlicePartialEq<u8>>::equal_same_length (374 sites)
       6.1 KiB   0.6%  core::ptr::copy_nonoverlapping::<u8> (733 sites)
       4.5 KiB   0.4%  <serde_json::read::SliceRead as serde_json::read::Read>::peek (153 sites)
```

Each record also names the source line the call was written on, which is ranked
separately:

```
source lines that pulled in the most inlined code
      21.8 KiB   2.1%  library/core/src/ptr/mod.rs:825 (9188 inlined)
      15.5 KiB   1.5%  library/std/src/alloc.rs:463 (1990 inlined)
       9.6 KiB   0.9%  serde_json-1.0.151/src/de.rs:1851 (8 inlined)
```

That list is topped by std and dependency lines you cannot edit, so a second one
keeps only the lines in this workspace — the code you can actually change. A path
is a workspace path when it resolves, against its compile unit's directory, to
somewhere under the workspace root; std reports `/rustc/<hash>`, a dependency its
registry checkout.

```
source lines in this workspace that pulled in the most inlined code
      58.8 KiB   0.4%  crates/oxc_allocator/src/boxed.rs:244 (14560 inlined)
      57.7 KiB   0.4%  crates/oxc_linter/src/fixer/mod.rs:262 (3566 inlined)
```

This attributes to the _immediate_ call site, so a deeply nested chain lands on
the library line that inlined it rather than on the code that started the chain.
Compile units spell the same file several ways — an absolute rustup path,
`/rustc/<hash>/library/…`, and a bare `library/…` — so paths are normalized, or
one source line splits across three rows.

Each `DW_TAG_inlined_subroutine` records the function inlined, the byte range it
occupies, and the call site it came from. Nested inlines share ranges —
`String::deref` inlining `as_str` inlining `Vec::as_slice` all cover the same
instructions — so a range is charged only to its innermost frame.

On macOS the debug info stays in the object files, so `dsymutil` gathers it
first; that is a link of existing debug info, not a recompile, and took 0.36s
here. Elsewhere it is already in the binary. The whole step is best-effort: if
debug info or `dsymutil` is missing, this section is omitted and the rest of the
report still runs.

`--limit` sets how many entries each list keeps (default 20).

## Assembly

The section and symbol views measure how big each function is. This reads the
assembly the compiler emitted, the way `cargo asm` does, to find _why_ — the
shapes a size cannot show. The binary is built with `cargo rustc -- --emit=asm`,
so under `lto = "fat"` the final crate's assembly is the whole program after
link-time optimization. Only functions that reached the linked binary are
counted, matched by mangled name, so the totals reconcile with the symbol view.

```
11.3 MiB  86.9%  code in 14665 functions with assembly, 2965551 instructions, 4.0 B each
```

Every figure below is `instructions × the binary's average bytes per
instruction`, marked `~`: the assembly names instructions, not the bytes each
became after the assembler.

**Identical bodies.** Functions whose instructions are the same, one for one,
once local labels are renamed and the constant pools and jump tables they load
are folded in. A linker's identical-code-folding keeps one of each group, and so
does not instantiating the others.

```
identical function bodies, by what folding each group would return
     148.4 KiB   1.1%  recoverable from 342 groups of identical functions (895 functions)
       6.6 KiB   0.0%  <alloc::raw_vec::RawVecInner>::finish_grow (23×, 308 B each)
       6.3 KiB   0.0%  <hashbrown::raw::RawTable<(…IdentifierId, (…Place, …))>>::reserve_rehash::<…>
```

**Panic call sites.** Every `[]`, `unwrap`, and allocation compiles to a
compare, a branch, and a cold block that loads a source location and calls the
panic machinery; the location is another 24 bytes of read-only data. The bytes
charged are the block that sets up the call, not the compare and branch that
skip it in the common case.

```
panic call sites
    ~333.5 KiB   2.5%  in the blocks of 21152 sites: 6088 bounds checks, 3699 unwraps, 7161 allocation failures, 4204 other
  (4096 distinct locations and messages loaded by those blocks)
```

**Formatting.** Calls into `core::fmt` and `alloc::fmt`, with the block before
each that builds the `Arguments`. A `Debug` derive that is only ever used in an
error path still emits all of this.

**Values copied through memory.** Runs of loads and stores back to back, and the
`memcpy` calls the compiler emits for anything too large to unroll — the cost of
moving a large value by value that boxing it or passing a reference removes.

```
values copied through memory
    ~498.6 KiB   3.7%  in 10650 runs, 127659 instructions, plus 6897 memcpy-family calls

functions copying the most
     ~20.7 KiB   0.2%  regex_automata::meta::strategy::new (5298 instructions in 108 runs, 14 calls)
      ~9.3 KiB   0.1%  oxlint::command::lint::lint_command (2382 instructions in 108 runs, 36 calls)
```

**Source lines.** Each instruction records the source line it was compiled from,
after inlining, and they are summed across every instantiation. The second list
keeps only lines in this workspace — the code you can actually edit.

```
source lines in this workspace compiled to the most instructions
     ~36.8 KiB   0.3%  crates/oxc_allocator/src/arena/alloc_impl.rs:33 (9433 instructions)
     ~34.5 KiB   0.3%  crates/oxc_allocator/src/vec2/raw_vec.rs:161 (8821 instructions)
     ~25.3 KiB   0.2%  crates/oxc_diagnostics/src/lib.rs:388 (6483 instructions)
```

rustc writes one assembly file for a single-codegen-unit crate and one per unit
otherwise. The whole step is best-effort: without assembly the section is
omitted and the rest of the report still runs. It is not cheap — for oxlint the
assembly is 1.4 GB and takes a few minutes to emit and read.

## LLVM IR by generic family

`--llvm-ir` is the [`cargo-llvm-lines`](https://github.com/dtolnay/cargo-llvm-lines)
technique: it emits the LLVM IR of every crate and rolls it up by generic family.
Every other view reads the linked binary, which shows only the code that survived
optimization; this reads the IR rustc handed to LLVM, before it inlined or deleted
anything, so it names the _source_ of monomorphization bloat.

```
cargo bsize --bin oxlint --llvm-ir

LLVM IR by generic family
  (pre-optimization IR lines, not binary bytes — where the code comes from, before the optimizer deletes it)
  87872 lines across 792 functions in 6 crates
    3250 lines  serde_json::value::ser::serialize (4×)
    2232 lines  core::ptr::drop_in_place (57×)
```

IR lines are not binary bytes — the optimizer removes much of this — so the view
shows line counts and instantiation counts, no size. It predicts where the code
comes from and what the compiler chewed through, and is read alongside the binary
views, not instead of them. It is **opt-in and slow**: the IR is asked of every
crate through `RUSTFLAGS`, so turning it on is a full rebuild and gigabytes of IR
(for oxlint, larger than the 1.4 GB of assembly).

## Baseline diff

`--baseline <binary>` compares this build against an earlier one and reports
what grew, by crate and by function — the "did my change bloat the binary, and
where" question a snapshot cannot answer.

```
cargo bsize --bin oxlint --baseline ./oxlint-before

vs baseline ./oxlint-before
      +18.3 KiB   0.1%  code, from 11.3 MiB to 11.3 MiB

by crate, largest change
      +12.1 KiB   0.1%  oxc_linter
       -4.2 KiB   0.0%  serde_json

by function, largest change
       +8.0 KiB   0.1%  <oxc_linter::rules::…::NewRule as Rule>::run (new)
```

Only code symbols are diffed, sized the same way (the gap inference) in both, so
the numbers are comparable — the read-only data, whose sizes are only bounds
without DWARF, is left out. The baseline should be built the same way this tool
builds (`debug = 2`, `strip = none`): pass a binary from an earlier
`target/bsize` build, not a stripped release one, or every symbol reads as
removed.

## What-if

`--what-if` measures, rather than guesses, what a build lever would save: it
rebuilds the binary with each profile setting toggled and reports the real change
in shipped size.

```
cargo bsize --what-if

what-if, measured by rebuilding
  (the change in shipped size under each lever)
     -48.0 KiB  13.6%  opt-level="z" (352.0 KiB → 304.0 KiB)
          +0 B   0.0%  panic="abort" (352.0 KiB → 352.0 KiB)
```

The `panic="abort"` row above is honest, not broken: without `-Zbuild-std` the
precompiled std still carries its unwind tables, so the flag alone saves little —
exactly the kind of thing a measured what-if shows and an estimate does not. Each
lever is a full build into its own target directory, so the primary cache is left
intact and turning this on is **opt-in and slow**. A lever some crate refuses to
build under (`panic="abort"` sometimes) is skipped.

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
