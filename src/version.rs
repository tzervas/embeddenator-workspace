//! Version management and bumping utilities.

use anyhow::{Context, Result};
use semver::Version;
use std::collections::HashMap;
use std::path::Path;

use crate::cargo::CargoManifest;
use crate::workspace::WorkspaceScanner;

/// Type of version bump to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BumpType {
    Major,
    Minor,
    Patch,
    Prerelease,
}

/// Manages version updates across the workspace.
pub struct VersionManager {
    scanner: WorkspaceScanner,
}

impl VersionManager {
    /// Create a new version manager for the workspace.
    pub fn new(workspace_root: impl AsRef<Path>) -> Self {
        Self {
            scanner: WorkspaceScanner::new(workspace_root),
        }
    }

    /// Bump versions across all embeddenator packages.
    pub fn bump_versions(&self, bump_type: BumpType, dry_run: bool) -> Result<Vec<VersionChange>> {
        let mut manifests = self
            .scanner
            .find_embeddenator_packages()
            .context("Failed to find packages")?;

        if manifests.is_empty() {
            anyhow::bail!("No embeddenator packages found in workspace");
        }

        let mut changes = Vec::new();

        // Calculate new versions
        for manifest in &mut manifests {
            let old_version = manifest.version.clone();
            let new_version = self.calculate_new_version(&old_version, bump_type)?;

            changes.push(VersionChange {
                package: manifest.package_name.clone(),
                path: manifest.path.clone(),
                old_version: old_version.clone(),
                new_version: new_version.clone(),
            });

            if !dry_run {
                manifest.set_version(&new_version)?;
            }
        }

        // Update inter-dependencies
        if !dry_run {
            self.update_dependencies(&mut manifests, &changes)?;

            // Save all changes
            for manifest in manifests {
                manifest.save()?;
            }
        }

        Ok(changes)
    }

    fn calculate_new_version(&self, current: &Version, bump_type: BumpType) -> Result<Version> {
        let mut new_version = current.clone();

        match bump_type {
            BumpType::Major => {
                new_version.major += 1;
                new_version.minor = 0;
                new_version.patch = 0;
                new_version.pre = semver::Prerelease::EMPTY;
            }
            BumpType::Minor => {
                new_version.minor += 1;
                new_version.patch = 0;
                new_version.pre = semver::Prerelease::EMPTY;
            }
            BumpType::Patch => {
                new_version.patch += 1;
                new_version.pre = semver::Prerelease::EMPTY;
            }
            BumpType::Prerelease => {
                if new_version.pre.is_empty() {
                    // Start with alpha.1
                    new_version.pre = "alpha.1".parse()?;
                } else {
                    // Increment prerelease number
                    let pre_str = new_version.pre.as_str();

                    // Parse "alpha.1" -> increment to "alpha.2"
                    if let Some((prefix, num_str)) = pre_str.rsplit_once('.') {
                        if let Ok(num) = num_str.parse::<u64>() {
                            new_version.pre = format!("{}.{}", prefix, num + 1).parse()?;
                        } else {
                            // No number, add .1
                            new_version.pre = format!("{}.1", pre_str).parse()?;
                        }
                    } else {
                        // No dot, add .1
                        new_version.pre = format!("{}.1", pre_str).parse()?;
                    }
                }
            }
        }

        Ok(new_version)
    }

    fn update_dependencies(
        &self,
        manifests: &mut [CargoManifest],
        changes: &[VersionChange],
    ) -> Result<()> {
        let version_map: HashMap<String, Version> = changes
            .iter()
            .map(|c| (c.package.clone(), c.new_version.clone()))
            .collect();

        for manifest in manifests {
            // Collect dependency names that need updating
            let deps_to_update: Vec<(String, Version)> = manifest
                .embeddenator_dependencies()
                .iter()
                .filter_map(|dep| {
                    version_map
                        .get(&dep.name)
                        .map(|new_version| (dep.name.clone(), new_version.clone()))
                })
                .collect();

            // Now update them
            for (dep_name, new_version) in deps_to_update {
                manifest.update_dependency(&dep_name, &new_version)?;
            }
        }

        Ok(())
    }

    /// Check for version inconsistencies across the workspace.
    pub fn check_consistency(&self) -> Result<VersionReport> {
        let manifests = self
            .scanner
            .find_embeddenator_packages()
            .context("Failed to find packages")?;

        let mut report = VersionReport::default();

        // Track package versions
        let package_versions: HashMap<String, Version> = manifests
            .iter()
            .map(|m| (m.package_name.clone(), m.version.clone()))
            .collect();

        // Check for version drift
        let mut versions_by_major: HashMap<u64, Vec<&str>> = HashMap::new();
        for (name, version) in &package_versions {
            versions_by_major
                .entry(version.major)
                .or_default()
                .push(name.as_str());
        }

        if versions_by_major.len() > 1 {
            report.drift_detected = true;
            for (major, packages) in versions_by_major {
                report.issues.push(format!(
                    "Version drift: {} package(s) on major version {}: {}",
                    packages.len(),
                    major,
                    packages.join(", ")
                ));
            }
        }

        // Check dependency consistency
        for manifest in &manifests {
            for dep in manifest.embeddenator_dependencies() {
                if let Some(dep_version) = &dep.version {
                    if let Some(actual_version) = package_versions.get(&dep.name) {
                        if dep_version != actual_version {
                            report.inconsistencies.push(VersionInconsistency {
                                package: manifest.package_name.clone(),
                                dependency: dep.name.clone(),
                                expected: actual_version.clone(),
                                found: dep_version.clone(),
                            });
                        }
                    }
                }
            }
        }

        report.total_packages = manifests.len();
        Ok(report)
    }
}

/// Represents a version change for a package.
#[derive(Debug, Clone)]
pub struct VersionChange {
    pub package: String,
    pub path: std::path::PathBuf,
    pub old_version: Version,
    pub new_version: Version,
}

/// Report of version consistency check.
#[derive(Debug, Default)]
pub struct VersionReport {
    pub total_packages: usize,
    pub drift_detected: bool,
    pub issues: Vec<String>,
    pub inconsistencies: Vec<VersionInconsistency>,
}

#[derive(Debug, Clone)]
pub struct VersionInconsistency {
    pub package: String,
    pub dependency: String,
    pub expected: Version,
    pub found: Version,
}

impl VersionReport {
    pub fn has_issues(&self) -> bool {
        self.drift_detected || !self.inconsistencies.is_empty()
    }
}

#[cfg(test)]
#[path = "version_tests.rs"]
mod tests;
