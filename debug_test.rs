// Simple debug test
use embeddenator_workspace::PatchManager;
use std::fs;
use tempfile::TempDir;

fn main() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create mock repo directories
    let repos = vec![
        "embeddenator-vsa",
        "embeddenator-fs",
    ];

    for repo in repos {
        let repo_path = root.join(repo);
        fs::create_dir_all(&repo_path).unwrap();

        // Create a simple Cargo.toml
        let manifest_content = format!(
            r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
            repo
        );
        fs::write(repo_path.join("Cargo.toml"), manifest_content).unwrap();
    }

    // Create a main package with git dependencies
    let main_path = root.join("embeddenator");
    fs::create_dir_all(&main_path).unwrap();

    let main_manifest = r#"[package]
name = "embeddenator"
version = "0.20.0"
edition = "2021"

[dependencies]
embeddenator-vsa = { git = "https://github.com/tzervas/embeddenator-vsa", tag = "v0.1.0" }
embeddenator-fs = { git = "https://github.com/tzervas/embeddenator-fs", branch = "main" }
serde = "1.0"
"#;
    fs::write(main_path.join("Cargo.toml"), main_manifest).unwrap();

    println!("Created workspace at: {:?}", root);
    
    let manager = PatchManager::new(root);
    
    match manager.discover_patchable_dependencies() {
        Ok(deps) => {
            println!("Found {} dependencies:", deps.len());
            for dep in deps {
                println!("  - {} -> {:?}", dep.name, dep.local_path);
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}
