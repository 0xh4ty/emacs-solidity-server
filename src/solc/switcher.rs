use crate::solc::manager::SolcManager;
use crate::solc::manager::make_executable;
use crate::util::log::log_to_file;

use anyhow::{Context, Result};
use regex::Regex;
use semver::{Version, VersionReq};
use std::fs;
use std::path::{Path, PathBuf};
use which::which;
use std::{thread, time::Duration};

use crate::solc::fetch::{download_to_file, verify_sha256};
use crate::solc::platform::get_platform_id;
use crate::solc::versions::SolcList;

pub enum Pragma {
    Exact(Version),
    Range(VersionReq),
}

pub fn extract_pragma(source_path: &Path) -> Result<Pragma> {
    let content = fs::read_to_string(source_path)
        .with_context(|| format!("Reading source file: {:?}", source_path))?;

    for line in content.lines() {
        if let Some(idx) = line.find("pragma solidity") {
            let rest = line[idx + "pragma solidity".len()..]
                .trim()
                .trim_end_matches(';');

            // If '=' is present anywhere, treat it as exact — take the first version only
            if rest.contains('=') {
                // Capture the first valid version (e.g., from ">=0.8.7 <0.9.0")
                let first = rest
                    .split_whitespace()
                    .next()
                    .and_then(|token| {
                        token.trim_start_matches(|c: char| !c.is_digit(10)).parse().ok()
                    });

                if let Some(v) = first {
                    return Ok(Pragma::Exact(v));
                } else {
                    return Err(anyhow::anyhow!("Could not parse exact version from: '{}'", rest));
                }
            } else {
                return Ok(Pragma::Range(VersionReq::parse(rest)?));
            }
        }
    }

    Err(anyhow::anyhow!("No valid pragma found"))
}


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
    let pragma = extract_pragma(source_path)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    match pragma {
        Pragma::Exact(version) => {
            let exact_cache_dir = dirs::cache_dir()
                .unwrap_or_else(|| PathBuf::from(".cache"))
                .join("emacs-solidity-server/solc-exact");

            let mut filename = format!("solc-{}", version);
            if cfg!(windows) {
                filename.push_str(".exe");
            }

            let binary_path = exact_cache_dir.join(&filename);

            if binary_path.exists() {
                log_to_file(&format!("[solc-switch] Using exact cached solc: {}", version));
                return Ok(binary_path);
            }

            // Spawn thread to download
            let version_clone = version.clone();
            thread::spawn(move || {
                std::fs::create_dir_all(&exact_cache_dir).ok();

                let platform = get_platform_id();
                let list_url = format!(
                    "https://binaries.soliditylang.org/{}/list.json",
                    platform
                );
                let list_path = exact_cache_dir.join("list.json");

                loop {
                    // Download list.json if missing
                    if !list_path.exists() {
                        if let Err(e) = download_to_file(&list_url, &list_path) {
                            log_to_file(&format!("[solc-exact] Failed to download list.json: {:?}", e));
                            thread::sleep(Duration::from_secs(5));
                            continue;
                        }
                    }

                    let list = match SolcList::from_file(&list_path) {
                        Ok(l) => l,
                        Err(e) => {
                            log_to_file(&format!("[solc-exact] Failed to parse list.json: {:?}", e));
                            break;
                        }
                    };

                    let release_map = list.by_version();
                    if let Some(release) = release_map.get(&version_clone.to_string()) {
                        let binary_url = format!(
                            "https://binaries.soliditylang.org/{}/{}",
                            platform, release.path
                        );

                        log_to_file(&format!(
                            "[solc-exact] Downloading solc {} from {}",
                            version_clone, binary_url
                        ));

                        if let Err(e) = download_to_file(&binary_url, &binary_path) {
                            log_to_file(&format!("[solc-exact] Download failed: {:?}", e));
                            thread::sleep(Duration::from_secs(5));
                            continue;
                        }

                        if let Err(e) = verify_sha256(&binary_path, &release.sha256) {
                            log_to_file(&format!("[solc-exact] Checksum mismatch: {:?}", e));
                            let _ = std::fs::remove_file(&binary_path);
                            thread::sleep(Duration::from_secs(5));
                            continue;
                        }

                        let _ = make_executable(&binary_path);
                        log_to_file(&format!("[solc-exact] Download complete: solc-{}", version_clone));
                        break;
                    } else {
                        log_to_file(&format!(
                            "[solc-exact] Version {} not found in list.json",
                            version_clone
                        ));
                        break;
                    }
                }
            });

            log_to_file(&format!(
                "Exact version {} not cached — using system solc temporarily",
                version
            ));
            which("solc").map_err(|e| std::io::Error::new(std::io::ErrorKind::NotFound, e))
        }

        Pragma::Range(req) => {
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

            candidates.sort_by(|a, b| b.0.cmp(&a.0)); // latest first

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
    }
}
