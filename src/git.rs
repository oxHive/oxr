use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

/// Runs `git <args>` in the given working directory and returns trimmed
/// stdout, or an error carrying git's stderr on non-zero exit.
fn run(cwd: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .with_context(|| format!("failed to execute `git {}`", args.join(" ")))?;

    if !output.status.success() {
        bail!(
            "`git {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn repo_root(cwd: &Path) -> Result<PathBuf> {
    let out = run(cwd, &["rev-parse", "--show-toplevel"])?;
    Ok(PathBuf::from(out))
}

/// Detects a shallow checkout (e.g. `actions/checkout`'s default
/// `fetch-depth: 1`), which silently breaks tag-based version resolution.
/// oxr must fail loudly here rather than compute a wrong version.
pub fn is_shallow(repo_root: &Path) -> Result<bool> {
    let out = run(repo_root, &["rev-parse", "--is-shallow-repository"])?;
    Ok(out == "true")
}

pub fn list_tags(repo_root: &Path) -> Result<Vec<String>> {
    let out = run(repo_root, &["tag", "-l"])?;
    Ok(out
        .lines()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

pub fn tag_exists(repo_root: &Path, name: &str) -> Result<bool> {
    Ok(list_tags(repo_root)?.iter().any(|t| t == name))
}

/// Resolves a tag (or any committish) to the commit sha it points at.
pub fn commit_of(repo_root: &Path, committish: &str) -> Result<String> {
    run(repo_root, &["rev-list", "-n", "1", committish])
}

pub fn create_tag(repo_root: &Path, name: &str, message: &str, sign: bool) -> Result<()> {
    let mut args = vec!["tag"];
    args.push(if sign { "-s" } else { "-a" });
    args.push(name);
    args.push("-m");
    args.push(message);
    run(repo_root, &args)?;
    Ok(())
}

/// Force-moves a tag to `target`, creating it if absent. Used for floating
/// tags (`v1`, `v1.2`) which are, by design, mutable pointers.
pub fn force_move_tag(
    repo_root: &Path,
    name: &str,
    target: &str,
    message: &str,
    sign: bool,
) -> Result<()> {
    let mut args = vec!["tag", "-f"];
    args.push(if sign { "-s" } else { "-a" });
    args.push(name);
    args.push("-m");
    args.push(message);
    args.push(target);
    run(repo_root, &args)?;
    Ok(())
}

pub fn push_tag(repo_root: &Path, name: &str, force: bool) -> Result<()> {
    let mut args = vec!["push", "origin"];
    if force {
        args.push("--force");
    }
    args.push(name);
    run(repo_root, &args)?;
    Ok(())
}

pub fn push_current_branch(repo_root: &Path) -> Result<()> {
    run(repo_root, &["push"])?;
    Ok(())
}

pub fn stage_and_commit(
    repo_root: &Path,
    paths: &[String],
    message: &str,
    sign: bool,
) -> Result<()> {
    let mut add_args = vec!["add"];
    add_args.extend(paths.iter().map(|s| s.as_str()));
    run(repo_root, &add_args)?;

    let mut commit_args = vec!["commit"];
    if sign {
        commit_args.push("-S");
    }
    commit_args.push("-m");
    commit_args.push(message);
    run(repo_root, &commit_args)?;
    Ok(())
}
