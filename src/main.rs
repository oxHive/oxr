mod cli;
mod config;
mod float;
mod git;
mod release;
mod replace;
mod template;
mod version;

use std::env;
use std::path::Path;

use anyhow::{bail, Result};
use clap::Parser;

use config::Config;
use release::{ForTarget, Level};
use version::Resolution;

fn main() -> Result<()> {
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
    Ok(())
}

fn run() -> Result<()> {
    let cli = cli::Cli::parse();
    let cwd = env::current_dir()?;
    let repo_root = git::repo_root(&cwd)?;

    if git::is_shallow(&repo_root)? {
        bail!(
            "'{}' is a shallow git checkout, so tag history is incomplete and version \
             resolution would silently be wrong. If running in GitHub Actions, set \
             `fetch-depth: 0` on the actions/checkout step (the default fetch-depth: 1 \
             does not fetch tags).",
            repo_root.display()
        );
    }

    let config = config::load(&repo_root)?;

    match cli.command {
        cli::Command::Current { json } => run_current(&repo_root, &config, json),
        cli::Command::Release {
            level,
            for_target,
            execute,
        } => run_release(&repo_root, &config, level, for_target, execute),
        cli::Command::Float { tag, execute } => run_float(&repo_root, &config, &tag, execute),
    }
}

fn resolve(repo_root: &Path, config: &Config) -> Result<Resolution> {
    let tags = git::list_tags(repo_root)?;
    version::resolve(&tags, &config.tag_pattern)
}

fn run_current(repo_root: &Path, config: &Config, json: bool) -> Result<()> {
    let resolution = resolve(repo_root, config)?;

    if json {
        let out = serde_json::json!({
            "latest_stable": resolution.latest_stable.as_ref().map(|v| v.to_string()),
            "active_train": resolution.active_train().map(|v| v.to_string()),
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        match &resolution.latest_stable {
            Some(v) => println!("latest_stable: {v}"),
            None => println!("latest_stable: none (no tags yet)"),
        }
        match resolution.active_train() {
            Some(v) => println!("active_train:  {v}"),
            None => println!("active_train:  none"),
        }
    }
    Ok(())
}

fn run_release(
    repo_root: &Path,
    config: &Config,
    level: Level,
    for_target: Option<ForTarget>,
    execute: bool,
) -> Result<()> {
    let resolution = resolve(repo_root, config)?;
    let next = release::next_version(&resolution, level, for_target)?;
    let tag_name = template::render(&config.tag_name, &next);

    if git::tag_exists(repo_root, &tag_name)? {
        bail!("tag '{tag_name}' already exists");
    }

    let from = resolution
        .latest_stable
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "none".to_string());
    println!("{from} -> {next}  (tag: {tag_name})");

    if !execute {
        println!("(dry run; pass --execute to apply)");
        for r in &config.pre_release_replacements {
            println!(
                "would update {} (expecting {} match(es) of the search pattern)",
                r.file, r.exactly
            );
        }
        if config.tag {
            println!("would create tag {tag_name}");
        }
        if config.push {
            println!("would push commit and tag to origin");
        }
        return Ok(());
    }

    let mut changed_paths = Vec::new();
    for r in &config.pre_release_replacements {
        replace::apply(repo_root, r, &next)?;
        changed_paths.push(r.file.clone());
    }

    let commit_message = template::render(&config.pre_release_commit_message, &next);

    if !changed_paths.is_empty() {
        git::stage_and_commit(
            repo_root,
            &changed_paths,
            &commit_message,
            config.sign_commit,
        )?;
    }

    if config.tag {
        git::create_tag(repo_root, &tag_name, &commit_message, config.sign_tag)?;
    }

    if config.push {
        if !changed_paths.is_empty() {
            git::push_current_branch(repo_root)?;
        }
        if config.tag {
            git::push_tag(repo_root, &tag_name, false)?;
        }
    }

    println!("released {tag_name}");
    Ok(())
}

fn run_float(repo_root: &Path, config: &Config, tag: &str, execute: bool) -> Result<()> {
    if !git::tag_exists(repo_root, tag)? {
        bail!("tag '{tag}' does not exist locally; fetch it first");
    }

    let plan = float::plan(tag, &config.float_tags)?;

    if plan.tags.is_empty() {
        println!(
            "no floating tags enabled in [float-tags] (major and minor both false); nothing to do"
        );
        return Ok(());
    }

    for floating in &plan.tags {
        println!("{floating} -> {tag} ({})", plan.version);
    }

    if !execute {
        println!("(dry run; pass --execute to apply)");
        return Ok(());
    }

    let target_sha = git::commit_of(repo_root, tag)?;
    let message = format!("float {tag}");
    for floating in &plan.tags {
        git::force_move_tag(repo_root, floating, &target_sha, &message, config.sign_tag)?;
        if config.push {
            git::push_tag(repo_root, floating, true)?;
        }
    }

    println!("floated: {}", plan.tags.join(", "));
    Ok(())
}
