use semver::Version;

/// Renders `{{version}}`, `{{major}}`, `{{minor}}`, `{{patch}}` placeholders
/// against a resolved version. `{{version}}` includes any pre-release suffix
/// (e.g. `1.5.0-rc.1`); the component placeholders are always the bare
/// numeric fields.
pub fn render(template: &str, version: &Version) -> String {
    template
        .replace("{{version}}", &version.to_string())
        .replace("{{major}}", &version.major.to_string())
        .replace("{{minor}}", &version.minor.to_string())
        .replace("{{patch}}", &version.patch.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_full_version() {
        let v = Version::parse("1.5.0-rc.1").unwrap();
        assert_eq!(render("v{{version}}", &v), "v1.5.0-rc.1");
    }

    #[test]
    fn renders_components_without_prerelease() {
        let v = Version::parse("2.3.4-beta.2").unwrap();
        assert_eq!(render("v{{major}}", &v), "v2");
        assert_eq!(render("v{{major}}.{{minor}}", &v), "v2.3");
    }
}
