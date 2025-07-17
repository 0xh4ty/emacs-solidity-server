use std::collections::HashSet;
use std::io::{Result, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::json;

use crate::project::remappings::Remapping;
use crate::util::imports::resolve_sources_recursive;
use crate::util::log::log_to_file;

use crate::analysis::definitions::extract_definitions_from_solc_json;
use crate::analysis::definitions::DEFINITION_MAP;

use crate::solc::switcher::get_solc_binary_from_cache;

pub fn run_solc(
    source_path: &Path,
    source_code: &str,
    remappings: &[Remapping],
    project_root: &Path,
) -> Result<Output> {
    log_to_file("=== run_solc ==================================================");

    let mut visited = HashSet::new();
    let mut sources = resolve_sources_recursive(project_root, source_path, &mut visited);

    let entry_virtual = sources
        .keys()
        .find(|k| sources[*k].as_ptr() == source_path.to_string_lossy().as_ptr())
        .cloned()
        .unwrap_or_else(|| {
            pathdiff::diff_paths(source_path, project_root)
                .unwrap_or_else(|| PathBuf::from("input.sol"))
                .to_string_lossy()
                .replace('\\', "/")
        });
    sources.insert(entry_virtual.clone(), source_code.to_string());

    let remap_strings: Vec<String> = remappings
        .iter()
        .map(|r| format!("{}={}", r.prefix, r.target.display()))
        .collect();
    log_to_file(&format!("Remappings: {:?}", remap_strings));

    let sources_json = sources
        .into_iter()
        .map(|(k, v)| (k, json!({ "content": v })))
        .collect::<serde_json::Map<_, _>>();

    let input_json = json!({
        "language": "Solidity",
        "sources": sources_json,
        "settings": {
            "remappings": remap_strings,
            "outputSelection": { "*": { "*": [], "": ["ast"] } }
        }
    });

    let solc_binary = get_solc_binary_from_cache(source_path, project_root)?;

    log_to_file(&format!("Using solc binary: {}", solc_binary.to_string_lossy()));

    let mut child = Command::new(solc_binary)
        .arg("--standard-json")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(input_json.to_string().as_bytes())?;

    let out = child.wait_with_output()?;
    log_to_file(&format!("solc exited with status {:?}", out.status));
    log_to_file(&format!("STDOUT bytes: {}", out.stdout.len()));
    log_to_file(&format!("STDERR bytes: {}", out.stderr.len()));

    if let Ok(parsed_json) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
        let defs_per_file = extract_definitions_from_solc_json(&parsed_json, project_root);

//        for (file, defs) in &defs_per_file {
//            log_to_file(&format!("Definitions in {}:", file));
//        }

        if let Ok(mut map) = DEFINITION_MAP.lock() {
            for (uri, defs) in defs_per_file {
                map.insert(uri, defs);
            }
        }
    } else {
        log_to_file("⚠️  Could not parse solc stdout as JSON");
    }

    Ok(out)
}
