//! Build the release binaries to analyze.
//!
//! Debug info is forced on and stripping off so the symbol table survives for
//! attribution. That changes the profile fingerprint, so the build goes to its
//! own target directory rather than invalidating the project's `target/release`.

use std::{
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{Context, Result, anyhow, bail};
use cargo_metadata::{Message, Metadata};

/// One bin target, named the way cargo needs to build just that one.
pub struct BinTarget {
    pub package: String,
    pub name: String,
}

/// Pick the bin target to analyze. `None` means the workspace has no binaries,
/// which is not an error — a library-only workspace still gets the rest of the
/// analysis.
///
/// # Errors
///
/// Errors when `requested` names no bin target, or when the workspace has
/// several and none was requested.
pub fn select_bin(metadata: &Metadata, requested: Option<&str>) -> Result<Option<BinTarget>> {
    let mut bins: Vec<BinTarget> = Vec::new();
    for package in metadata.workspace_packages() {
        for target in package.targets.iter().filter(|target| target.is_bin()) {
            bins.push(BinTarget {
                package: package.name.as_str().to_owned(),
                name: target.name.clone(),
            });
        }
    }
    bins.sort_by(|a, b| a.name.cmp(&b.name));

    let available = bins.iter().map(|bin| bin.name.as_str()).collect::<Vec<_>>().join(", ");

    if let Some(name) = requested {
        return bins
            .into_iter()
            .find(|bin| bin.name == name)
            .map(Some)
            .ok_or_else(|| anyhow!("no bin target named `{name}`; available: {available}"));
    }

    match bins.len() {
        0 => Ok(None),
        1 => Ok(bins.pop()),
        _ => bail!("workspace has several bin targets, pick one with --bin: {available}"),
    }
}

/// Build one bin target in release mode and return the executable it produced.
///
/// # Errors
///
/// Errors when cargo cannot be spawned or the build fails.
pub fn release_executable(
    path: &Path,
    target_dir: &Path,
    bin: &BinTarget,
    flags: &[&str],
) -> Result<Build> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut child = Command::new(cargo)
        .current_dir(path)
        .env("CARGO_TARGET_DIR", target_dir)
        .env("CARGO_PROFILE_RELEASE_DEBUG", "2")
        .env("CARGO_PROFILE_RELEASE_STRIP", "none")
        // `-Z` on a stable compiler. The alternative is requiring a nightly
        // toolchain the project may not have.
        .env("RUSTC_BOOTSTRAP", "1")
        .env("RUSTFLAGS", rustflags())
        .args(["build", "--release", "--message-format=json-render-diagnostics"])
        .args(["--package", &bin.package, "--bin", &bin.name])
        .args(flags)
        .stdout(Stdio::piped())
        .spawn()
        .context("failed to run `cargo build`")?;

    let stdout = child.stdout.take().ok_or_else(|| anyhow!("`cargo build` produced no stdout"))?;

    // rustc writes `MONO_ITEM` lines to the same stdout cargo puts its JSON on,
    // so one build yields both. Read every line: stopping early leaves cargo
    // writing into a closed pipe, which kills the build with a broken pipe.
    let mut executable = None;
    let mut mono_items = Vec::new();
    for line in BufReader::new(stdout).lines() {
        let line = line.context("failed to read `cargo build` output")?;

        if let Some(item) = line.strip_prefix("MONO_ITEM ") {
            mono_items.extend(mono_item(item));
            continue;
        }

        // Build scripts are also reported as artifacts with an executable, so
        // match the bin target by name rather than taking the first one seen.
        if let Ok(Message::CompilerArtifact(artifact)) = serde_json::from_str::<Message>(&line)
            && artifact.target.is_bin()
            && artifact.target.name == bin.name
            && let Some(path) = artifact.executable
        {
            executable = Some(path.into_std_path_buf());
        }
    }

    let status = child.wait().context("failed to wait for `cargo build`")?;
    if !status.success() {
        bail!("`cargo build --release` failed with {status}");
    }

    let executable = executable
        .ok_or_else(|| anyhow!("`cargo build` produced no executable for `{}`", bin.name))?;

    // rustc only prints `MONO_ITEM` lines when it actually runs, so a cached
    // build yields none and the monomorphization view would silently empty on
    // every repeat run. Keep them beside the binary they describe: the same
    // fingerprint that let cargo skip the build keeps them accurate.
    let cached = target_dir.join("mono-items");
    if mono_items.is_empty() {
        mono_items = read_mono_items(&cached);
    } else {
        write_mono_items(&cached, &mono_items);
    }

    Ok(Build { executable, mono_items })
}

fn read_mono_items(path: &Path) -> Vec<MonoItem> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| mono_item(line.strip_prefix("MONO_ITEM ")?))
        .collect()
}

/// Best-effort: losing the cache costs a view on the next run, not this one.
fn write_mono_items(path: &Path, items: &[MonoItem]) {
    let kind = |item: &MonoItem| if item.shim { " - shim" } else { "" };
    let lines: Vec<String> = items
        .iter()
        .map(|item| {
            format!("MONO_ITEM fn {}{} @@ {}.x-cgu.0[Internal]", item.name, kind(item), item.krate)
        })
        .collect();

    let _ = std::fs::write(path, lines.join("\n"));
}

/// What one build produced.
pub struct Build {
    pub executable: PathBuf,

    /// Every item the compiler monomorphized, whether or not it survived to the
    /// linked binary. Most do not.
    pub mono_items: Vec<MonoItem>,
}

/// One instantiation the compiler generated.
pub struct MonoItem {
    pub name: String,

    /// The crate it was generated in, read from its codegen unit. Needed to
    /// drop proc-macro crates, which are monomorphized like any other but run
    /// inside the compiler and reach no binary.
    pub krate: String,

    /// Compiler-generated glue — function-pointer coercions, vtable and drop
    /// shims. rustc marks these, and they correspond to nothing in the source.
    pub shim: bool,
}

/// Setting `RUSTFLAGS` overrides any `rustflags` the project configured, so
/// keep whatever the environment already asked for and append to it.
fn rustflags() -> String {
    let mut flags = std::env::var("RUSTFLAGS").unwrap_or_default();
    if !flags.is_empty() {
        flags.push(' ');
    }
    flags.push_str("-Zprint-mono-items=yes");
    flags
}

/// Parse `fn some::path @@ krate.hash-cgu.0[Internal]` into the path, which is
/// shaped like a demangled symbol, and the crate that generated it.
fn mono_item(item: &str) -> Option<MonoItem> {
    let item = item.strip_prefix("fn ").or_else(|| item.strip_prefix("static ")).unwrap_or(item);
    let (name, cgu) = item.rsplit_once(" @@ ")?;
    let krate = cgu.split('.').next()?;

    Some(MonoItem {
        shim: name.contains(" - shim"),
        name: name.to_owned(),
        krate: krate.to_owned(),
    })
}
