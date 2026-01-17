//! Workspace health checking utilities.
//!
//! Provides comprehensive health checks for the embeddenator multi-repo workspace,
//! including git status, version alignment, test coverage, doc coverage, and spec coverage.

use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;
use tokio::task::JoinHandle;

use crate::version::VersionManager;

/// Types of health checks that can be performed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthCheckType {
    Git,
    Version,
    Tests,
    Docs,
    Specs,
}

impl FromStr for HealthCheckType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "git" => Ok(Self::Git),
            "version" => Ok(Self::Version),
            "tests" => Ok(Self::Tests),
            "docs" => Ok(Self::Docs),
            "specs" => Ok(Self::Specs),
            _ => Err(format!("Unknown health check type: {}", s)),
        }
    }
}

impl HealthCheckType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Git => "git",
            Self::Version => "version",
            Self::Tests => "tests",
            Self::Docs => "docs",
            Self::Specs => "specs",
        }
    }
}

/// Status of a health check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Pass,
    Warn,
    Fail,
}

impl HealthStatus {
    pub fn is_critical(&self) -> bool {
        matches!(self, Self::Fail)
    }
}

/// Result of a single health check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub check_type: HealthCheckType,
    pub status: HealthStatus,
    pub message: String,
    pub details: Vec<String>,
}

/// Git repository status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    pub repo_path: PathBuf,
    pub branch: String,
    pub is_dirty: bool,
    pub ahead: usize,
    pub behind: usize,
    pub has_upstream: bool,
    pub dirty_files: Vec<String>,
}

/// Overall health report for the workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub timestamp: String,
    pub workspace_root: PathBuf,
    pub checks: Vec<HealthCheckResult>,
    pub overall_status: HealthStatus,
}

impl HealthReport {
    /// Check if the report contains any critical failures.
    pub fn has_failures(&self) -> bool {
        self.checks.iter().any(|c| c.status.is_critical())
    }

    /// Generate a Markdown report.
    pub fn to_markdown(&self) -> String {
        let mut output = String::new();

        output.push_str("# Workspace Health Report\n\n");
        output.push_str(&format!("**Generated:** {}\n", self.timestamp));
        output.push_str(&format!(
            "**Workspace:** `{}`\n\n",
            self.workspace_root.display()
        ));

        let status_emoji = match self.overall_status {
            HealthStatus::Pass => "✅",
            HealthStatus::Warn => "⚠️",
            HealthStatus::Fail => "❌",
        };
        output.push_str(&format!(
            "**Overall Status:** {} {:?}\n\n",
            status_emoji, self.overall_status
        ));

        output.push_str("## Check Results\n\n");

        for check in &self.checks {
            let icon = match check.status {
                HealthStatus::Pass => "✅",
                HealthStatus::Warn => "⚠️",
                HealthStatus::Fail => "❌",
            };

            output.push_str(&format!(
                "### {} {} Check\n\n",
                icon,
                check.check_type.as_str()
            ));
            output.push_str(&format!("**Status:** {:?}\n\n", check.status));
            output.push_str(&format!("{}\n\n", check.message));

            if !check.details.is_empty() {
                output.push_str("**Details:**\n\n");
                for detail in &check.details {
                    output.push_str(&format!("- {}\n", detail));
                }
                output.push('\n');
            }
        }

        output
    }

    /// Print a colorized terminal report.
    pub fn print_terminal(&self, verbose: bool) {
        println!("\n{}", "═".repeat(80).bright_black());
        println!("{}", "Workspace Health Report".bright_white().bold());
        println!("{}", "═".repeat(80).bright_black());

        println!("{} {}", "Generated:".cyan(), self.timestamp);
        println!("{} {}", "Workspace:".cyan(), self.workspace_root.display());

        let status_text = match self.overall_status {
            HealthStatus::Pass => "PASS".green().bold(),
            HealthStatus::Warn => "WARN".yellow().bold(),
            HealthStatus::Fail => "FAIL".red().bold(),
        };
        println!("{} {}\n", "Overall Status:".cyan(), status_text);

        for check in &self.checks {
            let icon = match check.status {
                HealthStatus::Pass => "✓".green(),
                HealthStatus::Warn => "⚠".yellow(),
                HealthStatus::Fail => "✗".red(),
            };

            println!(
                "{} {} {}",
                icon,
                check.check_type.as_str().bright_white().bold(),
                format!("[{:?}]", check.status).dimmed()
            );
            println!("  {}", check.message);

            if verbose && !check.details.is_empty() {
                for detail in &check.details {
                    println!("    • {}", detail.dimmed());
                }
            } else if !verbose && check.details.len() > 3 {
                for detail in check.details.iter().take(3) {
                    println!("    • {}", detail.dimmed());
                }
                println!(
                    "    {} {} more details (use --verbose)",
                    "...".dimmed(),
                    (check.details.len() - 3).to_string().dimmed()
                );
            } else if !check.details.is_empty() {
                for detail in &check.details {
                    println!("    • {}", detail.dimmed());
                }
            }
            println!();
        }

        println!("{}", "═".repeat(80).bright_black());
    }
}

