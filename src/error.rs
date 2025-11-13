// Copyright (c) 2025 BibCiTeX Contributors
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// This file contains code derived from tauri-plugin-updater
// Original source: https://github.com/tauri-apps/plugins-workspace/tree/v2/plugins/updater
// Copyright (c) 2015 - Present - The Tauri Programme within The Commons Conservancy.
// Licensed under MIT OR MIT/Apache-2.0
//! Error types for the updater.
//!
//! The `Error` enum captures failures that can happen when checking, downloading
//! or installing updates across supported platforms.
/// All errors that can occur while running the updater.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// GitHub errors.
    #[error(transparent)]
    GitHub(#[from] octocrab::Error),
    /// IO errors.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Semver errors.
    #[error(transparent)]
    Semver(#[from] semver::Error),
    /// Unsupported app architecture.
    #[error(
        "Unsupported application architecture, expected one of `x86`, `x86_64`, `arm` or `aarch64`."
    )]
    UnsupportedArch,
    /// Operating system is not supported.
    #[error("Unsupported OS, expected one of `linux`, `darwin` or `windows`.")]
    UnsupportedOs,
    /// Asset not found
    #[error("Asset not found.")]
    AssetNotFound,
    /// Failed to determine updater package extract path
    #[error("Failed to determine updater package extract path.")]
    FailedToDetermineExtractPath,
    /// `reqwest` crate errors.
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    /// The platform was not found on the updater JSON response.
    #[error("the platform `{0}` was not found on the response `platforms` object")]
    TargetNotFound(String),
    /// Download failed
    #[error("`{0}`")]
    Network(String),
    /// Temp dir is not on same mount mount. This prevents our updater to rename the AppImage to a temp file.
    #[error("temp directory is not on the same mount point as the AppImage")]
    TempDirNotOnSameMountPoint,
    #[error("failed to create temporary directory")]
    TempDirNotFound,
    #[error("Authentication failed or was cancelled")]
    AuthenticationFailed,
    #[error("invalid updater binary format")]
    InvalidUpdaterFormat,
    /// Windows installer execution failed due to insufficient privileges
    #[error("Installation failed: insufficient privileges. Please run as administrator.")]
    InsufficientPrivileges,
    /// Windows installer execution failed due to file being in use
    #[error("Installation failed: file in use. Please close the application and try again.")]
    FileInUse,
    /// Windows installer execution failed
    #[error("Installation failed: installer execution error. Error code: {0}")]
    InstallerExecutionFailed(i32),
    /// User cancelled the UAC prompt
    #[error("Installation cancelled: User declined administrator privileges.")]
    UserCancelledElevation,
    #[error(transparent)]
    Http(#[from] http::Error),
    #[error(transparent)]
    InvalidHeaderValue(#[from] http::header::InvalidHeaderValue),
    #[error(transparent)]
    InvalidHeaderName(#[from] http::header::InvalidHeaderName),
    /// The configured updater endpoint must use a secure protocol like `https`
    #[error(transparent)]
    URLParseError(#[from] url::ParseError),
    /// Zip extraction errors.
    #[cfg(target_os = "macos")]
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
}

/// Convenient result alias for functions that may return [`Error`].
pub type Result<T> = std::result::Result<T, Error>;
