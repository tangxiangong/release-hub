// Copyright (c) 2025 BibCiTeX Contributors
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// This file contains code derived from tauri-plugin-updater
// Original source: https://github.com/tauri-apps/plugins-workspace/tree/v2/plugins/updater
// Copyright (c) 2015 - Present - The Tauri Programme within The Commons Conservancy.
// Licensed under MIT OR MIT/Apache-2.0

//! Platform and system utilities used by the updater.
//!
//! Provides small types to model the current OS/architecture and helpers to
//! derive installation paths.

use crate::{Error, Result};
use std::path::{Path, PathBuf};

/// Supported operating systems.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OS {
    Macos,
    Windows,
}

impl std::fmt::Display for OS {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OS::Macos => write!(f, "macos"),
            OS::Windows => write!(f, "windows"),
        }
    }
}

/// Supported CPU architectures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arch {
    X86_64,
    Arm64,
}

impl std::fmt::Display for Arch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Arch::X86_64 => write!(f, "x86_64"),
            Arch::Arm64 => write!(f, "arm64"),
        }
    }
}

/// Detected local system information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemInfo {
    pub os: OS,
    pub arch: Arch,
}

impl SystemInfo {
    /// Get local system info.
    pub fn current() -> Result<Self> {
        let os = if cfg!(target_os = "macos") {
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

/// Bundle types supported by the installer logic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BundleType {
    MacOSAppZip,
    MacOSDMG,
    WindowsMSI,
    WindowsSetUp,
}

/// Derive the target extract/installation path from the current executable path.
///
/// On macOS, this transforms `/Applications/App.app/Contents/MacOS/App`
/// into `/Applications/App.app`.
pub fn extract_path_from_executable(executable_path: &Path) -> Result<PathBuf> {
    // Return the path of the current executable by default
    // Example C:\Program Files\My App\
    let extract_path = executable_path
        .parent()
        .map(PathBuf::from)
        .ok_or(Error::FailedToDetermineExtractPath)?;

    // MacOS example binary is in /Applications/TestApp.app/Contents/MacOS/myApp
    // We need to get /Applications/<app>.app
    // TODO(lemarier): Need a better way here
    // Maybe we could search for <*.app> to get the right path
    #[cfg(target_os = "macos")]
    if extract_path
        .display()
        .to_string()
        .contains("Contents/MacOS")
    {
        use std::path::PathBuf;

        return extract_path
            .parent()
            .map(PathBuf::from)
            .ok_or(Error::FailedToDetermineExtractPath)?
            .parent()
            .map(PathBuf::from)
            .ok_or(Error::FailedToDetermineExtractPath);
    }

    Ok(extract_path)
}
