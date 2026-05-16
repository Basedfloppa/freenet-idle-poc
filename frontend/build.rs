//! Build-time version stamping. Rewrites the patch component of
//! `CARGO_PKG_VERSION` to the current commit count so every push
//! advances the version even without a changelog edit, then exposes
//! the result as `BUILD_VERSION` for `env!()` to read at runtime.
//!
//! Fallback: when git is unavailable (sandboxed CI without `.git/`,
//! cargo publishing from a tarball, etc.) we leave the patch
//! untouched and just echo `CARGO_PKG_VERSION` — that keeps the
//! build deterministic without forcing every consumer onto git.

use std::process::Command;

fn main() {
    // Rebuild when HEAD moves or a new ref lands. The trailing
    // `index` line covers the staged-but-uncommitted edit case the
    // commit count won't notice but Cargo still needs to re-link.
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/refs");
    println!("cargo:rerun-if-changed=../.git/index");
    println!("cargo:rerun-if-changed=build.rs");

    let pkg_version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_string());
    let final_version = match commit_count() {
        Some(count) => version_with_patch(&pkg_version, count),
        None => pkg_version.clone(),
    };
    println!("cargo:rustc-env=BUILD_VERSION={final_version}");
}

fn commit_count() -> Option<u64> {
    let out = Command::new("git")
        .args(["rev-list", "--count", "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    s.trim().parse::<u64>().ok()
}

/// Replace the patch component of a `major.minor.patch[...]` string
/// with `patch`. If the input doesn't look like semver, fall back to
/// `major.minor.{patch}` synthesised from whatever leading numeric
/// chunks we can pull out — same shape the user asked for.
fn version_with_patch(pkg_version: &str, patch: u64) -> String {
    let parts: Vec<&str> = pkg_version.split('.').collect();
    let major = parts.first().copied().unwrap_or("0");
    let minor = parts.get(1).copied().unwrap_or("0");
    format!("{major}.{minor}.{patch}")
}
