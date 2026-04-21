// Copyright (c) 2025 BibCiTeX Contributors
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// This file contains code derived from tauri-plugin-updater
// Original source: https://github.com/tauri-apps/plugins-workspace/tree/v2/plugins/updater
// Copyright (c) 2015 - Present - The Tauri Programme within The Commons Conservancy.
// Licensed under MIT OR MIT/Apache-2.0

use crate::{
    Config, EndpointSource, Error, InstallerKind, ReleaseSource, Result, SourceRequest, TargetInfo,
    Update, extract_path_from_executable,
};
use http::header::ACCEPT;
use http::{
    HeaderName,
    header::{HeaderMap, HeaderValue},
};
use reqwest::ClientBuilder;
use semver::Version;
use std::{
    env::current_exe,
    ffi::OsString,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use url::Url;

const UPDATER_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

pub type VersionComparator =
    Arc<dyn Fn(Version, crate::RemoteRelease) -> bool + Send + Sync + 'static>;

/// Configures and creates an [`Updater`].
pub struct UpdaterBuilder {
    app_name: String,
    current_version: Version,
    config: Config,
    target: Option<String>,
    source: Option<Box<dyn ReleaseSource>>,
    headers: HeaderMap,
    timeout: Option<Duration>,
    proxy: Option<Url>,
    no_proxy: bool,
    executable_path: Option<PathBuf>,
    installer_args: Vec<OsString>,
    version_comparator: Option<VersionComparator>,
}

impl UpdaterBuilder {
    pub fn new(app_name: &str, current_version: &str, config: Config) -> Self {
        Self {
            app_name: app_name.to_owned(),
            current_version: Version::parse(current_version).expect("valid semver"),
            config,
            target: None,
            source: None,
            headers: HeaderMap::new(),
            timeout: None,
            proxy: None,
            no_proxy: false,
            executable_path: None,
            installer_args: Vec::new(),
            version_comparator: None,
        }
    }

    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    pub fn source(mut self, source: Box<dyn ReleaseSource>) -> Self {
        self.source = Some(source);
        self
    }

    pub fn version_comparator<F>(mut self, comparator: F) -> Self
    where
        F: Fn(Version, crate::RemoteRelease) -> bool + Send + Sync + 'static,
    {
        self.version_comparator = Some(Arc::new(comparator));
        self
    }

    pub fn executable_path<P: AsRef<Path>>(mut self, p: P) -> Self {
        self.executable_path.replace(p.as_ref().into());
        self
    }

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

    pub fn headers(mut self, headers: HeaderMap) -> Self {
        self.headers = headers;
        self
    }

    pub fn clear_headers(mut self) -> Self {
        self.headers.clear();
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn proxy(mut self, proxy: Url) -> Self {
        self.proxy = Some(proxy);
        self
    }

    pub fn no_proxy(mut self) -> Self {
        self.no_proxy = true;
        self
    }

    pub fn installer_arg<S>(mut self, arg: S) -> Self
    where
        S: Into<OsString>,
    {
        self.installer_args.push(arg.into());
        self
    }

    pub fn installer_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.installer_args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn clear_installer_args(mut self) -> Self {
        self.installer_args.clear();
        self
    }

    pub fn build(self) -> Result<Updater> {
        self.config.validate()?;

        if self.source.is_none() && self.config.endpoints.is_empty() {
            return Err(Error::Network("no endpoints configured".into()));
        }

        let target = match self.target {
            Some(target) => target,
            None => TargetInfo::from_system(crate::SystemInfo::current()?).target,
        };
        let source = match self.source {
            Some(source) => Arc::<dyn ReleaseSource>::from(source),
            None => Arc::new(EndpointSource::new(self.config.endpoints.clone())),
        };

        let executable_path = self.executable_path.unwrap_or(current_exe()?);
        let extract_path = if cfg!(target_os = "linux") {
            executable_path
        } else {
            extract_path_from_executable(&executable_path)?
        };

        Ok(Updater {
            app_name: self.app_name,
            current_version: self.current_version,
            config: self.config,
            target,
            source,
            headers: self.headers,
            timeout: self.timeout,
            proxy: self.proxy,
            no_proxy: self.no_proxy,
            extract_path,
            installer_args: self.installer_args,
            version_comparator: self.version_comparator,
        })
    }
}

/// Updater instance capable of checking, downloading and installing updates.
pub struct Updater {
    pub app_name: String,
    pub current_version: Version,
    pub config: Config,
    pub target: String,
    source: Arc<dyn ReleaseSource>,
    pub headers: HeaderMap,
    pub timeout: Option<Duration>,
    pub proxy: Option<Url>,
    pub no_proxy: bool,
    pub extract_path: PathBuf,
    pub installer_args: Vec<OsString>,
    pub version_comparator: Option<VersionComparator>,
}

impl Updater {
    pub fn latest_version(&self) -> Option<Version> {
        None
    }

    pub async fn check(&self) -> Result<Option<Update>> {
        let request = SourceRequest::new(self.target.clone());
        let release = self.source.fetch(&request).await?;

        let has_update = if let Some(comparator) = &self.version_comparator {
            comparator(self.current_version.clone(), release.clone())
        } else {
            release.version > self.current_version
        };
        if !has_update {
            return Ok(None);
        }

        Ok(Some(Update {
            current_version: self.current_version.clone(),
            version: release.version.clone(),
            date: release.pub_date,
            body: release.notes.clone(),
            raw_json: serde_json::to_value(&release)?,
            download_url: release.download_url(&self.target)?.clone(),
            signature: release.signature(&self.target)?.clone(),
            pubkey: self.config.pubkey.clone(),
            target: self.target.clone(),
            installer_kind: InstallerKind::from_path(Path::new(
                release.download_url(&self.target)?.path(),
            ))?,
            headers: self.headers.clone(),
            timeout: self.timeout,
            proxy: self.proxy.clone(),
            no_proxy: self.no_proxy,
            dangerous_accept_invalid_certs: self.config.dangerous_accept_invalid_certs,
            dangerous_accept_invalid_hostnames: self.config.dangerous_accept_invalid_hostnames,
            extract_path: self.extract_path.clone(),
            app_name: self.app_name.clone(),
        }))
    }

    pub async fn update<C: FnMut(usize)>(&self, on_chunk: C) -> Result<bool> {
        if let Some(update) = self.check().await? {
            update.download_and_install(on_chunk).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Downloads the updater package and returns it as bytes.
    pub async fn download<C: FnMut(usize)>(&self, update: &Update, on_chunk: C) -> Result<Vec<u8>> {
        update.download(on_chunk).await
    }

    /// Installs the updater package downloaded by [`Updater::download`].
    pub fn install(&self, bytes: impl AsRef<[u8]>) -> Result<()> {
        self.install_inner(bytes.as_ref())
    }

    pub fn relaunch(&self) -> Result<()> {
        self.relaunch_inner()
    }

    pub async fn download_and_install<C: FnMut(usize)>(
        &self,
        update: &Update,
        on_chunk: C,
    ) -> Result<()> {
        update.download_and_install(on_chunk).await
    }
}

impl Update {
    pub async fn download<C>(&self, mut on_chunk: C) -> Result<Vec<u8>>
    where
        C: FnMut(usize),
    {
        let mut headers = self.headers.clone();
        if !headers.contains_key(ACCEPT) {
            headers.insert(ACCEPT, HeaderValue::from_static("application/octet-stream"));
        }

        let mut request = ClientBuilder::new().user_agent(UPDATER_USER_AGENT);
        if self.dangerous_accept_invalid_certs {
            request = request.danger_accept_invalid_certs(true);
        }
        if self.dangerous_accept_invalid_hostnames {
            request = request.danger_accept_invalid_hostnames(true);
        }
        if let Some(timeout) = self.timeout {
            request = request.timeout(timeout);
        }
        if self.no_proxy {
            request = request.no_proxy();
        } else if let Some(ref proxy) = self.proxy {
            let proxy = reqwest::Proxy::all(proxy.as_str())?;
            request = request.proxy(proxy);
        }

        let response = request
            .build()?
            .get(self.download_url.clone())
            .headers(headers)
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(Error::Network(format!(
                "Download request failed with status: {}",
                response.status()
            )));
        }

        let bytes = response.bytes().await?;
        on_chunk(bytes.len());
        crate::verify_minisign(&bytes, &self.pubkey, &self.signature)?;
        Ok(bytes.to_vec())
    }

    pub fn install(&self, bytes: &[u8]) -> Result<()> {
        match self.installer_kind {
            InstallerKind::AppTarGz | InstallerKind::AppZip => self.install_macos(bytes),
            InstallerKind::Msi | InstallerKind::Nsis => self.install_windows(bytes),
            InstallerKind::AppImage | InstallerKind::Deb | InstallerKind::Rpm => {
                self.install_linux(bytes)
            }
        }
    }

    pub async fn download_and_install<C>(&self, on_chunk: C) -> Result<()>
    where
        C: FnMut(usize),
    {
        let bytes = self.download(on_chunk).await?;
        self.install(&bytes)
    }
}

#[cfg(not(target_os = "macos"))]
impl Update {
    pub(crate) fn install_macos(&self, _bytes: &[u8]) -> Result<()> {
        Err(Error::UnsupportedOs)
    }
}

#[cfg(not(target_os = "windows"))]
impl Update {
    pub(crate) fn install_windows(&self, _bytes: &[u8]) -> Result<()> {
        Err(Error::UnsupportedOs)
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
impl Updater {
    pub(crate) fn install_inner(&self, _bytes: &[u8]) -> Result<()> {
        Err(Error::UnsupportedOs)
    }

    pub(crate) fn relaunch_inner(&self) -> Result<()> {
        Err(Error::UnsupportedOs)
    }
}
