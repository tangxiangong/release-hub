use crate::{Arch, BundleType, Error, OS, Result, SystemInfo};
// GitHub release querying and asset selection utilities.
//
// This module wraps `octocrab` to fetch releases and provides a simplified
// representation (`GitHubRelease`, `GitHubAsset`) plus helpers to select the
// proper asset for the current platform.
use octocrab::{
    Octocrab,
    models::repos::{Asset, Release},
};
use semver::Version;
use url::Url;

/// Minimal GitHub API client configured for a single repository.
#[derive(Debug, Clone)]
pub struct GitHubClient {
    pub octocrab: Octocrab,
    pub owner: String,
    pub repo: String,
}

/// A single downloadable artifact from a GitHub release.
#[derive(Debug, Clone)]
pub struct GitHubAsset {
    pub name: String,
    pub os: OS,
    pub arch: Arch,
    pub browser_download_url: Url,
    pub size: u64,
    pub bundle_type: BundleType,
}

/// Simplified GitHub release information used by the updater.
#[derive(Debug, Clone)]
pub struct GitHubRelease {
    /// Version to install.
    pub version: Version,
    /// Release name.
    pub name: Option<String>,
    /// Release notes.
    pub note: Option<String>,
    /// Release date.
    pub published_at: Option<String>,
    /// Assets.
    pub assets: Vec<GitHubAsset>,
}

impl TryFrom<Release> for GitHubRelease {
    type Error = Error;

    fn try_from(release: Release) -> Result<Self> {
        let version =
            Version::parse(release.tag_name.trim_start_matches('v')).map_err(Error::Semver)?;

        let assets = get_assets(release.assets)?;
        Ok(GitHubRelease {
            version,
            name: release.name,
            note: release.body,
            published_at: release.published_at.map(|dt| dt.to_rfc3339()),
            assets,
        })
    }
}

impl GitHubClient {
    /// Create a new GitHub client for `owner/repo`.
    pub fn new(owner: &str, repo: &str) -> Self {
        let octocrab = Octocrab::default();
        Self {
            octocrab,
            owner: owner.to_owned(),
            repo: repo.to_owned(),
        }
    }

    /// Get the latest GitHub release for the configured repository.
    pub async fn get_latest_release(&self) -> Result<Release> {
        Ok(self
            .octocrab
            .repos(&self.owner, &self.repo)
            .releases()
            .get_latest()
            .await?)
    }
}

pub fn find_proper_asset(release: &GitHubRelease) -> Result<GitHubAsset> {
    release.find_proper_asset()
}

impl GitHubRelease {
    /// Find the appropriate asset for the local OS/arch.
    pub fn find_proper_asset(&self) -> Result<GitHubAsset> {
        let system_info = SystemInfo::current()?;
        let result = {
            #[cfg(target_os = "windows")]
            {
                self.assets
                    .iter()
                    .find(|asset| {
                        asset.os == system_info.os
                            && asset.arch == system_info.arch
                            && asset.bundle_type == BundleType::WindowsSetUp
                    })
                    .cloned()
                    .ok_or(Error::AssetNotFound)?
            }
            #[cfg(target_os = "macos")]
            {
                self.assets
                    .iter()
                    .find(|asset| {
                        asset.os == system_info.os
                            && asset.arch == system_info.arch
                            && asset.bundle_type == BundleType::MacOSAppZip
                    })
                    .cloned()
                    .ok_or(Error::AssetNotFound)?
            }
        };
        Ok(result)
    }
    /// The release's download URL for the asset matched to this platform.
    pub fn download_url(&self) -> Result<Url> {
        let asset = self.find_proper_asset()?;
        Ok(asset.browser_download_url)
    }
}

fn get_assets(assets: Vec<Asset>) -> Result<Vec<GitHubAsset>> {
    assets
        .into_iter()
        .map(|asset| {
            let name = asset.name.to_lowercase();
            let os = if name.contains("macos") || name.contains("darwin") || name.contains("osx") {
                OS::Macos
            } else if name.contains("windows") || name.contains("win") {
                OS::Windows
            } else {
                return Err(Error::TargetNotFound("macos or windows".into()));
            };
            let arch = if name.contains("x86_64") || name.contains("amd64") {
                Arch::X86_64
            } else if name.contains("aarch64") || name.contains("arm64") {
                Arch::Arm64
            } else {
                return Err(Error::TargetNotFound("x86_64 or amd64".into()));
            };
            let bundle_type = if name.ends_with(".dmg") {
                BundleType::MacOSDMG
            } else if name.ends_with(".app.zip") {
                BundleType::MacOSAppZip
            } else if name.ends_with(".msi") {
                BundleType::WindowsMSI
            } else if name.ends_with(".exe") {
                BundleType::WindowsSetUp
            } else {
                return Err(Error::TargetNotFound("os-arch".into()));
            };
            Ok(GitHubAsset {
                name,
                browser_download_url: asset.browser_download_url,
                size: asset.size as u64,
                os,
                arch,
                bundle_type,
            })
        })
        .collect::<Result<Vec<_>>>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_assets() {
        let client = GitHubClient::new("tangxiangong", "bibcitex");
        let release: GitHubRelease = client
            .get_latest_release()
            .await
            .unwrap()
            .try_into()
            .unwrap();
        println!("{:?}", release.assets);
        println!("{:?}", release.find_proper_asset());
    }
}
