//! Tests for patch management functionality.

use crate::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use toml_edit::{DocumentMut, Item};

fn create_test_workspace() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path().to_path_buf();

    // Create mock repo directories
    let repos = vec![
        "embeddenator-vsa",
        "embeddenator-fs",
        "embeddenator-io",
        "embeddenator-retrieval",
    ];

    for repo in repos {
        let repo_path = root.join(repo);
        fs::create_dir_all(&repo_path).unwrap();

        // Create a simple Cargo.toml
        let manifest_content = format!(
            r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
            repo
        );
        fs::write(repo_path.join("Cargo.toml"), manifest_content).unwrap();
    }

    // Create a main package with git dependencies
    let main_path = root.join("embeddenator");
    fs::create_dir_all(&main_path).unwrap();

    let main_manifest = r#"[package]
name = "embeddenator"
version = "0.20.0"
edition = "2021"

[dependencies]
embeddenator-vsa = { git = "https://github.com/tzervas/embeddenator-vsa", tag = "v0.1.0" }
embeddenator-fs = { git = "https://github.com/tzervas/embeddenator-fs", branch = "main" }
embeddenator-io = { git = "https://github.com/tzervas/embeddenator-io", tag = "v0.1.1" }
serde = "1.0"

[dev-dependencies]
embeddenator-retrieval = { git = "https://github.com/tzervas/embeddenator-retrieval", tag = "v0.1.3" }
"#;
    fs::write(main_path.join("Cargo.toml"), main_manifest).unwrap();

    (temp_dir, root)
}

#[test]
fn test_discover_patchable_dependencies() {
    let (_temp, root) = create_test_workspace();
    let manager = PatchManager::new(&root);

    let deps = manager.discover_patchable_dependencies().unwrap();

    // Should find 4 dependencies (all embeddenator-* with git URLs)
    assert_eq!(deps.len(), 4);

    // Check names are sorted
    let names: Vec<&str> = deps.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "embeddenator-fs",
            "embeddenator-io",
            "embeddenator-retrieval",
            "embeddenator-vsa"
        ]
    );

    // Verify local paths are correct
    for dep in &deps {
        assert!(dep.local_path.exists());
        assert!(dep.local_path.join("Cargo.toml").exists());
    }

    // Check git URLs are extracted
    let vsa = deps.iter().find(|d| d.name == "embeddenator-vsa").unwrap();
    assert_eq!(vsa.git_url, "https://github.com/tzervas/embeddenator-vsa");
    assert_eq!(vsa.branch_or_tag, Some("v0.1.0".to_string()));
}

#[test]
fn test_apply_patches() {
    let (_temp, root) = create_test_workspace();
    let manager = PatchManager::new(&root);

    let deps = manager.discover_patchable_dependencies().unwrap();
    let report = manager.apply_patches(&deps, false).unwrap();

    assert_eq!(report.patched_count, 4);
    assert!(!report.verified); // verification skipped

    // Check that .cargo/config.toml was created
    let config_path = root.join(".cargo/config.toml");
    assert!(config_path.exists());

    // Parse the config file
    let content = fs::read_to_string(&config_path).unwrap();
    let doc: DocumentMut = content.parse().unwrap();

    // Verify patch sections exist
    let patch_key = "patch.\"https://github.com/tzervas/embeddenator-vsa\"";
    assert!(doc.get(patch_key).is_some());

    // Verify specific patch entry
    let vsa_path = doc
        .get(patch_key)
        .and_then(|p| p.get("embeddenator-vsa"))
        .and_then(|e| e.get("path"))
        .and_then(|p| p.as_str())
        .unwrap();

    assert!(vsa_path.contains("embeddenator-vsa"));
}

#[test]
fn test_remove_patches() {
    let (_temp, root) = create_test_workspace();
    let manager = PatchManager::new(&root);

    // First apply patches
    let deps = manager.discover_patchable_dependencies().unwrap();
    manager.apply_patches(&deps, false).unwrap();

    let config_path = root.join(".cargo/config.toml");
    assert!(config_path.exists());

    // Now remove patches
    let report = manager.remove_patches().unwrap();
    assert_eq!(report.removed_count, 4);
    assert!(report.config_deleted); // Should be deleted as it's empty

    // Config should be deleted
    assert!(!config_path.exists());
}