/// Health checker for the workspace.
pub struct HealthChecker {
    workspace_root: PathBuf,
}

impl HealthChecker {
    /// Create a new health checker.
    pub fn new(workspace_root: impl AsRef<Path>) -> Self {
        let workspace_root = workspace_root.as_ref().to_path_buf();
        Self { workspace_root }
    }

    /// Run all health checks in parallel.
    pub async fn check_all(&self, verbose: bool) -> Result<HealthReport> {
        let checks = vec![
            HealthCheckType::Git,
            HealthCheckType::Version,
            HealthCheckType::Tests,
            HealthCheckType::Docs,
            HealthCheckType::Specs,
        ];

        self.check_selected(&checks, verbose).await
    }

    /// Run selected health checks in parallel.
    pub async fn check_selected(
        &self,
        check_types: &[HealthCheckType],
        verbose: bool,
    ) -> Result<HealthReport> {
        let mut handles: Vec<JoinHandle<Result<HealthCheckResult>>> = Vec::new();

        for &check_type in check_types {
            let workspace_root = self.workspace_root.clone();

            let handle = tokio::spawn(async move {
                match check_type {
                    HealthCheckType::Git => {
                        Self::check_git_status_static(&workspace_root, verbose).await
                    }
                    HealthCheckType::Version => {
                        Self::check_version_alignment_static(&workspace_root, verbose).await
                    }
                    HealthCheckType::Tests => {
                        Self::check_tests_static(&workspace_root, verbose).await
                    }
                    HealthCheckType::Docs => {
                        Self::check_docs_static(&workspace_root, verbose).await
                    }
                    HealthCheckType::Specs => {
                        Self::check_spec_coverage_static(&workspace_root, verbose).await
                    }
                }
            });

            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(Ok(result)) => results.push(result),
                Ok(Err(e)) => {
                    eprintln!("Health check failed: {}", e);
                    // Continue with other checks
                }
                Err(e) => {
                    eprintln!("Task panicked: {}", e);
                }
            }
        }

        // Determine overall status
        let overall_status = if results.iter().any(|r| r.status == HealthStatus::Fail) {
            HealthStatus::Fail
        } else if results.iter().any(|r| r.status == HealthStatus::Warn) {
            HealthStatus::Warn
        } else {
            HealthStatus::Pass
        };

