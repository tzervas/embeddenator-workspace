# Cargo Patch Management - Usage Examples

## Overview

The `embeddenator-workspace` tool provides two commands for managing local development patches:
- `patch-local`: Patches git dependencies to use local paths
- `patch-reset`: Removes patches and restores git dependencies

## Basic Usage

### Apply Local Patches

```bash
# From workspace root
embeddenator-workspace patch-local

# Or specify workspace root explicitly
embeddenator-workspace patch-local --workspace-root /path/to/workspace

# With verification (runs cargo metadata to check patches work)
embeddenator-workspace patch-local --verify
```

**Output:**
```
Discovering: Scanning for patchable dependencies in /home/user/workspace...

Discovered: Found 6 patchable dependencies:
  • embeddenator-fs → /home/user/workspace/embeddenator-fs
  • embeddenator-interop → /home/user/workspace/embeddenator-interop
  • embeddenator-io → /home/user/workspace/embeddenator-io
  • embeddenator-obs → /home/user/workspace/embeddenator-obs
  • embeddenator-retrieval → /home/user/workspace/embeddenator-retrieval
  • embeddenator-vsa → /home/user/workspace/embeddenator-vsa

Patching: Applying patches to .cargo/config.toml...

✓ 6 patches written to /home/user/workspace/.cargo/config.toml
✓ Patches verified successfully

Success: Local development mode enabled!
Note: Run 'embeddenator-workspace patch-reset' to restore git dependencies
```

### Remove Patches

```bash
# Basic reset
embeddenator-workspace patch-reset

# With cargo cache cleaning
embeddenator-workspace patch-reset --clean
```

**Output:**
```
Resetting: Removing patches from /home/user/workspace...

✓ 6 patches removed
  /home/user/workspace/.cargo/config.toml deleted (empty)

Success: Git dependencies restored!
```

## What It Does

### patch-local

1. **Discovers** all Cargo.toml files in the workspace
2. **Identifies** available local repositories (embeddenator-*)
3. **Finds** git dependencies that have local equivalents
4. **Generates** patch entries in `.cargo/config.toml`
5. **Verifies** patches work (if --verify flag used)

### Generated Config Format

The tool creates `.cargo/config.toml` in the workspace root:

```toml
['patch."https://github.com/tzervas/embeddenator-vsa"']

['patch."https://github.com/tzervas/embeddenator-vsa"'.embeddenator-vsa]
path = "/home/user/workspace/embeddenator-vsa"

['patch."https://github.com/tzervas/embeddenator-io"']

['patch."https://github.com/tzervas/embeddenator-io"'.embeddenator-io]
path = "/home/user/workspace/embeddenator-io"
```

### patch-reset

1. **Reads** .cargo/config.toml
2. **Removes** all [patch.*] sections
3. **Deletes** config file if empty (preserves other config if present)
4. **Cleans** cargo cache (if --clean flag used)

## Common Workflows

### Development Workflow

```bash
# 1. Clone all repos
git clone https://github.com/tzervas/embeddenator
git clone https://github.com/tzervas/embeddenator-vsa
git clone https://github.com/tzervas/embeddenator-fs
# ... etc

# 2. Enable local development
embeddenator-workspace patch-local

# 3. Make changes across repos
# Edit files in any repo...

# 4. Build and test with local changes
cd embeddenator
cargo build
cargo test

# 5. When done, restore git dependencies
cd ..
embeddenator-workspace patch-reset
```

### CI/Testing Workflow

```bash
# CI should NOT use patches - always use git dependencies
embeddenator-workspace patch-reset  # Ensure no local patches

# Run tests with git deps
cargo test --workspace
```

### Troubleshooting

```bash
# If builds fail after patching, verify patches
embeddenator-workspace patch-local --verify

# If verification fails, check cargo metadata manually
cargo metadata --format-version=1

# Reset and try again
embeddenator-workspace patch-reset --clean
embeddenator-workspace patch-local
```

## Implementation Details

### Dependency Discovery

The tool scans all Cargo.toml files and identifies:
- Git dependencies: `embeddenator-* = { git = "...", ... }`
- Local availability: Checks if repo exists in workspace
- Path resolution: Uses `workspace_root/repo_name`

### Patch Format

Cargo's [patch] mechanism allows replacing dependencies:
- `[patch."https://github.com/user/repo"]` - Source to patch
- `crate-name = { path = "..." }` - Local replacement

This works for:
- dependencies
- dev-dependencies
- build-dependencies

### Config File Location

Patches are written to `.cargo/config.toml` (preferred over root Cargo.toml):
- Workspace-specific (not committed to git typically)
- Easier to manage and reset
- Doesn't modify package manifests

## Command Reference

### patch-local

```
Apply local path patches for git dependencies

Usage: embeddenator-workspace patch-local [OPTIONS]

Options:
      --workspace-root <WORKSPACE_ROOT>
          Workspace root directory (defaults to current directory)
      --verify
          Verify patches with cargo metadata
  -h, --help
          Print help
```

### patch-reset

```
Remove local path patches and restore git dependencies

Usage: embeddenator-workspace patch-reset [OPTIONS]

Options:
      --workspace-root <WORKSPACE_ROOT>
          Workspace root directory (defaults to current directory)
      --clean
          Clean cargo cache after removing patches
  -h, --help
          Print help
```

## Exit Codes

- `0` - Success
- `1` - Error (missing repos, invalid config, verification failed)

## Notes

- Patches are workspace-specific, not package-specific
- The tool only patches embeddenator-* crates with local equivalents
- Non-git dependencies are ignored
- External git dependencies (non-embeddenator) are not patched
- `.cargo/config.toml` is created if it doesn't exist
- Existing config content is preserved during reset
