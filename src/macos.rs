// Copyright (c) 2025 BibCiTeX Contributors
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// This file contains code derived from tauri-plugin-updater
// Original source: https://github.com/tauri-apps/plugins-workspace/tree/v2/plugins/updater
// Copyright (c) 2015 - Present - The Tauri Programme within The Commons Conservancy.
// Licensed under MIT OR MIT/Apache-2.0

// macOS installation and relaunch implementation.
//
// Handles extracting `.app.zip` bundles, atomically swapping the installed
// application, and elevating privileges through AppleScript when necessary.

use crate::{Error, Result, Updater};
use std::{
    io::Cursor,
    path::{Path, PathBuf},
};

impl Updater {
    /// Extract ZIP file for macOS .app bundles
    fn extract_zip(&self, bytes: &[u8]) -> Result<Vec<PathBuf>> {
        use std::os::unix::fs::PermissionsExt;
        use zip::ZipArchive;

        let cursor = Cursor::new(bytes);
        let mut archive = ZipArchive::new(cursor)?;
        let mut extracted_files = Vec::new();

        // Create temp directory for extraction
        let tmp_extract_dir = tempfile::Builder::new()
            .prefix("tauri_updated_app")
            .tempdir()?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = match file.enclosed_name() {
                Some(path) => tmp_extract_dir.path().join(path),
                None => continue,
            };

            if file.name().ends_with('/') {
                // Directory
                std::fs::create_dir_all(&outpath)?;
                // Set directory permissions if available
                if let Some(mode) = file.unix_mode() {
                    let permissions = std::fs::Permissions::from_mode(mode);
                    std::fs::set_permissions(&outpath, permissions)?;
                }
            } else {
                // File
                if let Some(p) = outpath.parent()
                    && !p.exists()
                {
                    std::fs::create_dir_all(p)?;
                }
                let mut outfile = std::fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;

                // Set file permissions if available, otherwise use default executable permissions for binaries
                if let Some(mode) = file.unix_mode() {
                    let permissions = std::fs::Permissions::from_mode(mode);
                    std::fs::set_permissions(&outpath, permissions)?;
                } else {
                    // If no permissions in ZIP, check if this is likely an executable file
                    let file_name = outpath.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    let path_str = outpath.to_string_lossy();

                    // Check if this is a binary executable in Contents/MacOS/
                    if path_str.contains("Contents/MacOS/")
                        || (!file_name.contains('.') && !path_str.contains("Contents/Resources/"))
                    {
                        // Set executable permissions (0o755 = rwxr-xr-x)
                        let permissions = std::fs::Permissions::from_mode(0o755);
                        std::fs::set_permissions(&outpath, permissions)?;
                    }
                }
            }
            extracted_files.push(outpath);
        }

        // For .app.zip files, we need to find the .app bundle and move it to the correct location
        let app_bundle = extracted_files
            .iter()
            .find(|path| path.extension().and_then(|s| s.to_str()) == Some("app"))
            .cloned();

        if let Some(app_path) = app_bundle {
            // Move the .app bundle to the target location
            self.move_app_bundle(&app_path)?;
        } else {
            // If no .app bundle found, try to move the entire extracted directory
            self.move_extracted_files(tmp_extract_dir.path())?;
        }

