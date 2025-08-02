use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use anyhow::{Result, Context};

use crate::solc::versions::{SolcList, SolcRelease};
use crate::solc::fetch::{download_to_file, verify_sha256};
use crate::solc::platform::get_platform_id;
use crate::util::log::log_to_file;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub struct SolcManager {
    pub cache_dir: PathBuf,
    pub list: SolcList,
}

impl SolcManager {
    pub fn new(cache_dir: PathBuf, list: SolcList) -> Self {
        fs::create_dir_all(&cache_dir).ok(); // ensure exists
        Self { cache_dir, list }
    }

    pub fn ensure_latest_versions(&self) -> Result<()> {
        let latest_versions = self.list.latest_per_minor();

        let releases: Vec<_> = latest_versions.values().cloned().collect();

        for release in &releases {
            self.ensure_release_cached(release)?;
        }

        self.clean_old_versions(&latest_versions)?;
        Ok(())
    }

    pub fn clean_unused_exact_versions(&self) -> Result<()> {
        let exact_cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("emacs-solidity-server/solc-exact");

        if !exact_cache_dir.exists() {
            return Ok(()); // nothing to clean
        }

        let now = std::time::SystemTime::now();
        let retention_period = std::time::Duration::from_secs(30 * 24 * 60 * 60); // 30 days

        for entry in fs::read_dir(&exact_cache_dir)? {
            let entry = entry?;
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            let metadata = fs::metadata(&path)?;
            let modified = metadata.modified().or_else(|_| metadata.accessed())?;

            if now.duration_since(modified).unwrap_or_default() > retention_period {
                let _ = fs::remove_file(&path);
                log_to_file(&format!(
                    "[solc-prune] Removed unused exact binary: {}",
                    path.display()
                ));
            }
        }

        Ok(())
    }

    pub fn get_binary_path(&self, version: &str) -> Option<PathBuf> {
        let path = self.cache_dir.join(format!("solc-{}", version));
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    pub fn ensure_release_cached(&self, release: &SolcRelease) -> Result<()> {
        let mut filename = format!("solc-{}", release.version);
        if cfg!(windows) {
            filename.push_str(".exe");
        }

        let dest_path = self.cache_dir.join(&filename);

        if dest_path.exists() {
            verify_sha256(&dest_path, &release.sha256)
                .with_context(|| format!("Verifying {:?}", dest_path))?;
            return Ok(()); // already downloaded and verified
        }

        let platform = get_platform_id();
        let download_url = format!(
            "https://binaries.soliditylang.org/{}/{}",
            platform, release.path
        );

        log_to_file(&format!("Downloading {} → {}", release.version, download_url));

        loop {
            match download_to_file(&download_url, &dest_path) {
                Ok(_) => {
                    match verify_sha256(&dest_path, &release.sha256) {
                        Ok(_) => {
                            make_executable(&dest_path)?;
                            log_to_file(&format!(
                                "[solc-sync] Downloaded and verified {}",
                                filename
                            ));
                            return Ok(());
                        }
                        Err(e) => {
                            log_to_file(&format!(
                                "[solc-sync] Checksum mismatch for {}: {:?}",
                                filename, e
                            ));
                            let _ = std::fs::remove_file(&dest_path);
                        }
                    }
                }
                Err(e) => {
                    log_to_file(&format!(
                        "[solc-sync] Failed to download {}: {:?}",
                        filename, e
                    ));
                }
            }

            thread::sleep(Duration::from_secs(5));
        }
    }

    fn clean_old_versions(&self, latest: &HashMap<String, &SolcRelease>) -> Result<()> {
        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let name = entry.file_name().into_string().unwrap_or_default();

            if let Some(ver) = name.strip_prefix("solc-") {
                let keep = latest.values().any(|r| r.version == ver);
                if !keep {
                    let _ = fs::remove_file(entry.path());
                    log_to_file(&format!(
                        "[solc-sync] Removed old version: solc-{}",
                        ver
                    ));
                }
            }
        }
        Ok(())
    }
}

pub fn make_executable(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let mut perms = std::fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms)?;
    }

    #[cfg(windows)]
    {
        // No-op: Windows doesn't require +x permissions
        // But you may optionally check extension
    }

    Ok(())
}
