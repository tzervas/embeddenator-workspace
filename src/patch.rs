//! Cargo patch management for local development.
//!
//! This module provides functionality to patch git dependencies to use local
//! paths during development, and restore them when done.

use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use toml_edit::{value, DocumentMut, Item, Table};

use crate::workspace::WorkspaceScanner;

/// Information about a git dependency that can be patched.
#[derive(Debug, Clone)]
pub struct GitDependency {
    pub name: String,
    pub git_url: String,
    pub branch_or_tag: Option<String>,
    pub local_path: PathBuf,
}

/// Manager for Cargo patch operations.
pub struct PatchManager {
    workspace_root: PathBuf,
}

impl PatchManager {
    /// Create a new patch manager.
    pub fn new(workspace_root: impl AsRef<Path>) -> Self {
        Self {
            workspace_root: workspace_root.as_ref().to_path_buf(),
        }
    }

    /// Discover all embeddenator repos and their git dependencies.
    pub fn discover_patchable_dependencies(&self) -> Result<Vec<GitDependency>> {
        let scanner = WorkspaceScanner::new(&self.workspace_root);
        let manifests = scanner.find_manifests()?;

        let mut git_deps: HashMap<String, GitDependency> = HashMap::new();
        let mut available_repos: HashSet<String> = HashSet::new();

        // First pass: identify all available local repos
        for manifest in &manifests {
            if manifest.package_name.starts_with("embeddenator") {
                available_repos.insert(manifest.package_name.clone());
            }
        }

        // Second pass: find git dependencies that have local equivalents
        for manifest in &manifests {
            let content = std::fs::read_to_string(&manifest.path)?;
            let doc: DocumentMut = content.parse()?;

            // Check dependencies, dev-dependencies, build-dependencies
            for section in &["dependencies", "dev-dependencies", "build-dependencies"] {
                if let Some(Item::Table(deps_table)) = doc.get(section) {
                    for (name, dep_item) in deps_table.iter() {
                        if let Some(git_dep) = Self::parse_git_dependency(name, dep_item) {
                            // Check if we have this repo locally
                            if available_repos.contains(name) {
                                // Find the local path
                                if let Some(local_path) = self.find_local_repo_path(name) {
                                    git_deps.insert(
                                        name.to_string(),
                                        GitDependency {
                                            name: name.to_string(),
                                            git_url: git_dep.0,
                                            branch_or_tag: git_dep.1,
                                            local_path,
                                        },
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut deps: Vec<GitDependency> = git_deps.into_values().collect();
        deps.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(deps)
    }

    /// Parse git dependency from TOML item.
    fn parse_git_dependency(_name: &str, item: &Item) -> Option<(String, Option<String>)> {
        // Handle both inline tables and regular tables
        let git_url = item.get("git")?.as_str()?.to_string();
        let branch_or_tag = item
            .get("branch")
            .or_else(|| item.get("tag"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        Some((git_url, branch_or_tag))
    }

    /// Find the local path for a repository.
    fn find_local_repo_path(&self, repo_name: &str) -> Option<PathBuf> {
        let expected_path = self.workspace_root.join(repo_name);
        if expected_path.join("Cargo.toml").exists() {
            Some(expected_path)
        } else {
            None
        }
    }

    /// Apply local patches to .cargo/config.toml
    pub fn apply_patches(&self, deps: &[GitDependency], verify: bool) -> Result<PatchReport> {
        let cargo_dir = self.workspace_root.join(".cargo");
        let config_path = cargo_dir.join("config.toml");

        // Create .cargo directory if it doesn't exist
        if !cargo_dir.exists() {
            std::fs::create_dir(&cargo_dir).context("Failed to create .cargo directory")?;
        }

        // Load or create config.toml
        let mut doc: DocumentMut = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            content.parse()?
        } else {
            DocumentMut::new()
        };

        let mut patched_count = 0;

        // Group dependencies by git URL
        let mut patches_by_url: HashMap<String, Vec<&GitDependency>> = HashMap::new();
        for dep in deps {
            patches_by_url
                .entry(dep.git_url.clone())
                .or_default()
                .push(dep);
        }

        // Apply patches for each git URL
        for (git_url, deps_for_url) in patches_by_url {
            let patch_key = format!("patch.\"{}\"", git_url);

            // Create patch section if it doesn't exist
            if doc.get(&patch_key).is_none() {
                doc[&patch_key] = Item::Table(Table::new());
            }

            if let Some(Item::Table(patch_table)) = doc.get_mut(&patch_key) {
                for dep in deps_for_url {
                    // Create patch entry
                    let mut dep_table = Table::new();
                    dep_table.insert("path", value(dep.local_path.to_string_lossy().to_string()));

                    patch_table.insert(&dep.name, Item::Table(dep_table));
                    patched_count += 1;
                }
            }
        }

        // Save the config file
        std::fs::write(&config_path, doc.to_string())
            .context("Failed to write .cargo/config.toml")?;

        let mut report = PatchReport {
            patched_count,
            config_path: config_path.clone(),
            verified: false,
            verification_error: None,
        };

        // Verify patches if requested
        if verify {
            match self.verify_patches() {
                Ok(_) => report.verified = true,
                Err(e) => report.verification_error = Some(e.to_string()),
            }
        }

        Ok(report)
    }

    /// Remove all patches from .cargo/config.toml
    pub fn remove_patches(&self) -> Result<ResetReport> {
        let cargo_dir = self.workspace_root.join(".cargo");
        let config_path = cargo_dir.join("config.toml");

        if !config_path.exists() {
            return Ok(ResetReport {
                removed_count: 0,
                config_path,
                config_deleted: false,
            });
        }

        let content = std::fs::read_to_string(&config_path)?;
        let mut doc: DocumentMut = content.parse()?;

        let mut removed_count = 0;

        // Find all patch.* sections (both dotted keys like patch."url" and nested [patch] table)
        let mut keys_to_remove = Vec::new();

        for (key, _) in doc.as_table().iter() {
            if key == "patch" {
                // Handle [patch] table with nested sources
                if let Some(Item::Table(patch_table)) = doc.get("patch") {
                    for (_source_url, dep_item) in patch_table.iter() {
                        if let Item::Table(deps) = dep_item {
                            removed_count += deps.len();
                        }
                    }
                }
                keys_to_remove.push(key.to_string());
            } else if key.starts_with("patch.") {
                // Handle dotted keys like [patch."https://..."]
                if let Some(Item::Table(patch_deps)) = doc.get(key) {
                    removed_count += patch_deps.len();
                }
                keys_to_remove.push(key.to_string());
            }
        }

        // Remove all patch sections
        for key in keys_to_remove {
            doc.remove(&key);
        }

        // Check if the document is now empty or only has whitespace
        let is_empty = doc.as_table().is_empty();

        if is_empty {
            // Delete the config file
            std::fs::remove_file(&config_path)?;
            Ok(ResetReport {
                removed_count,
                config_path,
                config_deleted: true,
            })
        } else {
            // Save the modified config
            std::fs::write(&config_path, doc.to_string())?;
            Ok(ResetReport {
                removed_count,
                config_path,
                config_deleted: false,
            })
        }
    }

    /// Verify that patches are working by running cargo metadata.
    fn verify_patches(&self) -> Result<()> {
        use std::process::Command;

        let output = Command::new("cargo")
            .arg("metadata")
            .arg("--format-version=1")
            .current_dir(&self.workspace_root)
            .output()
            .context("Failed to run cargo metadata")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("cargo metadata failed:\n{}", stderr);
        }

        Ok(())
    }

    /// Clean cargo cache (useful after removing patches).
    pub fn clean_cache(&self) -> Result<()> {
        use std::process::Command;

        println!("{}", "  Cleaning cargo cache...".dimmed());

        let output = Command::new("cargo")
            .arg("clean")
            .current_dir(&self.workspace_root)
            .output()
            .context("Failed to run cargo clean")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("cargo clean failed:\n{}", stderr);
        }

        Ok(())
    }
}

/// Report from applying patches.
#[derive(Debug)]
pub struct PatchReport {
    pub patched_count: usize,
    pub config_path: PathBuf,
    pub verified: bool,
    pub verification_error: Option<String>,
}

/// Report from removing patches.
#[derive(Debug)]
pub struct ResetReport {
    pub removed_count: usize,
    pub config_path: PathBuf,
    pub config_deleted: bool,
}

impl PatchReport {
    pub fn print(&self) {
        println!(
            "\n{} {} patches written to {}",
            "✓".green().bold(),
            self.patched_count,
            self.config_path.display().to_string().bright_white()
        );

        if self.verified {
            println!("{} Patches verified successfully", "✓".green().bold());
        } else if let Some(err) = &self.verification_error {
            println!("{} Verification failed: {}", "✗".red().bold(), err);
            println!(
                "\n{} Run 'cargo build' to diagnose the issue",
                "Suggestion:".cyan().bold()
            );
        }
    }
}

impl ResetReport {
    pub fn print(&self) {
        if self.removed_count == 0 {
            println!("{} No patches found to remove", "Info:".blue().bold());
        } else {
            println!(
                "\n{} {} patches removed",
                "✓".green().bold(),
                self.removed_count
            );

            if self.config_deleted {
                println!(
                    "  {} deleted (empty)",
                    self.config_path.display().to_string().dimmed()
                );
            } else {
                println!(
                    "  {} updated",
                    self.config_path.display().to_string().dimmed()
                );
            }
        }
    }
}

#[cfg(test)]
#[path = "patch_tests.rs"]
mod tests;
