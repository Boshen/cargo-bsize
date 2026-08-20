//! Measure, not guess, what a build lever would save.
//!
//! The other views point at levers — `panic="abort"` drops the unwind tables,
//! `opt-level="z"` trades speed for size, `-Zfmt-debug=none` deletes every
//! `Debug` impl. This one rebuilds the binary with each lever set and reports
//! the real change in shipped size, so the number is measured rather than
//! estimated; and, function by function, what moved — the functions that shrink
//! most under a lever are where that cost sits, which is what turns a
//! configuration measurement into a source-level target (the `Debug` impls
//! worth dropping, the functions -O3 expanded, the landing pads). Each lever is
//! a full build into its own target directory (so the primary's cache is left
//! intact), which is why it is opt-in and selectable.
//!
//! Nightly-only flags are reached through `RUSTC_BOOTSTRAP=1` on whatever
//! toolchain is pinned; the levers that need it say so, and any lever that
//! fails to build is skipped rather than sinking the report.

use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{Context, Result, anyhow, bail};

use crate::{
    build::{self, BuildTarget},
    diff::{self, DiffReport},
    sections,
};

#[derive(Debug)]
pub struct WhatIfReport {
    pub levers: Vec<LeverResult>,

    /// Levers asked for whose build failed (a crate rejected the setting, or
    /// the toolchain lacks what it needs).
    pub skipped: Vec<String>,
}

#[derive(Debug)]
pub struct LeverResult {
    pub name: String,

    /// Shipped size now, and under the lever.
    pub before: u64,
    pub after: u64,

    /// What moved, function by function and crate by crate: `before` is the
    /// primary build, `after` the lever's.
    pub diff: DiffReport,
}

/// One lever: a name to ask for it by, and the settings that apply it.
#[derive(Debug)]
pub struct Lever {
    pub name: &'static str,

    /// Cargo profile overrides, as `CARGO_PROFILE_RELEASE_*` variables.
    env: &'static [(&'static str, &'static str)],

    /// Flags appended to `RUSTFLAGS`, so they reach every crate.
    rustflags: &'static [&'static str],

    /// Extra cargo arguments (`-Zbuild-std…`).
    cargo: &'static [&'static str],

    /// Needs `RUSTC_BOOTSTRAP=1` for a `-Z` flag, and an explicit `--target`
    /// (which `-Zbuild-std` requires, and which keeps `RUSTFLAGS` off build
    /// scripts and proc macros).
    nightly: bool,

    /// What the saving means for source-level work.
    pub reading: &'static str,
}

/// The two levers `--what-if` measures by default.
pub const DEFAULT_LEVERS: [&str; 2] = ["opt-level=\"z\"", "panic=\"abort\""];

