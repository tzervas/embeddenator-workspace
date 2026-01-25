use crate::version::{BumpType, VersionManager};
use semver::Version;

#[test]
fn test_bump_major() {
    let manager = VersionManager::new(".");
    let current = Version::parse("0.20.0-alpha.1").unwrap();
    let new = manager
        .calculate_new_version(&current, BumpType::Major)
        .unwrap();
    assert_eq!(new.to_string(), "1.0.0");
}

#[test]
fn test_bump_minor() {
    let manager = VersionManager::new(".");
    let current = Version::parse("0.20.0-alpha.1").unwrap();
    let new = manager
        .calculate_new_version(&current, BumpType::Minor)
        .unwrap();
    assert_eq!(new.to_string(), "0.21.0");
}

#[test]
fn test_bump_patch() {
    let manager = VersionManager::new(".");
    let current = Version::parse("0.20.0-alpha.1").unwrap();
    let new = manager
        .calculate_new_version(&current, BumpType::Patch)
        .unwrap();
    assert_eq!(new.to_string(), "0.20.1");
}

#[test]
fn test_bump_prerelease_initial() {
    let manager = VersionManager::new(".");
    let current = Version::parse("0.20.0").unwrap();
    let new = manager
        .calculate_new_version(&current, BumpType::Prerelease)
        .unwrap();
    assert_eq!(new.to_string(), "0.20.0-alpha.1");
}

#[test]
fn test_bump_prerelease_increment() {
    let manager = VersionManager::new(".");
    let current = Version::parse("0.20.0-alpha.1").unwrap();
    let new = manager
        .calculate_new_version(&current, BumpType::Prerelease)
        .unwrap();
    assert_eq!(new.to_string(), "0.20.0-alpha.2");
}

#[test]
fn test_bump_prerelease_beta() {
    let manager = VersionManager::new(".");
    let current = Version::parse("0.20.0-beta.3").unwrap();
    let new = manager
        .calculate_new_version(&current, BumpType::Prerelease)
        .unwrap();
    assert_eq!(new.to_string(), "0.20.0-beta.4");
}
