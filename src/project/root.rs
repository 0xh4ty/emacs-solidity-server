use std::path::{Path, PathBuf};

const PROJECT_FILES: [&str; 5] = [
    "foundry.toml",
    "remappings.txt",
    "hardhat.config.js",
    "hardhat.config.ts",
    "truffle-config.js",
];

pub fn find_project_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    let mut last_match = None;

    loop {
        for file in &PROJECT_FILES {
            if current.join(file).exists() {
                last_match = Some(current.clone());
            }
        }

        if !current.pop() {
            break;
        }
    }

    last_match
}
