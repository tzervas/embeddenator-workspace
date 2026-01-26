//! Workspace scanning and repository discovery.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::cargo::CargoManifest;

/// Scans the workspace for Cargo.toml files.
#[derive(Debug)]
pub struct WorkspaceScanner {
    root: PathBuf,
}

impl WorkspaceScanner {
    /// Create a new workspace scanner.
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    /// Find all Cargo.toml files in the workspace, excluding target/ and .git/ directories.
    pub fn find_manifests(&self) -> Result<Vec<CargoManifest>> {
        let mut manifests = Vec::new();

        for entry in WalkDir::new(&self.root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                // Skip target, .git, and other build directories
                !matches!(name.as_ref(), "target" | ".git" | "node_modules" | ".cargo")
            })
        {
            let entry = entry.context("Failed to read directory entry")?;

            if entry.file_type().is_file() && entry.file_name() == "Cargo.toml" {
                match CargoManifest::load(entry.path()) {
                    Ok(manifest) => manifests.push(manifest),
                    Err(e) => {
                        eprintln!("Warning: Failed to parse {}: {}", entry.path().display(), e);
                    }
                }
            }
        }

        Ok(manifests)
    }

    /// Find all embeddenator-* package manifests (excluding nested crates).
    pub fn find_embeddenator_packages(&self) -> Result<Vec<CargoManifest>> {
        let all_manifests = self.find_manifests()?;

        // Filter for top-level embeddenator packages
        // Exclude: embeddenator/crates/*, embeddenator/embeddenator-core/crates/*
        let mut packages: Vec<CargoManifest> = all_manifests
            .into_iter()
            .filter(|m| {
                let path_str = m.path.to_string_lossy();
                m.package_name.starts_with("embeddenator")
                    && !path_str.contains("/crates/")
                    && !path_str.contains("/target/")
            })
            .collect();

        packages.sort_by(|a, b| a.package_name.cmp(&b.package_name));
        Ok(packages)
    }
}
