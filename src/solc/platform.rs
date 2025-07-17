use std::fmt;

/// Represents the supported operating systems for solc binaries.
#[derive(Debug, PartialEq, Eq)]
pub enum OS {
    Linux,
    MacOS,
    Windows,
}

/// Represents the supported architectures.
#[derive(Debug, PartialEq, Eq)]
pub enum Arch {
    Amd64,
    Aarch64,
}

/// Combined platform target (OS + Arch).
#[derive(Debug, PartialEq, Eq)]
pub struct Platform {
    pub os: OS,
    pub arch: Arch,
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let arch_str = match self.arch {
            Arch::Amd64 => "amd64",
            Arch::Aarch64 => "aarch64",
        };

        let os_str = match self.os {
            OS::Linux => "linux",
            OS::MacOS => "macosx",
            OS::Windows => "windows",
        };

        write!(f, "{}-{}", os_str, arch_str)
    }
}

impl Platform {
    /// Detects the current platform (OS and Arch).
    pub fn detect() -> Option<Self> {
        let os = match std::env::consts::OS {
            "linux" => OS::Linux,
            "macos" => OS::MacOS,
            "windows" => OS::Windows,
            _ => return None,
        };

        let arch = match std::env::consts::ARCH {
            "x86_64" => Arch::Amd64,
            "aarch64" => Arch::Aarch64,
            _ => return None,
        };

        Some(Platform { os, arch })
    }

    /// Returns the expected `solc` binary name for the platform.
    pub fn solc_binary_basename(&self, version: &str, build: &str) -> String {
        let platform = self.to_string();
        format!("solc-{}-v{}+{}", platform, version, build)
    }

    /// Returns the executable name, with `.exe` suffix on Windows.
    pub fn executable_name(&self, base: &str) -> String {
        match self.os {
            OS::Windows => format!("{}.exe", base),
            _ => base.to_string(),
        }
    }

    /// Returns just the Solidity platform ID (e.g., `linux-amd64`)
    pub fn id(&self) -> String {
        self.to_string()
    }
}

/// Helper to return current platform ID string like `linux-amd64`
pub fn get_platform_id() -> String {
    Platform::detect()
        .expect("Unsupported platform")
        .id()
}
