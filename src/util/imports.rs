use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use pathdiff::diff_paths;
use regex::Regex;

/// Recursively resolves relative Solidity imports into a map of virtual path → source content.
pub fn resolve_sources_recursive(
    project_root: &Path,
    physical_path: &Path,
    visited: &mut HashSet<PathBuf>,
) -> HashMap<String, String> {
    let mut sources = HashMap::new();

    // This handles:
    // import "./X.sol";
    // import {X} from "../Y/X.sol";
    // import {X as Y} from "../Z/X.sol";
    let import_re = Regex::new(r#"import\s+(?:\{[^}]*\}\s+from\s+)?["']([^"']+)["']"#).unwrap();

    fn walk(
        project_root: &Path,
        phys: &Path,
        visited: &mut HashSet<PathBuf>,
        acc: &mut HashMap<String, String>,
        re: &Regex,
    ) {
        if !visited.insert(phys.to_path_buf()) {
            return; // already visited
        }

        let Ok(code) = fs::read_to_string(phys) else {
            return;
        };

        let virt = diff_paths(phys, project_root)
            .unwrap_or_else(|| phys.to_path_buf())
            .to_string_lossy()
            .replace('\\', "/");

        acc.insert(virt.clone(), code.clone());

        let dir = phys.parent().unwrap_or(Path::new("."));
        for cap in re.captures_iter(&code) {
            let imp = cap[1].trim();
            if !imp.starts_with('.') {
                continue; // skip non-relative imports
            }
            let child_phys = dir.join(imp);
            if let Ok(abs_child) = child_phys.canonicalize() {
                walk(project_root, &abs_child, visited, acc, re);
            }
        }
    }

    walk(project_root, physical_path, visited, &mut sources, &import_re);
    sources
}
