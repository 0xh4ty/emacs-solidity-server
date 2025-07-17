use semver::Version;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use anyhow::{self, Result};

#[derive(Debug, Deserialize)]
pub struct SolcRelease {
    pub path: String,
    pub version: String,
    pub build: String,

    #[serde(rename = "longVersion")]
    pub long_version: String,

    #[serde(default)]
    pub keccak256: String,

    #[serde(default)]
    pub sha256: String,

    #[serde(default)]
    pub urls: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SolcList {
    pub builds: Vec<SolcRelease>,

    #[serde(default)]
    pub releases: HashMap<String, String>, // version → path

    #[serde(default)]
    pub latest_release: Option<String>,
}

impl SolcList {
    /// Load list.json from a local file.
    pub fn from_file(path: &PathBuf) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        Ok(serde_json::from_reader(file)?)
    }

    /// Return latest patch release for each major.minor series (e.g., 0.8.x, 0.7.x)
    pub fn latest_per_minor(&self) -> HashMap<String, &SolcRelease> {
        let mut result: HashMap<String, &SolcRelease> = HashMap::new();

        for release in &self.builds {
            let parsed_version = match Version::parse(&release.version) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let key = format!("{}.{}", parsed_version.major, parsed_version.minor);

            let is_newer = match result.get(&key) {
                Some(&existing_release) => {
                    match Version::parse(&existing_release.version) {
                        Ok(existing_version) => parsed_version > existing_version,
                        Err(_) => true,
                    }
                }
                None => true,
            };

            if is_newer {
                result.insert(key, release);
            }
        }

        result
    }

    /// Map of all releases by version string
    pub fn by_version(&self) -> HashMap<String, &SolcRelease> {
        let mut map = HashMap::new();
        for release in &self.builds {
            map.insert(release.version.clone(), release);
        }
        map
    }
}
