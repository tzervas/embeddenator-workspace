//! Workspace management utilities for embeddenator multi-repo development.
//!
//! This crate provides tools for managing version consistency, dependencies,
//! and synchronization across the embeddenator workspace.

pub mod cargo;
pub mod health;
pub mod patch;
pub mod version;
pub mod workspace;

#[cfg(test)]
mod health_tests;

pub use cargo::CargoManifest;
pub use health::{HealthCheckType, HealthChecker, HealthReport, HealthStatus};
pub use patch::{GitDependency, PatchManager, PatchReport, ResetReport};
pub use version::{BumpType, VersionManager};
pub use workspace::WorkspaceScanner;
