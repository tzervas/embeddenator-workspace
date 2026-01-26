//! Cargo.toml file parsing and manipulation utilities.

use anyhow::{Context, Result};
use semver::Version;
use std::path::{Path, PathBuf};
use toml_edit::{value, DocumentMut, Item};

/// Represents a Cargo.toml manifest file.
#[derive(Debug, Clone)]
pub struct CargoManifest {
    pub path: PathBuf,
    pub package_name: String,
    pub version: Version,
    pub dependencies: Vec<Dependency>,
    document: DocumentMut,
}

/// Represents a dependency in Cargo.toml.
#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub version: Option<Version>,
    pub dep_type: DependencyType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyType {
    Normal,
    Dev,
    Build,
}

impl CargoManifest {
    /// Load a Cargo.toml file from disk.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let document: DocumentMut = content
            .parse()
            .with_context(|| format!("Failed to parse {}", path.display()))?;

        let package_name = document["package"]["name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing package.name in {}", path.display()))?
            .to_string();

        let version_str = document["package"]["version"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing package.version in {}", path.display()))?;

        let version = Version::parse(version_str)
            .with_context(|| format!("Invalid version '{}' in {}", version_str, path.display()))?;

        let mut dependencies = Vec::new();

        // Parse dependencies
        if let Some(Item::Table(deps)) = document.get("dependencies") {
            for (name, item) in deps.iter() {
                if let Some(dep) = Self::parse_dependency(name, item, DependencyType::Normal) {
                    dependencies.push(dep);
                }
            }
        }

        // Parse dev-dependencies
        if let Some(Item::Table(deps)) = document.get("dev-dependencies") {
            for (name, item) in deps.iter() {
                if let Some(dep) = Self::parse_dependency(name, item, DependencyType::Dev) {
                    dependencies.push(dep);
                }
            }
        }

        // Parse build-dependencies
        if let Some(Item::Table(deps)) = document.get("build-dependencies") {
            for (name, item) in deps.iter() {
                if let Some(dep) = Self::parse_dependency(name, item, DependencyType::Build) {
                    dependencies.push(dep);
                }
            }
        }

        Ok(Self {
            path: path.to_path_buf(),
            package_name,
            version,
            dependencies,
            document,
        })
    }

    fn parse_dependency(name: &str, item: &Item, dep_type: DependencyType) -> Option<Dependency> {
        let version = match item {
            Item::Value(val) if val.is_str() => {
                // Simple version string: "0.20.0-alpha.1"
                val.as_str().and_then(|s| Version::parse(s).ok())
            }
            Item::Table(_) => {
                // Table format: { version = "0.20.0-alpha.1", ... }
                item.get("version")
                    .and_then(|v| v.as_str())
                    .and_then(|s| Version::parse(s).ok())
            }
            _ => None,
        };

        Some(Dependency {
            name: name.to_string(),
            version,
            dep_type,
        })
    }

    /// Update the package version.
    pub fn set_version(&mut self, new_version: &Version) -> Result<()> {
        self.version = new_version.clone();

        if let Some(package) = self.document.get_mut("package") {
            if let Some(pkg_table) = package.as_table_mut() {
                pkg_table["version"] = value(new_version.to_string());
            }
        }

        Ok(())
    }

    /// Update a dependency version.
    pub fn update_dependency(&mut self, dep_name: &str, new_version: &Version) -> Result<()> {
        let sections = [
            ("dependencies", DependencyType::Normal),
            ("dev-dependencies", DependencyType::Dev),
            ("build-dependencies", DependencyType::Build),
        ];

        for (section, dep_type) in &sections {
            if let Some(deps) = self.document.get_mut(section) {
                if let Some(deps_table) = deps.as_table_mut() {
                    if let Some(dep_item) = deps_table.get_mut(dep_name) {
                        Self::update_dep_item_static(dep_item, new_version)?;

                        // Update our internal tracking
                        if let Some(dep) = self
                            .dependencies
                            .iter_mut()
                            .find(|d| d.name == dep_name && &d.dep_type == dep_type)
                        {
                            dep.version = Some(new_version.clone());
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn update_dep_item_static(item: &mut Item, new_version: &Version) -> Result<()> {
        match item {
            Item::Value(val) if val.is_str() => {
                // Simple string version
                *item = value(new_version.to_string());
            }
            Item::Table(_) => {
                // Table format with version key
                if let Some(version_item) = item.get_mut("version") {
                    *version_item = value(new_version.to_string());
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Save the manifest back to disk.
    pub fn save(&self) -> Result<()> {
        std::fs::write(&self.path, self.document.to_string())
            .with_context(|| format!("Failed to write {}", self.path.display()))?;
        Ok(())
    }

    /// Get all embeddenator-* dependencies.
    pub fn embeddenator_dependencies(&self) -> Vec<&Dependency> {
        self.dependencies
            .iter()
            .filter(|d| d.name.starts_with("embeddenator-"))
            .collect()
    }
}

#[cfg(test)]
#[path = "cargo_tests.rs"]
mod tests;
