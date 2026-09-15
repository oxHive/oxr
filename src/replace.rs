use std::path::Path;

use anyhow::{bail, Context, Result};
use regex::Regex;
use semver::Version;

use crate::config::Replacement;
use crate::template;

/// Reads `replacement.file` and errors unless `search` matches it exactly
/// `exactly` times. Performs no write, so it's safe to call for a dry-run
/// preview as well as before `apply`'s real write.
pub fn check(repo_root: &Path, replacement: &Replacement) -> Result<()> {
    let path = repo_root.join(&replacement.file);
    let contents =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;

    let re = Regex::new(&replacement.search)
        .with_context(|| format!("invalid search regex for {}", replacement.file))?;

    let match_count = re.find_iter(&contents).count();
    if match_count != replacement.exactly {
        bail!(
            "{}: expected search pattern to match exactly {} time(s), found {}",
            replacement.file,
            replacement.exactly,
            match_count
        );
    }
    Ok(())
}

/// Applies one `[[pre-release-replacements]]` entry to its file: the
/// `search` regex must match the file exactly `exactly` times (see
/// `check`), and `replace` is rendered for `{{version}}`/etc. before regex
/// backreference expansion (`$1`, `$2`, ...) is applied.
pub fn apply(repo_root: &Path, replacement: &Replacement, version: &Version) -> Result<()> {
    check(repo_root, replacement)?;

    let path = repo_root.join(&replacement.file);
    let contents =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let re = Regex::new(&replacement.search)
        .with_context(|| format!("invalid search regex for {}", replacement.file))?;

    let replace_template = template::render(&replacement.replace, version);
    let updated = re.replace_all(&contents, replace_template.as_str());

    std::fs::write(&path, updated.as_ref())
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn replacement(search: &str, replace: &str, exactly: usize) -> Replacement {
        Replacement {
            file: "plugin.json".to_string(),
            search: search.to_string(),
            replace: replace.to_string(),
            exactly,
        }
    }

    #[test]
    fn replaces_version_field() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("plugin.json"),
            r#"{"name": "foo", "version": "0.1.0"}"#,
        )
        .unwrap();
        let r = replacement(r#""version": "[^"]+""#, r#""version": "{{version}}""#, 1);
        let version = Version::parse("0.2.0").unwrap();
        apply(dir.path(), &r, &version).unwrap();
        let out = std::fs::read_to_string(dir.path().join("plugin.json")).unwrap();
        assert_eq!(out, r#"{"name": "foo", "version": "0.2.0"}"#);
    }

    #[test]
    fn check_passes_when_match_count_is_exact() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("plugin.json"),
            r#"{"name": "foo", "version": "0.1.0"}"#,
        )
        .unwrap();
        let r = replacement(r#""version": "[^"]+""#, r#""version": "{{version}}""#, 1);
        check(dir.path(), &r).unwrap();
    }

    #[test]
    fn check_reports_a_typo_in_search_without_writing_anything() {
        let dir = tempfile::tempdir().unwrap();
        let original = r#"{"name": "foo", "version": "0.1.0"}"#;
        std::fs::write(dir.path().join("plugin.json"), original).unwrap();
        // "varsion" instead of "version": matches nothing.
        let r = replacement(r#""varsion": "[^"]+""#, r#""varsion": "{{version}}""#, 1);

        let err = check(dir.path(), &r).unwrap_err();
        assert!(err.to_string().contains("found 0"), "{err}");

        let untouched = std::fs::read_to_string(dir.path().join("plugin.json")).unwrap();
        assert_eq!(untouched, original);
    }

    #[test]
    fn errors_on_match_count_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("plugin.json"), "no version field here").unwrap();
        let r = replacement(r#""version": "[^"]+""#, r#""version": "{{version}}""#, 1);
        let version = Version::parse("0.2.0").unwrap();
        let err = apply(dir.path(), &r, &version).unwrap_err();
        assert!(err.to_string().contains("expected search pattern"));
    }

    #[test]
    fn supports_regex_backreferences() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("plugin.json"), "prefix-1.0.0-suffix").unwrap();
        let r = replacement(
            r"prefix-([0-9.]+)-suffix",
            "prefix-{{version}}-suffix-$1",
            1,
        );
        let version = Version::parse("2.0.0").unwrap();
        apply(dir.path(), &r, &version).unwrap();
        let out = std::fs::read_to_string(dir.path().join("plugin.json")).unwrap();
        assert_eq!(out, "prefix-2.0.0-suffix-1.0.0");
    }
}