        Ok(HealthReport {
            timestamp: chrono::Local::now().to_rfc3339(),
            workspace_root: self.workspace_root.clone(),
            checks: results,
            overall_status,
        })
    }

    /// Check git status across all repositories.
    async fn check_git_status_static(
        workspace_root: &Path,
        verbose: bool,
    ) -> Result<HealthCheckResult> {
        let repos = Self::find_git_repos_static(workspace_root)?;
        let mut all_clean = true;
        let mut details = Vec::new();
        let mut warnings = Vec::new();

        for repo_path in &repos {
            match Self::get_git_status_static(repo_path) {
                Ok(status) => {
                    let repo_name = repo_path
                        .strip_prefix(workspace_root)
                        .unwrap_or(repo_path)
                        .display()
                        .to_string();

                    if status.is_dirty {
                        all_clean = false;
                        details.push(format!(
                            "{}: {} dirty file(s) on branch {}",
                            repo_name,
                            status.dirty_files.len(),
                            status.branch
                        ));

                        if verbose {
                            for file in &status.dirty_files {
                                details.push(format!("  - {}", file));
                            }
                        }
                    }

                    if status.ahead > 0 || status.behind > 0 {
                        warnings.push(format!(
                            "{}: {} ahead, {} behind upstream on {}",
                            repo_name, status.ahead, status.behind, status.branch
                        ));
                    }

                    if !status.has_upstream {
                        warnings.push(format!(
                            "{}: no upstream configured for {}",
                            repo_name, status.branch
                        ));
                    }
                }
                Err(e) => {
                    warnings.push(format!("Failed to check {}: {}", repo_path.display(), e));
                }
            }
        }

        let status = if !all_clean {
            HealthStatus::Fail
        } else if !warnings.is_empty() {
            HealthStatus::Warn
        } else {
            HealthStatus::Pass
        };

        let message = if all_clean && warnings.is_empty() {
            format!("All {} repositories are clean and synced", repos.len())
        } else if !all_clean {
            format!(
                "Found {} repositories with uncommitted changes",
                details.len()
            )
        } else {
            format!("All repositories clean, {} warning(s)", warnings.len())
        };

        details.extend(warnings);

        Ok(HealthCheckResult {
            check_type: HealthCheckType::Git,
            status,
            message,
            details,
        })
    }

    /// Check version alignment across packages.
    async fn check_version_alignment_static(
        workspace_root: &Path,
        _verbose: bool,
    ) -> Result<HealthCheckResult> {
        let version_manager = VersionManager::new(workspace_root);

        match version_manager.check_consistency() {
            Ok(report) => {
                let status = if report.has_issues() {
                    HealthStatus::Fail
                } else {
                    HealthStatus::Pass
                };

                let message = if report.has_issues() {
                    format!(
                        "Version inconsistencies detected: {} issue(s), {} dependency mismatch(es)",
                        report.issues.len(),
                        report.inconsistencies.len()
                    )
                } else {
                    format!(
                        "All {} packages have consistent versions",
                        report.total_packages
                    )
                };

                let mut details = Vec::new();

                for issue in &report.issues {
                    details.push(issue.clone());
                }

                for inc in &report.inconsistencies {
                    details.push(format!(
                        "{} depends on {} {} (expected: {})",
                        inc.package, inc.dependency, inc.found, inc.expected
                    ));
                }

                Ok(HealthCheckResult {
                    check_type: HealthCheckType::Version,
                    status,
                    message,
                    details,
                })
            }
            Err(e) => Ok(HealthCheckResult {
                check_type: HealthCheckType::Version,
                status: HealthStatus::Fail,
                message: format!("Failed to check versions: {}", e),
                details: vec![],
            }),
        }
    }

    /// Check test coverage by running cargo test.
    async fn check_tests_static(
        workspace_root: &Path,
        _verbose: bool,
    ) -> Result<HealthCheckResult> {
        let packages = Self::find_packages_static(workspace_root)?;
        let mut passed = 0;
        let mut failed = 0;
        let mut details = Vec::new();

        for pkg_path in &packages {
            let output = Command::new("cargo")
                .arg("test")
                .arg("--manifest-path")
                .arg(pkg_path.join("Cargo.toml"))
                .arg("--all-features")
                .arg("--")
                .arg("--test-threads=1")
                .arg("--quiet")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output();

            let pkg_name = pkg_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            match output {
                Ok(output) => {
                    if output.status.success() {
                        passed += 1;
                    } else {
                        failed += 1;
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        details.push(format!("{}: tests failed", pkg_name));

                        // Extract test failure summary
                        for line in stderr.lines() {
                            if line.contains("test result:") || line.contains("FAILED") {
                                details.push(format!("  {}", line.trim()));
                            }
                        }
                    }
                }
                Err(e) => {
                    failed += 1;
                    details.push(format!("{}: failed to run tests: {}", pkg_name, e));
                }
            }
        }

        let status = if failed > 0 {
            HealthStatus::Fail
        } else {
            HealthStatus::Pass
        };

        let message = format!(
            "Tests: {} passed, {} failed out of {} packages",
            passed,
            failed,
            packages.len()
        );

        Ok(HealthCheckResult {
            check_type: HealthCheckType::Tests,
            status,
            message,
            details,
        })
    }

    /// Check documentation coverage.
    async fn check_docs_static(workspace_root: &Path, _verbose: bool) -> Result<HealthCheckResult> {
        let packages = Self::find_packages_static(workspace_root)?;
        let mut passed = 0;
        let mut warnings = 0;
        let mut details = Vec::new();

        for pkg_path in &packages {
            let output = Command::new("cargo")
                .arg("rustdoc")
                .arg("--manifest-path")
                .arg(pkg_path.join("Cargo.toml"))
                .arg("--")
                .arg("-D")
                .arg("warnings")
                .arg("--document-private-items")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output();

            let pkg_name = pkg_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            match output {
                Ok(output) => {
                    if output.status.success() {
                        passed += 1;
                    } else {
                        warnings += 1;
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let warning_count = stderr
                            .lines()
                            .filter(|l| {
                                l.contains("warning:") || l.contains("missing documentation")
                            })
                            .count();

                        if warning_count > 0 {
                            details.push(format!(
                                "{}: {} documentation warning(s)",
                                pkg_name, warning_count
                            ));
                        }
                    }
                }
                Err(e) => {
                    warnings += 1;
                    details.push(format!("{}: failed to check docs: {}", pkg_name, e));
                }
            }
        }

        let status = if warnings > 0 {
            HealthStatus::Warn
        } else {
            HealthStatus::Pass
        };

        let message = format!(
            "Documentation: {} clean, {} with warnings out of {} packages",
            passed,
            warnings,
            packages.len()
        );

        Ok(HealthCheckResult {
            check_type: HealthCheckType::Docs,
            status,
            message,
            details,
        })
    }

    /// Check spec coverage (presence of specs/ directories and documentation).
    async fn check_spec_coverage_static(
        workspace_root: &Path,
        _verbose: bool,
    ) -> Result<HealthCheckResult> {
        let packages = Self::find_packages_static(workspace_root)?;
        let mut with_specs = 0;
        let mut without_specs = 0;
        let mut details = Vec::new();

        for pkg_path in &packages {
            let specs_dir = pkg_path.join("specs");
            let pkg_name = pkg_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            if specs_dir.exists() && specs_dir.is_dir() {
                // Count spec files
                let spec_count = walkdir::WalkDir::new(&specs_dir)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                    .filter(|e| {
                        e.path()
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .map(|ext| ext == "md" || ext == "txt")
                            .unwrap_or(false)
                    })
                    .count();

                with_specs += 1;
                if spec_count > 0 {
                    details.push(format!("{}: {} spec file(s)", pkg_name, spec_count));
                }
            } else {
                without_specs += 1;
                details.push(format!("{}: missing specs/ directory", pkg_name));
            }
        }

        let total = packages.len();
        let coverage_pct = if total > 0 {
            (with_specs as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        let status = if without_specs > 0 {
            HealthStatus::Warn
        } else {
            HealthStatus::Pass
        };

        let message = format!(
            "Spec coverage: {:.1}% ({}/{} packages with specs/)",
            coverage_pct, with_specs, total
        );

        Ok(HealthCheckResult {
            check_type: HealthCheckType::Specs,
            status,
            message,
            details,
        })
    }

    // Helper methods

    fn find_git_repos_static(workspace_root: &Path) -> Result<Vec<PathBuf>> {
        let mut repos = Vec::new();

        for entry in walkdir::WalkDir::new(workspace_root)
            .max_depth(2)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !matches!(name.as_ref(), "target" | "node_modules" | ".cargo")
            })
        {
            let entry = entry?;
            if entry.file_type().is_dir() && entry.file_name() == ".git" {
                if let Some(parent) = entry.path().parent() {
                    repos.push(parent.to_path_buf());
                }
            }
        }

        repos.sort();
        Ok(repos)
    }

    fn get_git_status_static(repo_path: &Path) -> Result<GitStatus> {
        let repo = git2::Repository::open(repo_path).context("Failed to open git repository")?;

        let head = repo.head().context("Failed to get HEAD")?;
        let branch = head.shorthand().unwrap_or("(detached)").to_string();

        // Check for dirty files
        let statuses = repo.statuses(None)?;
        let is_dirty = !statuses.is_empty();
        let dirty_files: Vec<String> = statuses
            .iter()
            .filter_map(|s| s.path().map(String::from))
            .collect();

        // Check upstream
        let (ahead, behind, has_upstream) =
            if let Ok(local_branch) = repo.find_branch(&branch, git2::BranchType::Local) {
                if let Ok(upstream) = local_branch.upstream() {
                    let local_oid = local_branch.get().target().context("No local target")?;
                    let upstream_oid = upstream.get().target().context("No upstream target")?;

                    let (ahead, behind) = repo.graph_ahead_behind(local_oid, upstream_oid)?;
                    (ahead, behind, true)
                } else {
                    (0, 0, false)
                }
            } else {
                (0, 0, false)
            };

        Ok(GitStatus {
            repo_path: repo_path.to_path_buf(),
            branch,
            is_dirty,
            ahead,
            behind,
            has_upstream,
            dirty_files,
        })
    }

    fn find_packages_static(workspace_root: &Path) -> Result<Vec<PathBuf>> {
        let mut packages = Vec::new();

        for entry in walkdir::WalkDir::new(workspace_root)
            .max_depth(2)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !matches!(name.as_ref(), "target" | ".git" | "node_modules" | ".cargo")
            })
        {
            let entry = entry?;
            if entry.file_type().is_dir() {
                let cargo_toml = entry.path().join("Cargo.toml");
                if cargo_toml.exists() {
                    // Only include embeddenator-* packages
                    if let Some(name) = entry.path().file_name() {
                        if name.to_string_lossy().starts_with("embeddenator") {
                            packages.push(entry.path().to_path_buf());
                        }
                    }
                }
            }
        }

        packages.sort();
        Ok(packages)
    }
}

// Note: chrono is not in dependencies yet, using a simple timestamp instead
mod chrono {
    pub struct Local;
    impl Local {
        pub fn now() -> DateTime {
            DateTime
        }
    }
    pub struct DateTime;
    impl DateTime {
        pub fn to_rfc3339(&self) -> String {
            use std::time::{SystemTime, UNIX_EPOCH};
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            format!("{}", now)
        }
    }
}
