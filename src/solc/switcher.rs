use crate::solc::manager::SolcManager;
use crate::util::log::log_to_file;

use anyhow::{Context, Result};
use regex::Regex;
use semver::{Version, VersionReq};
use std::fs;
use std::path::{Path, PathBuf};
use which::which;

/// Extracts `pragma solidity ^0.8.0` or similar and parses it.
pub fn extract_pragma_version(source_path: &Path) -> Result<VersionReq> {
    let content = fs::read_to_string(source_path)
        .with_context(|| format!("Reading source file: {:?}", source_path))?;

    for line in content.lines() {
        if let Some(idx) = line.find("pragma solidity") {
            let rest = &line[idx + "pragma solidity".len()..];
            let version_str = rest
                .trim()
                .trim_end_matches(';')
                .trim_matches(|c: char| c == '^' || c == '=' || c == '>' || c == '<' || c == '~')
                .split_whitespace()
                .next()
                .unwrap_or("");

            if Version::parse(version_str).is_ok() {
                let req_str = rest.trim().trim_end_matches(';');
                return VersionReq::parse(req_str).context("Parsing version requirement");
            }
        }
    }

    Err(anyhow::anyhow!("No pragma solidity directive found"))
}

/// Finds the best matching version from SolcList that has been downloaded
pub fn match_cached_solc_version(manager: &SolcManager, req: &VersionReq) -> Option<String> {
    manager
        .list
        .builds
        .iter()
        .filter_map(|release| {
            Version::parse(&release.version).ok().map(|ver| (ver, &release.version))
        })
        .filter(|(ver, v_str)| req.matches(ver) && manager.get_binary_path(v_str).is_some())
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, v)| v.to_string())
}

/// Resolve solc binary path for given source based on downloaded binaries
/// Falls back to system solc if no match found
pub fn get_solc_binary_from_cache(
    source_path: &Path,
    _project_root: &Path,
) -> std::io::Result<PathBuf> {
    let req = extract_pragma_version(source_path)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from(".cache"))
        .join("emacs-solidity-server/solc");

    let version_re = Regex::new(r"^solc-(\d+\.\d+\.\d+)$").unwrap();
    let mut candidates = Vec::new();

    for entry in fs::read_dir(&cache_dir)? {
        let entry = entry?;
        let fname = entry.file_name().to_string_lossy().to_string();

        if let Some(cap) = version_re.captures(&fname) {
            if let Some(ver_str) = cap.get(1) {
                if let Ok(ver) = Version::parse(ver_str.as_str()) {
                    if req.matches(&ver) {
                        candidates.push((ver, entry.path()));
                    }
                }
            }
        }
    }

    candidates.sort_by(|a, b| b.0.cmp(&a.0)); // latest version first

    if let Some((ver, path)) = candidates.first() {
        log_to_file(&format!("Using cached solc: {} → {:?}", ver, path));
        Ok(path.clone())
    } else {
        log_to_file(&format!(
            "No cached solc version matched {}; falling back to system solc",
            req
        ));

        which("solc").map_err(|e| std::io::Error::new(std::io::ErrorKind::NotFound, e))
    }
}
