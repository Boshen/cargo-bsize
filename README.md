# cargo-bsize

Analyze Rust binary size and propose size-reducing changes.

## Duplicate dependencies

```bash
cargo bsize
```

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

`--format=json` emits the same data for machine consumers.

# [Sponsored By](https://oxc.rs/sponsor)

<p align="center">
  <a href="https://oxc.rs/sponsor">
    <img src="https://raw.githubusercontent.com/oxc-project/sponsors/main/sponsors.svg" alt="Our sponsors" />
  </a>
</p>