/// Every lever, in the order they are reported.
pub const LEVERS: &[Lever] = &[
    Lever {
        name: "opt-level=\"z\"",
        env: &[("CARGO_PROFILE_RELEASE_OPT_LEVEL", "z")],
        rustflags: &[],
        cargo: &[],
        nightly: false,
        reading: "the functions that shrink most are where -O3 unrolled, vectorized, and inlined most: #[cold] and #[inline(never)] on their cold paths, or a simpler loop, gets part of it back at -O3",
    },
    Lever {
        name: "panic=\"abort\"",
        env: &[("CARGO_PROFILE_RELEASE_PANIC", "abort")],
        rustflags: &[],
        cargo: &[],
        nightly: false,
        reading: "the functions that shrink most carry the most landing pads: values with destructors held across calls",
    },
    Lever {
        name: "lto=\"fat\"",
        env: &[("CARGO_PROFILE_RELEASE_LTO", "fat")],
        rustflags: &[],
        cargo: &[],
        nightly: false,
        reading: "cross-crate inlining and dead-code removal",
    },
    Lever {
        name: "codegen-units=1",
        env: &[("CARGO_PROFILE_RELEASE_CODEGEN_UNITS", "1")],
        rustflags: &[],
        cargo: &[],
        nightly: false,
        reading: "one unit per crate: no duplicated generics across units",
    },
    Lever {
        name: "fmt-debug=none",
        env: &[],
        rustflags: &["-Zfmt-debug=none"],
        cargo: &[],
        nightly: true,
        reading: "the whole cost of Debug formatting outside std; the functions that shrink most are the derives and {:?} sites worth removing",
    },
    Lever {
        name: "location-detail=none",
        env: &[],
        rustflags: &["-Zlocation-detail=none"],
        cargo: &[],
        nightly: true,
        reading: "the panic locations outside std: file paths, line and column records, and the branches they keep apart",
    },
    Lever {
        name: "share-generics=yes",
        env: &[],
        rustflags: &["-Zshare-generics=yes"],
        cargo: &[],
        nightly: true,
        reading: "generic instantiations reused across crates instead of duplicated",
    },
    Lever {
        name: "virtual-function-elimination",
        env: &[],
        rustflags: &["-Zvirtual-function-elimination"],
        cargo: &[],
        nightly: true,
        reading: "vtable entries nothing calls (needs lto=\"fat\")",
    },
    Lever {
        name: "force-unwind-tables=no",
        env: &[("CARGO_PROFILE_RELEASE_PANIC", "abort")],
        rustflags: &["-Cforce-unwind-tables=no"],
        cargo: &[],
        nightly: false,
        reading: "the unwind tables, with panic=\"abort\"",
    },
    Lever {
        name: "no-vectorize",
        env: &[],
        rustflags: &["-Cno-vectorize-loops", "-Cno-vectorize-slp"],
        cargo: &[],
        nightly: false,
        reading: "the code the loop and SLP vectorizers added; the functions that shrink most hold the loops they widened",
    },
    Lever {
        name: "build-std",
        env: &[],
        rustflags: &[],
        cargo: &["-Zbuild-std=std,panic_abort"],
        nightly: true,
        reading: "std compiled with this profile instead of the prebuilt one",
    },
    Lever {
        name: "optimize-for-size",
        env: &[],
        rustflags: &[],
        cargo: &["-Zbuild-std=std,panic_abort", "-Zbuild-std-features=optimize_for_size"],
        nightly: true,
        reading: "std compiled with its optimize_for_size feature",
    },
    Lever {
        name: "panic=\"immediate-abort\"",
        env: &[("CARGO_PROFILE_RELEASE_PANIC", "abort")],
        rustflags: &["-Zunstable-options", "-Cpanic=immediate-abort"],
        cargo: &["-Zbuild-std=std,panic_abort"],
        nightly: true,
        reading: "the whole panic machinery, std included: messages, locations, formatting, and the unwinder",
    },
    Lever {
        name: "min-size",
        env: &[
            ("CARGO_PROFILE_RELEASE_OPT_LEVEL", "z"),
            ("CARGO_PROFILE_RELEASE_LTO", "fat"),
            ("CARGO_PROFILE_RELEASE_CODEGEN_UNITS", "1"),
            ("CARGO_PROFILE_RELEASE_PANIC", "abort"),
        ],
        rustflags: &[
            "-Zunstable-options",
            "-Cpanic=immediate-abort",
            "-Zfmt-debug=none",
            "-Zlocation-detail=none",
        ],
        cargo: &["-Zbuild-std=std,panic_abort", "-Zbuild-std-features=optimize_for_size"],
        nightly: true,
        reading: "every lever at once: the floor",
    },
];

/// What a lever's saving means for source-level work, by lever name.
#[must_use]
pub fn reading(name: &str) -> Option<&'static str> {
    LEVERS.iter().find(|lever| lever.name == name).map(|lever| lever.reading)
}

/// The levers named in a `--levers` list — `all`, or comma-separated names —
/// or the default two.
///
/// # Errors
///
/// Errors when a name is not a lever.
pub fn select(levers: Option<&str>) -> Result<Vec<&'static Lever>> {
    let Some(list) = levers else {
        return Ok(LEVERS.iter().filter(|lever| DEFAULT_LEVERS.contains(&lever.name)).collect());
    };
    if list.trim() == "all" {
        return Ok(LEVERS.iter().collect());
    }

    let mut selected = Vec::new();
    for name in list.split(',').map(str::trim).filter(|name| !name.is_empty()) {
        let lever = LEVERS
            .iter()
            .find(|lever| lever.name == name || lever.name.replace('"', "") == name)
            .ok_or_else(|| {
                anyhow!(
                    "unknown lever `{name}`; the levers are: all, {}",
                    LEVERS.iter().map(|lever| lever.name).collect::<Vec<_>>().join(", ")
                )
            })?;
        if !selected.iter().any(|known: &&Lever| known.name == lever.name) {
            selected.push(lever);
        }
    }
    Ok(selected)
}

/// The primary build the levers are measured against.
pub struct Primary<'a> {
    pub file: &'a object::File<'a>,
    pub shipped: u64,
}

/// How the lever builds are run: the project, where they build, and what.
pub struct Job<'a> {
    pub path: &'a Path,
    pub target_dir: &'a Path,
    pub target: &'a BuildTarget,

    /// The resolution flags (`--locked`, …) forwarded to cargo.
    pub flags: &'a [&'a str],

    /// The host triple, for the builds that need an explicit `--target`.
    pub host: &'a str,
}

