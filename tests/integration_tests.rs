//! Integration tests for embeddenator-workspace CLI

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn create_test_workspace() -> TempDir {
    let temp = TempDir::new().unwrap();

    // Create a small workspace with two packages
    let pkg1 = temp.path().join("pkg1");
    let pkg2 = temp.path().join("pkg2");

    fs::create_dir_all(&pkg1).unwrap();
    fs::create_dir_all(&pkg2).unwrap();

    fs::write(
        pkg1.join("Cargo.toml"),
        r#"[package]
name = "embeddenator-pkg1"
version = "0.20.0-alpha.1"
edition = "2021"

[dependencies]
embeddenator-pkg2 = "0.20.0-alpha.1"
"#,
    )
    .unwrap();

    fs::write(
        pkg2.join("Cargo.toml"),
        r#"[package]
name = "embeddenator-pkg2"
version = "0.20.0-alpha.1"
edition = "2021"

[dependencies]
"#,
    )
    .unwrap();

    temp
}

#[test]
fn test_check_versions_consistent() {
    let workspace = create_test_workspace();

    let output = Command::new(env!("CARGO_BIN_EXE_embeddenator-workspace"))
        .arg("check-versions")
        .current_dir(workspace.path())
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(0),
        "Expected success for consistent versions"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("All versions are consistent"));
}

#[test]
fn test_check_versions_inconsistent() {
    let workspace = create_test_workspace();

    // Modify pkg2 to have a different version
    fs::write(
        workspace.path().join("pkg2/Cargo.toml"),
        r#"[package]
name = "embeddenator-pkg2"
version = "0.21.0"
edition = "2021"

[dependencies]
"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_embeddenator-workspace"))
        .arg("check-versions")
        .current_dir(workspace.path())
        .output()
        .unwrap();

    assert_ne!(
        output.status.code(),
        Some(0),
        "Expected failure for inconsistent versions"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Dependency Inconsistencies"));
}

#[test]
fn test_bump_version_dry_run() {
    let workspace = create_test_workspace();

    let output = Command::new(env!("CARGO_BIN_EXE_embeddenator-workspace"))
        .args(["bump-version", "--prerelease", "--dry-run"])
        .current_dir(workspace.path())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Dry run mode"));
    assert!(stdout.contains("0.20.0-alpha.1 → 0.20.0-alpha.2"));

    // Verify files weren't actually changed
    let content = fs::read_to_string(workspace.path().join("pkg1/Cargo.toml")).unwrap();
    assert!(content.contains("version = \"0.20.0-alpha.1\""));
}

#[test]
fn test_bump_version_actual() {
    let workspace = create_test_workspace();

    let output = Command::new(env!("CARGO_BIN_EXE_embeddenator-workspace"))
        .args(["bump-version", "--prerelease"])
        .current_dir(workspace.path())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));

    // Verify files were changed
    let pkg1_content = fs::read_to_string(workspace.path().join("pkg1/Cargo.toml")).unwrap();
    assert!(pkg1_content.contains("version = \"0.20.0-alpha.2\""));
    assert!(pkg1_content.contains("embeddenator-pkg2 = \"0.20.0-alpha.2\""));

    let pkg2_content = fs::read_to_string(workspace.path().join("pkg2/Cargo.toml")).unwrap();
    assert!(pkg2_content.contains("version = \"0.20.0-alpha.2\""));
}
