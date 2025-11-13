// Copyright (c) 2025 BibCiTeX Contributors
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// This file contains code derived from tauri-plugin-updater
// Original source: https://github.com/tauri-apps/plugins-workspace/tree/v2/plugins/updater
// Copyright (c) 2015 - Present - The Tauri Programme within The Commons Conservancy.
// Licensed under MIT OR MIT/Apache-2.0

use crate::{
    Error, GitHubAsset, GitHubClient, GitHubRelease, Result, extract_path_from_executable,
};
use futures_util::StreamExt;
use http::{HeaderName, header::ACCEPT};
use reqwest::{
    ClientBuilder,
    header::{HeaderMap, HeaderValue},
};
use semver::Version;
use std::{
    env::current_exe,
    ffi::OsString,
    path::{Path, PathBuf},
    time::Duration,
};
use url::Url;

const UPDATER_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

// Builder and core updater logic.
//
// This module exposes the `UpdaterBuilder` used to configure the updater
// and the `Updater` type that performs release checks, downloads and
// installation on supported platforms.

/// Configures and creates an [`Updater`].
pub struct UpdaterBuilder {
    app_name: String,
    github_owner: String,
    github_repo: String,
    current_version: String,
    executable_path: Option<PathBuf>,
    headers: HeaderMap,
    timeout: Option<Duration>,
    proxy: Option<Url>,
    installer_args: Vec<OsString>,
    current_exe_args: Vec<OsString>,
}

impl UpdaterBuilder {
    /// Create a new builder.
    ///
    /// - `app_name`: Display name used in temp file prefixes and logs.
    /// - `current_version`: Your app's current semantic version.
    /// - `github_owner`/`github_repo`: Repository to query releases from.
    pub fn new(
        app_name: &str,
        current_version: &str,
        github_owner: &str,
        github_repo: &str,
    ) -> Self {
        Self {
            installer_args: Vec::new(),
            current_exe_args: Vec::new(),
            app_name: app_name.to_owned(),
            current_version: current_version.to_owned(),
            executable_path: None,
            github_owner: github_owner.to_owned(),
            github_repo: github_repo.to_owned(),
            headers: HeaderMap::new(),
            timeout: None,
            proxy: None,
        }
    }

    /// Override the executable path used to derive install/extract target.
    pub fn executable_path<P: AsRef<Path>>(mut self, p: P) -> Self {
        self.executable_path.replace(p.as_ref().into());
        self
    }

    /// Add a single HTTP header applied to the download request.
    pub fn header<K, V>(mut self, key: K, value: V) -> Result<Self>
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        let key: std::result::Result<HeaderName, http::Error> = key.try_into().map_err(Into::into);
        let value: std::result::Result<HeaderValue, http::Error> =
            value.try_into().map_err(Into::into);
        self.headers.insert(key?, value?);

        Ok(self)
    }

    /// Replace all headers with the provided map.
    pub fn headers(mut self, headers: HeaderMap) -> Self {
        self.headers = headers;
        self
    }

    /// Remove all configured headers.
    pub fn clear_headers(mut self) -> Self {
        self.headers.clear();
        self
    }

    /// Set a request timeout for downloads.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Route network requests through the given proxy.
    pub fn proxy(mut self, proxy: Url) -> Self {
        self.proxy.replace(proxy);
        self
    }

    /// Append a single argument to the platform installer invocation (if used).
    pub fn installer_arg<S>(mut self, arg: S) -> Self
    where
        S: Into<OsString>,
    {
        self.installer_args.push(arg.into());
        self
    }

    /// Append multiple installer arguments.
    pub fn installer_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.installer_args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Clear all installer arguments.
    pub fn clear_installer_args(mut self) -> Self {
        self.installer_args.clear();
        self
    }

    /// Finalize configuration and create an [`Updater`].
    pub fn build(self) -> Result<Updater> {
        let executable_path = self.executable_path.clone().unwrap_or(current_exe()?);

        // Get the extract_path from the provided executable_path
        let extract_path = if cfg!(target_os = "linux") {
            executable_path
        } else {
            extract_path_from_executable(&executable_path)?
        };

        let github_client = GitHubClient::new(&self.github_owner, &self.github_repo);

        let current_version = Version::parse(&self.current_version)?;

        Ok(Updater {
            app_name: self.app_name,
            current_version,
            proxy: self.proxy,
            installer_args: self.installer_args,
            current_exe_args: self.current_exe_args,
            headers: self.headers,
            timeout: self.timeout,
            extract_path,
            github_client,
            latest_release: None,
            proper_asset: None,
        })
    }
}