/// Rebuild the binary under each of `levers` and report the change in shipped
/// size against `primary`, and what moved.
pub fn analyze(
    job: &Job<'_>,
    levers: &[&Lever],
    primary: &Primary<'_>,
    limit: usize,
) -> WhatIfReport {
    let mut results = Vec::new();
    let mut skipped = Vec::new();
    for lever in levers {
        let target = job.target_dir.join(format!("whatif-{}", slug(lever.name)));
        match build(job, &target, lever) {
            Ok(binary) => match measure(&binary, primary, lever.name, limit) {
                Ok((after, diff)) => {
                    results.push(LeverResult {
                        name: lever.name.to_owned(),
                        before: primary.shipped,
                        after,
                        diff,
                    });
                }
                Err(_) => skipped.push(lever.name.to_owned()),
            },
            Err(_) => skipped.push(lever.name.to_owned()),
        }
    }

    WhatIfReport { levers: results, skipped }
}

/// A lever's name as a directory name.
fn slug(name: &str) -> String {
    name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect()
}

/// Build the linked artifact into `target_dir` with `lever` applied and return
/// its path.
fn build(job: &Job<'_>, target_dir: &Path, lever: &Lever) -> Result<PathBuf> {
    let target = job.target;
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(cargo);
    command
        .current_dir(job.path)
        .env("CARGO_TARGET_DIR", target_dir)
        .env("CARGO_PROFILE_RELEASE_DEBUG", "2")
        .env("CARGO_PROFILE_RELEASE_STRIP", "none")
        .envs(lever.env.iter().copied())
        .args(["build", "--release", "--message-format=json-render-diagnostics"])
        .args(target.cargo_args())
        .args(job.flags)
        .args(lever.cargo)
        .stdout(Stdio::piped());
    if !lever.rustflags.is_empty() {
        command.env("RUSTFLAGS", rustflags_with(lever.rustflags));
    }
    if lever.nightly {
        command.env("RUSTC_BOOTSTRAP", "1").args(["--target", job.host]);
    }

    let mut child = command.spawn().context("failed to run `cargo build`")?;
    let stdout = child.stdout.take().ok_or_else(|| anyhow!("`cargo build` produced no stdout"))?;
    let binary = build::find_artifact(stdout, target)?;

    let status = child.wait().context("failed to wait for `cargo build`")?;
    if !status.success() {
        bail!("`cargo build` failed with {status}");
    }

    binary.ok_or_else(|| anyhow!("no linked artifact for `{}`", target.name))
}

/// `RUSTFLAGS` with `flags` appended to whatever the environment set, so the
/// lever reaches every crate without dropping the caller's flags.
fn rustflags_with(flags: &[&str]) -> String {
    let mut all = env::var("RUSTFLAGS").unwrap_or_default();
    for flag in flags {
        if !all.is_empty() {
            all.push(' ');
        }
        all.push_str(flag);
    }
    all
}

/// The lever binary's shipped size, and its code diffed against the primary.
fn measure(
    binary: &Path,
    primary: &Primary<'_>,
    name: &str,
    limit: usize,
) -> Result<(u64, DiffReport)> {
    let data =
        std::fs::read(binary).with_context(|| format!("failed to read {}", binary.display()))?;
    let file = object::File::parse(&*data)
        .with_context(|| format!("failed to parse {}", binary.display()))?;

    let shipped = sections::analyze(&file, binary, data.len() as u64).shipped;
    let diff = diff::between(primary.file, &file, name, limit);
    Ok((shipped, diff))
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_LEVERS, LEVERS, select, slug};

    #[test]
    fn selects_levers_by_name_with_the_default_two_when_unnamed() {
        let names = |levers: Vec<&super::Lever>| {
            levers.into_iter().map(|lever| lever.name).collect::<Vec<_>>()
        };

        assert_eq!(names(select(None).unwrap()), DEFAULT_LEVERS);
        assert_eq!(names(select(Some("all")).unwrap()).len(), LEVERS.len());
        // Quotes are optional on the command line, and repeats collapse.
        assert_eq!(
            names(select(Some("panic=abort, fmt-debug=none,panic=\"abort\"")).unwrap()),
            ["panic=\"abort\"", "fmt-debug=none"]
        );
        assert!(select(Some("no-such-lever")).unwrap_err().to_string().contains("unknown lever"));

        assert_eq!(slug("panic=\"immediate-abort\""), "panic--immediate-abort-");
    }
}
