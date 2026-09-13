use anyhow::{bail, Result};
use semver::Version;

use crate::config::FloatTags;
use crate::template;
use crate::version::extract_version;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FloatPlan {
    pub version: Version,
    /// Floating tags to force-move to the target, e.g. `[("v1", ...)]`.
    /// A major bump only ever adds a new entry here (e.g. `v2`); the
    /// previous major's floating tag is never included, never touched.
    pub tags: Vec<String>,
}

/// Computes which floating tags should move to `tag`, or hard-refuses if
/// `tag` carries a pre-release identifier. This safety check must live in
/// the binary itself, not just in CI trigger wiring — floating tags exist
/// so external consumers get trusted, stable code.
pub fn plan(tag: &str, float_tags: &FloatTags) -> Result<FloatPlan> {
    let version = extract_version(tag)
        .ok_or_else(|| anyhow::anyhow!("'{tag}' does not contain a parseable semver version"))?;

    if !version.pre.is_empty() {
        bail!(
            "refusing to float a pre-release tag ({version}): floating tags must only ever \
             point at stable releases"
        );
    }

    let mut tags = Vec::new();
    if float_tags.major {
        tags.push(template::render(&float_tags.major_tag_name, &version));
    }
    if float_tags.minor {
        tags.push(template::render(&float_tags.minor_tag_name, &version));
    }

    Ok(FloatPlan { version, tags })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn float_tags(major: bool, minor: bool) -> FloatTags {
        FloatTags {
            major,
            minor,
            major_tag_name: "v{{major}}".to_string(),
            minor_tag_name: "v{{major}}.{{minor}}".to_string(),
        }
    }

    #[test]
    fn refuses_prerelease_tag() {
        let err = plan("v1.5.0-rc.1", &float_tags(true, false)).unwrap_err();
        assert!(err
            .to_string()
            .contains("refusing to float a pre-release tag"));
    }

    #[test]
    fn major_only_produces_major_tag() {
        let p = plan("v1.4.3", &float_tags(true, false)).unwrap();
        assert_eq!(p.tags, vec!["v1".to_string()]);
    }

    #[test]
    fn major_and_minor_both_enabled() {
        let p = plan("v1.4.3", &float_tags(true, true)).unwrap();
        assert_eq!(p.tags, vec!["v1".to_string(), "v1.4".to_string()]);
    }

    #[test]
    fn major_bump_never_references_previous_major() {
        // v2.0.0 plan must only ever mention v2, never touch v1.
        let p = plan("v2.0.0", &float_tags(true, false)).unwrap();
        assert_eq!(p.tags, vec!["v2".to_string()]);
        assert!(!p.tags.iter().any(|t| t == "v1"));
    }
}
