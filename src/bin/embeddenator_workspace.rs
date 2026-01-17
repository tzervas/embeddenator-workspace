use clap::{Parser, Subcommand};
use colored::Colorize;
use embeddenator_workspace::{
    BumpType, HealthCheckType, HealthChecker, PatchManager, VersionManager,
};
use std::process::{Command, ExitCode};

#[derive(Parser)]
#[command(name = "embeddenator-workspace")]
#[command(about = "Workspace management utilities for embeddenator development")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate documentation (rustdoc + mdBook)
    Docs,
    /// Run rustdoc
    Rustdoc,
    /// Run mdBook
    Mdbook,
    /// Bump version across all packages
    BumpVersion {
        /// Bump major version (X.0.0)
        #[arg(long, group = "bump_type")]
        major: bool,
        /// Bump minor version (0.X.0)
        #[arg(long, group = "bump_type")]
        minor: bool,
        /// Bump patch version (0.0.X)
        #[arg(long, group = "bump_type")]
        patch: bool,
        /// Bump prerelease version (0.0.0-alpha.X)
        #[arg(long, group = "bump_type")]
        prerelease: bool,
        /// Show what would be changed without making changes
        #[arg(long)]
        dry_run: bool,
    },
    /// Check version consistency across packages
    CheckVersions {
        /// Show detailed information
        #[arg(long)]
        verbose: bool,
    },
    /// Apply local path patches for git dependencies
    PatchLocal {
        /// Workspace root directory (defaults to current directory)
        #[arg(long)]
        workspace_root: Option<String>,
        /// Verify patches with cargo metadata
        #[arg(long)]
        verify: bool,
    },
    /// Remove local path patches and restore git dependencies
    PatchReset {
        /// Workspace root directory (defaults to current directory)
        #[arg(long)]
        workspace_root: Option<String>,
        /// Clean cargo cache after removing patches
        #[arg(long)]
        clean: bool,
    },
    /// Check workspace health (git status, versions, tests, docs, specs)
    Health {
        /// Workspace root directory (defaults to current directory)
        #[arg(long)]
        workspace_root: Option<String>,
        /// Show detailed information
        #[arg(long)]
        verbose: bool,
        /// Output as JSON instead of terminal/markdown
        #[arg(long)]
        json: bool,
        /// Write markdown report to file
        #[arg(long)]
        output: Option<String>,
        /// Run specific checks only (git, version, tests, docs, specs)
        #[arg(long, value_delimiter = ',')]
        check: Vec<String>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::Docs => docs(),
        Commands::Health {
            workspace_root,
            verbose,
            json,
            output,
            check,
        } => health(workspace_root, verbose, json, output, check),
        Commands::Rustdoc => rustdoc(),
        Commands::Mdbook => mdbook(),
        Commands::BumpVersion {
            major,
            minor,
            patch,
            prerelease,
            dry_run,
        } => bump_version(major, minor, patch, prerelease, dry_run),
        Commands::CheckVersions { verbose } => check_versions(verbose),
        Commands::PatchLocal {
            workspace_root,
            verify,
        } => patch_local(workspace_root, verify),
        Commands::PatchReset {
            workspace_root,
            clean,
        } => patch_reset(workspace_root, clean),
    }
}

fn bump_version(
    major: bool,
    minor: bool,
    patch: bool,
    _prerelease: bool,
    dry_run: bool,
) -> ExitCode {
    // Determine bump type (default to prerelease if none specified)
    let bump_type = if major {
        BumpType::Major
    } else if minor {
        BumpType::Minor
    } else if patch {
        BumpType::Patch
    } else {
        BumpType::Prerelease
    };

    // Find workspace root (go up until we find update_all.sh or are at root)
    let workspace_root = std::env::current_dir().expect("Failed to get current directory");
    let workspace_root = find_workspace_root(&workspace_root).unwrap_or(workspace_root);

    let manager = VersionManager::new(&workspace_root);

    if dry_run {
        println!(
            "{}",
            "Dry run mode - no changes will be made".yellow().bold()
        );
    }

    println!(
        "{} {:?} version bump...",
        "Performing".cyan().bold(),
        bump_type
    );

    match manager.bump_versions(bump_type, dry_run) {
        Ok(changes) => {
            if changes.is_empty() {
                println!("{}", "No packages found to update".yellow());
                return ExitCode::from(1);
            }

            println!("\n{}", "Version Changes:".green().bold());
            for change in &changes {
                println!(
                    "  {} {} → {}",
                    change.package.bright_white().bold(),
                    change.old_version.to_string().red(),
                    change.new_version.to_string().green()
                );
            }

            if !dry_run {
                println!(
                    "\n{} {} package(s) updated",
                    "✓".green().bold(),
                    changes.len()
                );
                println!(
                    "\n{} git commit -am \"chore: bump version to {}\"",
                    "Next:".cyan().bold(),
                    changes[0].new_version
                );
            } else {
                println!(
                    "\n{} {} package(s) would be updated",
                    "Info:".blue().bold(),
                    changes.len()
                );
            }

            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{} {}", "Error:".red().bold(), e);
            ExitCode::from(1)
        }
    }
}

