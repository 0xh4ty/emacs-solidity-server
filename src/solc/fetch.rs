use std::fs::File;
use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};
use anyhow::{Result, anyhow};
use reqwest::blocking::Client;

pub fn download_to_file(url: &str, dest: &Path) -> Result<()> {
    let client = Client::new();
    let mut resp = client.get(url).send()?.error_for_status()?;
    let mut file = File::create(dest)?;
    resp.copy_to(&mut file)?;
    Ok(())
}

pub fn verify_sha256(path: &Path, expected: &str) -> Result<()> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    let result = format!("0x{:x}", hasher.finalize());
    if result != expected {
        return Err(anyhow!("Checksum mismatch for {:?}: expected {}, got {}", path, expected, result));
    }
    Ok(())
}