#[test]
fn test_remove_patches_preserves_other_config() {
    let (_temp, root) = create_test_workspace();
    let manager = PatchManager::new(&root);

    // Create .cargo directory and config with existing content
    let cargo_dir = root.join(".cargo");
    fs::create_dir_all(&cargo_dir).unwrap();

    let config_content = r#"[build]
target-dir = "custom-target"

[patch."https://github.com/tzervas/embeddenator-vsa"]
embeddenator-vsa = { path = "embeddenator-vsa" }
"#;
    let config_path = cargo_dir.join("config.toml");
    fs::write(&config_path, config_content).unwrap();

    // Remove patches
    let report = manager.remove_patches().unwrap();
    assert_eq!(report.removed_count, 1);
    assert!(!report.config_deleted); // Should be preserved

    // Verify the config still exists with other content
    let content = fs::read_to_string(&config_path).unwrap();
    let doc: DocumentMut = content.parse().unwrap();

    assert!(doc.get("build").is_some());
    assert!(doc
        .get("patch.\"https://github.com/tzervas/embeddenator-vsa\"")
        .is_none());
}

#[test]
fn test_remove_patches_when_none_exist() {
    let (_temp, root) = create_test_workspace();
    let manager = PatchManager::new(&root);

    // Remove patches without applying any
    let report = manager.remove_patches().unwrap();
    assert_eq!(report.removed_count, 0);
}

#[test]
fn test_multiple_repos_same_git_url() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create two repos
    for repo in &["embeddenator-vsa", "embeddenator-fs"] {
        let repo_path = root.join(repo);
        fs::create_dir_all(&repo_path).unwrap();
        let manifest = format!(
            r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"
"#,
            repo
        );
        fs::write(repo_path.join("Cargo.toml"), manifest).unwrap();
    }

    // Create main package that depends on both from same git URL
    let main_path = root.join("embeddenator");
    fs::create_dir_all(&main_path).unwrap();
    let manifest = r#"[package]
name = "embeddenator"
version = "0.1.0"
edition = "2021"

[dependencies]
embeddenator-vsa = { git = "https://github.com/tzervas/embeddenator", branch = "main" }
embeddenator-fs = { git = "https://github.com/tzervas/embeddenator", branch = "main" }
"#;
    fs::write(main_path.join("Cargo.toml"), manifest).unwrap();

    let manager = PatchManager::new(root);
    let deps = manager.discover_patchable_dependencies().unwrap();

    assert_eq!(deps.len(), 2);

    // Apply patches
    let report = manager.apply_patches(&deps, false).unwrap();
    assert_eq!(report.patched_count, 2);

    // Verify both patches are in the same patch section
    let config_path = root.join(".cargo/config.toml");
    let content = fs::read_to_string(&config_path).unwrap();
    let doc: DocumentMut = content.parse().unwrap();

    let patch_section = doc
        .get("patch.\"https://github.com/tzervas/embeddenator\"")
        .unwrap();
    assert!(patch_section.get("embeddenator-vsa").is_some());
    assert!(patch_section.get("embeddenator-fs").is_some());
}

#[test]
fn test_parse_git_dependency() {
    use toml_edit::value;

    // Test table format with tag
    let mut table = toml_edit::Table::new();
    table.insert("git", value("https://github.com/user/repo"));
    table.insert("tag", value("v1.0.0"));
    let item = Item::Table(table);

    let result = PatchManager::parse_git_dependency("test-crate", &item);
    assert!(result.is_some());
    let (url, tag) = result.unwrap();
    assert_eq!(url, "https://github.com/user/repo");
    assert_eq!(tag, Some("v1.0.0".to_string()));

    // Test table format with branch
    let mut table = toml_edit::Table::new();
    table.insert("git", value("https://github.com/user/repo"));
    table.insert("branch", value("main"));
    let item = Item::Table(table);

    let result = PatchManager::parse_git_dependency("test-crate", &item);
    assert!(result.is_some());
    let (url, branch) = result.unwrap();
    assert_eq!(url, "https://github.com/user/repo");
    assert_eq!(branch, Some("main".to_string()));

    // Test non-git dependency (version string)
    let item = value("1.0.0");
    let result = PatchManager::parse_git_dependency("test-crate", &item);
    assert!(result.is_none());
}