fn check_versions(verbose: bool) -> ExitCode {
    let workspace_root = std::env::current_dir().expect("Failed to get current directory");
    let workspace_root = find_workspace_root(&workspace_root).unwrap_or(workspace_root);

    let manager = VersionManager::new(&workspace_root);

    println!("{}", "Checking version consistency...".cyan().bold());

    match manager.check_consistency() {
        Ok(report) => {
            println!(
                "\n{} {} package(s) scanned",
                "Scanned:".blue().bold(),
                report.total_packages
            );

            if report.has_issues() {
                println!("\n{}", "Issues Found:".red().bold());

                // Report version drift
                for issue in &report.issues {
                    println!("  {} {}", "•".red(), issue);
                }

                // Report dependency inconsistencies
                if !report.inconsistencies.is_empty() {
                    println!("\n{}", "Dependency Inconsistencies:".yellow().bold());
                    for inc in &report.inconsistencies {
                        println!(
                            "  {} {} depends on {} {} (expected: {})",
                            "•".yellow(),
                            inc.package.bright_white(),
                            inc.dependency,
                            inc.found.to_string().red(),
                            inc.expected.to_string().green()
                        );
                    }
                }

                println!(
                    "\n{} Run 'embeddenator-workspace bump-version --prerelease' to fix",
                    "Suggestion:".cyan().bold()
                );

                ExitCode::from(1)
            } else {
                println!("\n{} All versions are consistent!", "✓".green().bold());

                if verbose {
                    println!("\n{}", "Package Versions:".blue().bold());
                    // This would require additional data from the report
                    // For now, just show success
                }

                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("{} {}", "Error:".red().bold(), e);
            ExitCode::from(1)
        }
    }
}

fn find_workspace_root(start: &std::path::Path) -> Option<std::path::PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join("update_all.sh").exists() || current.join("embeddenator").is_dir() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

fn run(cmd: &mut Command) -> ExitCode {
    match cmd.status() {
        Ok(st) if st.success() => ExitCode::SUCCESS,
        Ok(st) => ExitCode::from(st.code().unwrap_or(1) as u8),
        Err(e) => {
            eprintln!("Failed to run command: {e}");
            ExitCode::from(1)
        }
    }
}

fn rustdoc() -> ExitCode {
    let mut cmd = Command::new("bash");
    cmd.arg("./generate_docs.sh");
    run(&mut cmd)
}

fn mdbook() -> ExitCode {
    let mut cmd = Command::new("bash");
    cmd.arg("./scripts/docs/build_mdbook.sh");
    run(&mut cmd)
}

fn docs() -> ExitCode {
    let rc = rustdoc();
    if rc != ExitCode::SUCCESS {
        return rc;
    }

    // mdBook is optional; if not installed, the script exits nonzero. Treat that as non-fatal.
    let mut cmd = Command::new("bash");
    cmd.arg("./scripts/docs/build_mdbook.sh");
    match cmd.status() {
        Ok(st) if st.success() => ExitCode::SUCCESS,
        Ok(_) => {
            eprintln!("Note: mdBook not built (mdbook not installed?)");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Note: mdBook not built: {e}");
            ExitCode::SUCCESS
        }
    }
}

fn patch_local(workspace_root: Option<String>, verify: bool) -> ExitCode {
    let workspace_root = resolve_workspace_root(workspace_root);

    println!(
        "{} Scanning for patchable dependencies in {}...",
        "Discovering:".cyan().bold(),
        workspace_root.display().to_string().bright_white()
    );

    let manager = PatchManager::new(&workspace_root);

    match manager.discover_patchable_dependencies() {
        Ok(deps) => {
            if deps.is_empty() {
                println!(
                    "{} No git dependencies with local equivalents found",
                    "Info:".blue().bold()
                );
                return ExitCode::SUCCESS;
            }

            println!(
                "\n{} Found {} patchable dependencies:",
                "Discovered:".green().bold(),
                deps.len()
            );

            for dep in &deps {
                println!(
                    "  {} {} → {}",
                    "•".green(),
                    dep.name.bright_white().bold(),
                    dep.local_path.display().to_string().dimmed()
                );
            }

            println!(
                "\n{} Applying patches to .cargo/config.toml...",
                "Patching:".cyan().bold()
            );

            match manager.apply_patches(&deps, verify) {
                Ok(report) => {
                    report.print();

                    if report.verification_error.is_some() {
                        ExitCode::from(1)
                    } else {
                        println!(
                            "\n{} Local development mode enabled!",
                            "Success:".green().bold()
                        );
                        println!(
                            "{} Run 'embeddenator-workspace patch-reset' to restore git dependencies",
                            "Note:".cyan().bold()
                        );
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("{} {}", "Error:".red().bold(), e);
                    ExitCode::from(1)
                }
            }
        }
        Err(e) => {
            eprintln!("{} {}", "Error:".red().bold(), e);
            ExitCode::from(1)
        }
    }
}

fn patch_reset(workspace_root: Option<String>, clean: bool) -> ExitCode {
    let workspace_root = resolve_workspace_root(workspace_root);

    println!(
        "{} Removing patches from {}...",
        "Resetting:".cyan().bold(),
        workspace_root.display().to_string().bright_white()
    );

    let manager = PatchManager::new(&workspace_root);

    match manager.remove_patches() {
        Ok(report) => {
            report.print();

            if clean && report.removed_count > 0 {
                match manager.clean_cache() {
                    Ok(_) => {
                        println!("{} Cargo cache cleaned", "✓".green().bold());
                    }
                    Err(e) => {
                        eprintln!(
                            "{} Failed to clean cache: {}",
                            "Warning:".yellow().bold(),
                            e
                        );
                    }
                }
            }

            if report.removed_count > 0 {
                println!("\n{} Git dependencies restored!", "Success:".green().bold());
            }

            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{} {}", "Error:".red().bold(), e);
            ExitCode::from(1)
        }
    }
}

fn resolve_workspace_root(workspace_root: Option<String>) -> std::path::PathBuf {
    workspace_root
        .map(std::path::PathBuf::from)
        .or_else(|| {
            let current = std::env::current_dir().ok()?;
            find_workspace_root(&current)
        })
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"))
}

fn health(
    workspace_root: Option<String>,
    verbose: bool,
    json: bool,
    output: Option<String>,
    check: Vec<String>,
) -> ExitCode {
    let workspace_root = resolve_workspace_root(workspace_root);

    println!(
        "{} Checking workspace health in {}...",
        "Analyzing:".cyan().bold(),
        workspace_root.display().to_string().bright_white()
    );

    let checker = HealthChecker::new(&workspace_root);

    // Parse check types
    let check_types = if check.is_empty() {
        // Run all checks
        vec![
            HealthCheckType::Git,
            HealthCheckType::Version,
            HealthCheckType::Tests,
            HealthCheckType::Docs,
            HealthCheckType::Specs,
        ]
    } else {
        let mut types = Vec::new();
        for check_str in &check {
            match check_str.parse::<HealthCheckType>() {
                Ok(t) => types.push(t),
                Err(_) => {
                    eprintln!(
                        "{} Unknown check type: '{}'. Valid types: git, version, tests, docs, specs",
                        "Error:".red().bold(),
                        check_str
                    );
                    return ExitCode::from(1);
                }
            }
        }
        types
    };

    // Run checks asynchronously
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    let report = match runtime.block_on(checker.check_selected(&check_types, verbose)) {
        Ok(report) => report,
        Err(e) => {
            eprintln!("{} {}", "Error:".red().bold(), e);
            return ExitCode::from(1);
        }
    };

    // Output results
    if json {
        match serde_json::to_string_pretty(&report) {
            Ok(json_output) => {
                println!("{}", json_output);
            }
            Err(e) => {
                eprintln!(
                    "{} Failed to serialize to JSON: {}",
                    "Error:".red().bold(),
                    e
                );
                return ExitCode::from(1);
            }
        }
    } else {
        report.print_terminal(verbose);
    }

    // Write markdown report if requested
    if let Some(output_path) = output {
        let markdown = report.to_markdown();
        match std::fs::write(&output_path, markdown) {
            Ok(_) => {
                println!(
                    "\n{} Report written to {}",
                    "Saved:".green().bold(),
                    output_path.bright_white()
                );
            }
            Err(e) => {
                eprintln!("{} Failed to write report: {}", "Error:".red().bold(), e);
                return ExitCode::from(1);
            }
        }
    }

    // Exit with appropriate code
    if report.has_failures() {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