        Ok(extracted_files)
    }

    /// Move .app bundle to target location
    fn move_app_bundle(&self, app_path: &Path) -> Result<()> {
        // Create temp directory for backup
        let tmp_backup_dir = tempfile::Builder::new()
            .prefix("tauri_current_app")
            .tempdir()?;

        // Try to move the current app to backup
        let move_result = std::fs::rename(
            &self.extract_path,
            tmp_backup_dir.path().join("current_app"),
        );

        let need_authorization = if let Err(err) = move_result {
            if err.kind() == std::io::ErrorKind::PermissionDenied {
                true
            } else {
                return Err(err.into());
            }
        } else {
            false
        };

        if need_authorization {
            // Use AppleScript to perform atomic move with admin privileges
            // First create a backup, then move new app, then remove backup
            let backup_path = format!("{}.backup", self.extract_path.display());
            let apple_script = format!(
                "do shell script \"mv '{src}' '{backup}' && mv '{new}' '{src}' && rm -rf '{backup}'\" with administrator privileges",
                src = self.extract_path.display(),
                new = app_path.display(),
                backup = backup_path
            );

            let mut script =
                osakit::Script::new_from_source(osakit::Language::AppleScript, &apple_script);
            script.compile().expect("invalid AppleScript");
            let result = script.execute();

            if result.is_err() {
                // Try to restore from backup if it exists
                let restore_script = format!(
                    "do shell script \"if [ -d '{backup}' ]; then rm -rf '{src}' && mv '{backup}' '{src}'; fi\" with administrator privileges",
                    src = self.extract_path.display(),
                    backup = backup_path
                );
                let mut restore_script =
                    osakit::Script::new_from_source(osakit::Language::AppleScript, &restore_script);
                restore_script.compile().expect("invalid AppleScript");
                let _ = restore_script.execute(); // Best effort restore

                return Err(Error::Io(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "Failed to move the new app into place",
                )));
            }
        } else {
            // Atomic move: backup current app, move new app, remove backup
            let backup_path = self.extract_path.with_extension("backup");

            // Step 1: Move current app to backup (if it exists)
            if self.extract_path.exists() {
                std::fs::rename(&self.extract_path, &backup_path)?;
            }

            // Step 2: Move new app to target location
            let move_result = std::fs::rename(app_path, &self.extract_path);

            if let Err(err) = move_result {
                // If move failed, try to restore from backup
                if backup_path.exists() {
                    let _ = std::fs::rename(&backup_path, &self.extract_path); // Best effort restore
                }
                return Err(err.into());
            }

            // Step 3: Remove backup if everything succeeded
            if backup_path.exists() {
                let _ = std::fs::remove_dir_all(&backup_path); // Best effort cleanup
            }
        }

        Ok(())
    }

    /// Move extracted files to target location (fallback for non-.app bundles)
    fn move_extracted_files(&self, extract_dir: &Path) -> Result<()> {
        // Create temp directory for backup
        let tmp_backup_dir = tempfile::Builder::new()
            .prefix("tauri_current_app")
            .tempdir()?;

        // Try to move the current app to backup
        let move_result = std::fs::rename(
            &self.extract_path,
            tmp_backup_dir.path().join("current_app"),
        );

        let need_authorization = if let Err(err) = move_result {
            if err.kind() == std::io::ErrorKind::PermissionDenied {
                true
            } else {
                return Err(err.into());
            }
        } else {
            false
        };

        if need_authorization {
            // Use AppleScript to perform moves with admin privileges
            let apple_script = format!(
                "do shell script \"rm -rf '{src}' && mv -f '{new}' '{src}'\" with administrator privileges",
                src = self.extract_path.display(),
                new = extract_dir.display()
            );

            let mut script =
                osakit::Script::new_from_source(osakit::Language::AppleScript, &apple_script);
            script.compile().expect("invalid AppleScript");
            let result = script.execute();

            if result.is_err() {
                return Err(Error::Io(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "Failed to move the new app into place",
                )));
            }
        } else {
            // Remove existing directory if it exists
            if self.extract_path.exists() {
                std::fs::remove_dir_all(&self.extract_path)?;
            }
            // Move the new app to the target path
            std::fs::rename(extract_dir, &self.extract_path)?;
        }

        Ok(())
    }

    pub(crate) fn install_inner(&self, bytes: &[u8]) -> Result<()> {
        self.extract_zip(bytes)?;

        // Touch the app to update modification time
        let _ = std::process::Command::new("touch")
            .arg(&self.extract_path)
            .status()?;

        Ok(())
    }

    pub(crate) fn relaunch_inner(&self) -> Result<()> {
        // Use 'open' command to launch the updated app in the background
        // The -n flag opens a new instance even if one is already running
        // The -a flag specifies the application to open
        let _ = std::process::Command::new("open")
            .arg("-n")
            .arg(&self.extract_path)
            .spawn()?;
        std::process::exit(0);
    }
}