#[derive(Debug, Clone)]
/// Updater instance capable of checking, downloading and installing updates.
pub struct Updater {
    pub app_name: String,
    pub current_version: Version,
    pub proxy: Option<Url>,
    pub github_client: GitHubClient,
    pub headers: HeaderMap,
    pub extract_path: PathBuf,
    pub timeout: Option<Duration>,
    pub installer_args: Vec<OsString>,
    pub current_exe_args: Vec<OsString>,
    pub latest_release: Option<GitHubRelease>,
    pub proper_asset: Option<GitHubAsset>,
}

impl Updater {
    /// Fetch the latest GitHub release and convert it into a simplified structure.
    pub async fn latest_release(&self) -> Result<GitHubRelease> {
        self.github_client.get_latest_release().await?.try_into()
    }

    /// The version of the latest release if it has been previously cached on this instance.
    pub fn latest_version(&self) -> Option<Version> {
        self.latest_release
            .as_ref()
            .map(|release| release.version.clone())
    }

    /// The size in bytes of the asset selected for this platform, if already resolved.
    pub fn asset_size(&self) -> Option<u64> {
        self.proper_asset.as_ref().map(|asset| asset.size)
    }

    /// Resolve the proper asset for the current OS/arch.
    pub async fn proper_asset(&self) -> Result<GitHubAsset> {
        let release = self.latest_release().await?;
        release.find_proper_asset()
    }

    /// Check for a newer version. Returns `Ok(Some(Updater))` configured with the
    /// selected asset if an update is available, or `Ok(None)` if up-to-date.
    pub async fn check(&self) -> Result<Option<Updater>> {
        let latest_release = self.latest_release().await?;
        if latest_release.version > self.current_version {
            let asset = latest_release.find_proper_asset()?;
            Ok(Some(Self {
                latest_release: Some(latest_release),
                proper_asset: Some(asset),
                ..self.clone()
            }))
        } else {
            Ok(None)
        }
    }

    /// Check for updates and download/install if available.
    ///
    /// This is a convenience method that combines [`Updater::check()`] and [`Updater::download_and_install()`].
    /// Returns `Ok(true)` if an update was found and installed, `Ok(false)` if no update was needed.
    pub async fn update<C: FnMut(usize)>(
        &self,
        on_chunk: C,
        // on_download_finish: D,
    ) -> Result<bool> {
        if let Some(updater) = self.check().await? {
            updater.download_and_install(on_chunk).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl Updater {
    /// Downloads the updater package, verifies it then return it as bytes.
    ///
    /// Use [`Updater::install`] to install it
    pub async fn download<C: FnMut(usize)>(
        &self,
        mut on_chunk: C,
        // on_download_finish: D,
    ) -> Result<Vec<u8>> {
        // Fallback to reqwest if octocrab is not available
        let mut headers = self.headers.clone();
        if !headers.contains_key(ACCEPT) {
            headers.insert(ACCEPT, HeaderValue::from_static("application/octet-stream"));
        }

        let mut request = ClientBuilder::new().user_agent(UPDATER_USER_AGENT);
        if let Some(timeout) = self.timeout {
            request = request.timeout(timeout);
        }
        if let Some(ref proxy) = self.proxy {
            let proxy = reqwest::Proxy::all(proxy.as_str())?;
            request = request.proxy(proxy);
        }

        let download_url = self
            .proper_asset
            .clone()
            .ok_or(Error::AssetNotFound)?
            .browser_download_url
            .clone();

        let response = request
            .build()?
            .get(download_url)
            .headers(headers)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Network(format!(
                "Download request failed with status: {}",
                response.status()
            )));
        }

        let mut buffer = Vec::new();

        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            on_chunk(chunk.len());
            buffer.extend(chunk);
        }
        Ok(buffer)
    }

    /// Installs the updater package downloaded by [`Updater::download`]
    pub fn install(&self, bytes: impl AsRef<[u8]>) -> Result<()> {
        self.install_inner(bytes.as_ref())
    }

    pub fn relaunch(&self) -> Result<()> {
        self.relaunch_inner()
    }

    /// Downloads and installs the updater package
    pub async fn download_and_install<C: FnMut(usize)>(
        &self,
        on_chunk: C,
        // on_download_finish: D,
    ) -> Result<()> {
        let bytes = self.download(on_chunk).await?;
        self.install(bytes)
    }
}
