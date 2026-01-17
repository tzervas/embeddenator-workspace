#[cfg(test)]
mod tests {
    use crate::cargo::{CargoManifest, DependencyType};
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_manifest(dir: &TempDir, name: &str, version: &str) -> PathBuf {
        let manifest_path = dir.path().join(name).join("Cargo.toml");
        fs::create_dir_all(manifest_path.parent().unwrap()).unwrap();

        let content = format!(
            r#"[package]
name = "{}"
version = "{}"
edition = "2021"

[dependencies]
"#,
            name, version
        );

        fs::write(&manifest_path, content).unwrap();
        manifest_path
    }

    fn create_test_manifest_with_deps(
        dir: &TempDir,
        name: &str,
        version: &str,
        deps: &[(&str, &str)],
    ) -> PathBuf {
        let manifest_path = dir.path().join(name).join("Cargo.toml");
        fs::create_dir_all(manifest_path.parent().unwrap()).unwrap();

        let mut content = format!(
            r#"[package]
name = "{}"
version = "{}"
edition = "2021"

[dependencies]
"#,
            name, version
        );

        for (dep_name, dep_version) in deps {
            content.push_str(&format!("{} = \"{}\"\n", dep_name, dep_version));
        }

        fs::write(&manifest_path, content).unwrap();
        manifest_path
    }

    #[test]
    fn test_load_manifest() {
        let temp_dir = TempDir::new().unwrap();
        let path = create_test_manifest(&temp_dir, "test-package", "0.20.0-alpha.1");

        let manifest = CargoManifest::load(&path).unwrap();

        assert_eq!(manifest.package_name, "test-package");
        assert_eq!(manifest.version.to_string(), "0.20.0-alpha.1");
    }

    #[test]
    fn test_set_version() {
        let temp_dir = TempDir::new().unwrap();
        let path = create_test_manifest(&temp_dir, "test-package", "0.20.0-alpha.1");

        let mut manifest = CargoManifest::load(&path).unwrap();
        let new_version = semver::Version::parse("0.21.0").unwrap();

        manifest.set_version(&new_version).unwrap();
        manifest.save().unwrap();

        // Reload and verify
        let reloaded = CargoManifest::load(&path).unwrap();
        assert_eq!(reloaded.version, new_version);
    }

    #[test]
    fn test_update_dependency() {
        let temp_dir = TempDir::new().unwrap();
        let path = create_test_manifest_with_deps(
            &temp_dir,
            "test-package",
            "0.20.0-alpha.1",
            &[("embeddenator-vsa", "0.20.0-alpha.1")],
        );

        let mut manifest = CargoManifest::load(&path).unwrap();
        let new_version = semver::Version::parse("0.21.0").unwrap();

        manifest
            .update_dependency("embeddenator-vsa", &new_version)
            .unwrap();
        manifest.save().unwrap();

        // Reload and verify
        let reloaded = CargoManifest::load(&path).unwrap();
        let vsa_dep = reloaded
            .dependencies
            .iter()
            .find(|d| d.name == "embeddenator-vsa")
            .unwrap();

        assert_eq!(vsa_dep.version.as_ref().unwrap(), &new_version);
    }

    #[test]
    fn test_embeddenator_dependencies() {
        let temp_dir = TempDir::new().unwrap();
        let path = create_test_manifest_with_deps(
            &temp_dir,
            "test-package",
            "0.20.0-alpha.1",
            &[
                ("embeddenator-vsa", "0.20.0-alpha.1"),
                ("serde", "1.0"),
                ("embeddenator-io", "0.20.0-alpha.1"),
            ],
        );

        let manifest = CargoManifest::load(&path).unwrap();
        let embeddenator_deps = manifest.embeddenator_dependencies();

        assert_eq!(embeddenator_deps.len(), 2);
        assert!(embeddenator_deps
            .iter()
            .any(|d| d.name == "embeddenator-vsa"));
        assert!(embeddenator_deps
            .iter()
            .any(|d| d.name == "embeddenator-io"));
    }
}
