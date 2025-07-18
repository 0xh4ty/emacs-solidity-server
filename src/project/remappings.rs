use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct Remapping {
    pub prefix: String,
    pub target: PathBuf,
}

pub fn parse_remappings_txt(path: &Path) -> Vec<Remapping> {
    if let Ok(content) = fs::read_to_string(path) {
        content
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.trim().split('=').map(str::trim).collect();
                if parts.len() == 2 {
                    Some(Remapping {
                        prefix: parts[0].to_string(),
                        target: PathBuf::from(parts[1]),
                    })
                } else {
                    None
                }
            })
            .collect()
    } else {
        vec![]
    }
}

pub fn parse_foundry_toml(path: &Path) -> Vec<Remapping> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut remappings = vec![];
    let mut in_remappings_block = false;

    for line in content.lines() {
        let line = line.trim();

        if line.starts_with("[") {
            in_remappings_block = line == "[remappings]";
            continue;
        }

        if in_remappings_block && line.contains('=') {
            let parts: Vec<&str> = line.trim_matches('"').split('=').map(str::trim).collect();
            if parts.len() == 2 {
                remappings.push(Remapping {
                    prefix: parts[0].to_string(),
                    target: PathBuf::from(parts[1]),
                });
            }
        }
    }

    remappings
}

fn has_hardhat_config(root: &Path) -> bool {
    root.join("hardhat.config.js").exists() || root.join("hardhat.config.ts").exists()
}

pub fn parse_remappings(project_root: &Path) -> Vec<Remapping> {
    let mut seen = HashSet::new();
    let mut all = vec![];

    for rem in parse_remappings_txt(&project_root.join("remappings.txt"))
        .into_iter()
        .chain(parse_foundry_toml(&project_root.join("foundry.toml")))
    {
        let key = format!("{}={}", rem.prefix, rem.target.display());
        if seen.insert(key) {
            all.push(rem);
        }
    }
    // If hardhat.config.js or hardhat.config.ts exists
    if has_hardhat_config(project_root) {
        let node_modules_remap = Remapping {
            prefix: "@".to_string(),
            target: PathBuf::from("node_modules/@"),
        };

        let key = format!("{}={}", node_modules_remap.prefix, node_modules_remap.target.display());
        if seen.insert(key) {
            all.push(node_modules_remap);
        }
    }
    all
}
