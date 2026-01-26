# embeddenator-workspace

Workspace management utilities for developing the Embeddenator monorepo.

This crate provides CLI tools for managing version consistency, dependency updates, and documentation generation across the 11+ embeddenator repositories.

**Independent component** for managing the [Embeddenator workspace](https://github.com/tzervas/embeddenator).

**Repository:** [https://github.com/tzervas/embeddenator-workspace](https://github.com/tzervas/embeddenator-workspace)

## Features

- **Version Management**: Bump versions consistently across all packages
- **Dependency Tracking**: Check for version drift and inconsistencies
- **Workspace Health**: Comprehensive health checks for git, tests, docs, and specs
- **Patch Management**: Local development mode for git dependencies
- **Documentation Generation**: Build rustdoc and mdBook documentation

## Status

Alpha / internal use only.

## Installation

From the workspace root:

```bash
cargo build -p embeddenator-workspace --release
```

The binary will be available at `target/release/embeddenator-workspace`.

## Commands

### bump-version

Update package versions across all embeddenator crates and their inter-dependencies.

```bash
# Bump prerelease version (e.g., 0.20.0-alpha.1 → 0.20.0-alpha.2)
embeddenator-workspace bump-version --prerelease

# Bump patch version (e.g., 0.20.0-alpha.1 → 0.20.1)
embeddenator-workspace bump-version --patch

# Bump minor version (e.g., 0.20.0 → 0.21.0)
embeddenator-workspace bump-version --minor

# Bump major version (e.g., 0.20.0 → 1.0.0)
embeddenator-workspace bump-version --major

# Dry run - see what would change without making modifications
embeddenator-workspace bump-version --prerelease --dry-run
```

**What it does:**
1. Scans all `Cargo.toml` files in the workspace
2. Updates `package.version` in each embeddenator package
3. Updates dependency versions (e.g., `embeddenator-vsa = "0.20.0-alpha.1"`)
4. Writes changes to disk
5. Suggests a git commit command

**Example output:**
```
Performing Prerelease version bump...

Version Changes:
  embeddenator 0.20.0-alpha.1 → 0.20.0-alpha.2
  embeddenator-cli 0.20.0-alpha.1 → 0.20.0-alpha.2
  embeddenator-fs 0.21.0 → 0.21.1-alpha.1
  embeddenator-vsa 0.20.0-alpha.1 → 0.20.0-alpha.2
  ...

✓ 11 package(s) updated

Next: git commit -am "chore: bump version to 0.20.0-alpha.2"
```

### check-versions

Detect version inconsistencies and drift across the workspace.

```bash
# Check version consistency
embeddenator-workspace check-versions

# Show detailed information
embeddenator-workspace check-versions --verbose
```

**What it checks:**
- Version drift (packages on different major versions)
- Dependency mismatches (package A depends on package B v0.20.0 but B is at v0.21.0)
- Prerelease tag consistency

**Example output (with issues):**
```
Checking version consistency...

Scanned: 11 package(s)

Issues Found:
  • Version drift: 1 package(s) on major version 0: embeddenator-fs

Dependency Inconsistencies:
  • embeddenator-fs depends on embeddenator-vsa 0.20.0-alpha.1 (expected: 0.20.0-alpha.2)
  • embeddenator-retrieval depends on embeddenator-vsa 0.20.0-alpha.1 (expected: 0.20.0-alpha.2)

Suggestion: Run 'embeddenator-workspace bump-version --prerelease' to fix

# Exit code: 1
```

**Example output (no issues):**
```
Checking version consistency...

Scanned: 11 package(s)

✓ All versions are consistent!

# Exit code: 0
```

### health

Run comprehensive workspace health checks across all repositories.

```bash
# Run all health checks
embeddenator-workspace health

# Run specific checks only
embeddenator-workspace health --check git,version

# Show detailed output
embeddenator-workspace health --verbose

# Output as JSON for CI parsing
embeddenator-workspace health --json

# Save markdown report to file
embeddenator-workspace health --output health-report.md

# Specify workspace root
embeddenator-workspace health --workspace-root /path/to/workspace
```

**Health Check Categories:**

1. **Git Status** (`--check git`)
   - Dirty files (uncommitted changes)
   - Ahead/behind status vs upstream
   - Branch information
   - Orphaned upstream branches

2. **Version Alignment** (`--check version`)
   - Version consistency across packages
   - Dependency version drift
   - Prerelease tag alignment

3. **Test Coverage** (`--check tests`)
   - Run `cargo test` on all packages
   - Report pass/fail status
   - Identify failing test suites

4. **Documentation Coverage** (`--check docs`)
   - Run `cargo rustdoc` with `-D warnings`
   - Detect missing documentation
   - Count documentation warnings

5. **Spec Coverage** (`--check specs`)
   - Check for `specs/` directories
   - Calculate coverage percentage
   - Identify packages without specs

**Exit Codes:**
- `0`: All checks passed or warnings only
- `1`: Critical failures detected (dirty git, version drift, failing tests)

**Example output:**
```
════════════════════════════════════════════════════════════════════════════════
Workspace Health Report
════════════════════════════════════════════════════════════════════════════════
Generated: 2026-01-16T12:34:56-05:00
Workspace: /home/user/projects/embdntr
Overall Status: WARN

✓ git [Pass]
  All 11 repositories are clean and synced

✗ version [Fail]
  Version inconsistencies detected: 0 issue(s), 2 dependency mismatch(es)
    • embeddenator-fs depends on embeddenator-vsa 0.20.0 (expected: 0.20.1)
    • embeddenator-cli depends on embeddenator-vsa 0.20.0 (expected: 0.20.1)

✓ tests [Pass]
  Tests: 11 passed, 0 failed out of 11 packages

⚠ docs [Warn]
  Documentation: 9 clean, 2 with warnings out of 11 packages
    • embeddenator-io: 3 documentation warning(s)
    • embeddenator-obs: 1 documentation warning(s)

⚠ specs [Warn]
  Spec coverage: 81.8% (9/11 packages with specs/)
    • embeddenator-io: missing specs/ directory
    • embeddenator-obs: missing specs/ directory

════════════════════════════════════════════════════════════════════════════════
```

**JSON Output** (`--json`):
```json
{
  "timestamp": "2026-01-16T12:34:56-05:00",
  "workspace_root": "/home/user/projects/embdntr",
  "overall_status": "warn",
  "checks": [
    {
      "check_type": "git",
      "status": "pass",
      "message": "All 11 repositories are clean and synced",
      "details": []
    },
    {
      "check_type": "version",
      "status": "fail",
      "message": "Version inconsistencies detected",
      "details": [
        "embeddenator-fs depends on embeddenator-vsa 0.20.0 (expected: 0.20.1)"
      ]
    }
  ]
}
```

### patch-local / patch-reset

Manage local development patches for git dependencies (see [PATCH_MANAGEMENT_GUIDE.md](PATCH_MANAGEMENT_GUIDE.md)).

```bash
# Enable local development mode
embeddenator-workspace patch-local

# Disable and restore git dependencies
embeddenator-workspace patch-reset --clean
```

### docs / rustdoc / mdbook

Generate documentation:

```bash
# Generate both rustdoc and mdBook (if available)
embeddenator-workspace docs

# Generate only rustdoc
embeddenator-workspace rustdoc

# Generate only mdBook
embeddenator-workspace mdbook
```

## Typical Workflow

### Before a release:

1. **Run health checks**:
   ```bash
   embeddenator-workspace health
   ```

2. **Fix any critical issues** (version drift, failing tests)

3. **Check consistency**:
   ```bash
   embeddenator-workspace check-versions
   ```

4. **Bump versions** if needed:
   ```bash
   embeddenator-workspace bump-version --patch
   ```

5. **Verify changes**:
   ```bash
   git diff
   ```

6. **Commit**:
   ```bash
   git commit -am "chore: bump version to 0.20.1"
   git push
   ```

### During development:

Use `--dry-run` to preview changes:
```bash
embeddenator-workspace bump-version --prerelease --dry-run
```

Run specific health checks:
```bash
embeddenator-workspace health --check git,version
```

## Integration with CI

### Health Check Pipeline

```yaml
# .github/workflows/health-check.yml
name: Workspace Health Check

on: [pull_request]

jobs:
  health:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      
      - name: Build health checker
        run: cargo build -p embeddenator-workspace --release
      
      - name: Run health checks
        run: |
          ./target/release/embeddenator-workspace health --json > health-report.json
          ./target/release/embeddenator-workspace health --output health-report.md
      
      - name: Upload health report
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: health-report
          path: |
            health-report.json
            health-report.md
      
      - name: Check for failures
        run: |
          if jq -e '.overall_status == "fail"' health-report.json; then
            echo "Health check failed!"
            exit 1
          fi
```

### Version Consistency Check

```yaml
# .github/workflows/version-check.yml
name: Version Consistency Check

on: [pull_request]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      - name: Check version consistency
        run: |
          cargo build -p embeddenator-workspace
          cargo run -p embeddenator-workspace -- check-versions
```

## Architecture

### Module Structure

- `cargo.rs` - Cargo.toml parsing and manipulation using `toml_edit`
- `version.rs` - Version bumping logic with `semver`
- `workspace.rs` - Repository discovery and scanning with `walkdir`
- `patch.rs` - Patch management for git dependencies
- `health.rs` - Comprehensive workspace health checking with parallel execution
- `bin/embeddenator_workspace.rs` - CLI interface using `clap`

### Design Principles

1. **Safety First**: All operations use `toml_edit` to preserve formatting and comments
2. **Parallel Execution**: Health checks run concurrently using `tokio` for performance
3. **Dry Run Support**: Preview changes before applying them
4. **Clear Output**: Color-coded, structured output for easy parsing
5. **Exit Codes**: Non-zero exit on errors for CI integration
6. **Multiple Output Formats**: Terminal (colorized), Markdown, and JSON

### Performance Characteristics

- **Health Check Parallelization**: All health checks run concurrently
  - Git status: ~50ms per repo × 11 repos = ~550ms sequential → ~100ms parallel
  - Version check: Single pass over all packages (~200ms)
  - Test execution: Longest running (~5-30s per package, run in parallel)
  - Doc check: ~2-5s per package (parallel)
  - Spec coverage: Fast filesystem scan (~50ms)
  
- **Expected Runtime**:
  - All checks with passing tests: 5-10 seconds
  - All checks with failing tests: 10-30 seconds (depends on test suites)
  - Specific checks only (`--check git,version`): <1 second

## Running

From anywhere in the workspace:

```bash
cargo run -p embeddenator-workspace -- --help
cargo run -p embeddenator-workspace -- health --help

## License

MIT
