use crate::{Error, Result};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OS {
    Linux,
    Macos,
    Windows,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arch {
    X86_64,
    Arm64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallerKind {
    AppImage,
    Deb,
    Rpm,
    AppTarGz,
    AppZip,
    Msi,
    Nsis,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemInfo {
    pub os: OS,
    pub arch: Arch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetInfo {
    pub target: String,
    pub system: SystemInfo,
}

impl TargetInfo {
    pub fn from_system(system: SystemInfo) -> Self {
        let os = match system.os {
            OS::Linux => "linux",
            OS::Macos => "darwin",
            OS::Windows => "windows",
        };
        let arch = match system.arch {
            Arch::X86_64 => "x86_64",
            Arch::Arm64 => "aarch64",
        };
        Self {
            target: format!("{os}-{arch}"),
            system,
        }
    }
}

impl SystemInfo {
    pub fn current() -> Result<Self> {
        let os = if cfg!(target_os = "linux") {
            OS::Linux
        } else if cfg!(target_os = "macos") {
            OS::Macos
        } else if cfg!(target_os = "windows") {
            OS::Windows
        } else {
            return Err(Error::UnsupportedOs);
        };
        let arch = if cfg!(target_arch = "x86_64") {
            Arch::X86_64
        } else if cfg!(target_arch = "aarch64") {
            Arch::Arm64
        } else {
            return Err(Error::UnsupportedArch);
        };
        Ok(Self { os, arch })
    }
}

impl InstallerKind {
    pub fn from_path(path: &Path) -> Result<Self> {
        let name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
        if name.ends_with(".AppImage") {
            Ok(Self::AppImage)
        } else if name.ends_with(".deb") {
            Ok(Self::Deb)
        } else if name.ends_with(".rpm") {
            Ok(Self::Rpm)
        } else if name.ends_with(".app.tar.gz") {
            Ok(Self::AppTarGz)
        } else if name.ends_with(".app.zip") {
            Ok(Self::AppZip)
        } else if name.ends_with(".msi") {
            Ok(Self::Msi)
        } else if name.ends_with(".exe") {
            Ok(Self::Nsis)
        } else {
            Err(Error::InvalidUpdaterFormat)
        }
    }
}
