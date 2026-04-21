use crate::{
    Error, InstallerKind, ReleaseManifestPlatform, ReleaseSource, RemoteRelease,
    RemoteReleaseInner, Result, SourceFuture, SourceRequest,
};
use octocrab::{
    Octocrab,
    models::repos::{Asset, Release},
};
use semver::Version;
use serde_json::json;
use std::{collections::HashMap, path::Path};
use time::OffsetDateTime;

#[derive(Debug, Clone)]
struct FixtureRelease {
    version: String,
    assets: Vec<FixtureAsset>,
}

impl ReleaseSource for GitHubSource {
    fn fetch<'a>(&'a self, request: &'a SourceRequest) -> SourceFuture<'a> {
        Box::pin(async move { self.release_source_impl(request).await })
    }
}

#[derive(Debug, Clone)]
struct FixtureAsset {
    name: String,
    value: String,
}

#[derive(Debug, Clone)]
enum SignatureSource<'a> {
    Download(&'a Asset),
    Fixture(&'a str),
}

#[derive(Debug, Clone)]
pub struct GitHubSource {
    client: octocrab::Octocrab,
    owner: String,
    repo: String,
    fixture_release: Option<FixtureRelease>,
}

impl GitHubSource {
    /// Creates a GitHub-backed release source for production use.
    pub fn new(owner: impl Into<String>, repo: impl Into<String>) -> Self {
        Self {
            client: Octocrab::default(),
            owner: owner.into(),
            repo: repo.into(),
            fixture_release: None,
        }
    }

    /// Builds a fixture-backed source for tests that need deterministic assets
    /// without hitting the GitHub API.
    ///
    /// This helper is intentionally test-oriented. Production code should use
    /// [`GitHubSource::new`] so signatures are fetched from the real paired
    /// release asset.
    #[doc(hidden)]
    pub fn from_assets(
        owner: impl Into<String>,
        repo: impl Into<String>,
        version: &str,
        assets: Vec<(&str, &str)>,
    ) -> Self {
        Self {
            client: Octocrab::default(),
            owner: owner.into(),
            repo: repo.into(),
            fixture_release: Some(FixtureRelease {
                version: version.into(),
                assets: assets
                    .into_iter()
                    .map(|(name, value)| FixtureAsset {
                        name: name.into(),
                        value: value.into(),
                    })
                    .collect(),
            }),
        }
    }

    pub(crate) async fn release_source_impl(
        &self,
        request: &SourceRequest,
    ) -> Result<RemoteRelease> {
        if let Some(fixture_release) = &self.fixture_release {
            let asset = select_fixture_target_asset(&fixture_release.assets, &request.target)?;
            let signature_asset =
                find_fixture_signature_asset(&fixture_release.assets, &asset.name)
                    .ok_or_else(|| Error::MissingSignatureAsset(asset.name.clone()))?;
            let download_asset = fixture_download_asset(asset, 1);

            return build_remote_release_from_assets(
                &request.target,
                &fixture_release.version,
                None,
                None,
                &download_asset,
                SignatureSource::Fixture(&signature_asset.value),
            )
            .await;
        }

        let release = self
            .client
            .repos(&self.owner, &self.repo)
            .releases()
            .get_latest()
            .await?;
        let pub_date = parse_pub_date(&release)?;
        let asset = select_target_asset(&release.assets, &request.target)?;
        let signature_asset = find_signature_asset(&release.assets, &asset.name)
            .ok_or_else(|| Error::MissingSignatureAsset(asset.name.clone()))?;

        build_remote_release_from_assets(
            &request.target,
            &release.tag_name,
            release.body.clone(),
            pub_date,
            asset,
            SignatureSource::Download(signature_asset),
        )
        .await
    }
}

fn fixture_asset(id: u64, name: &str, url: &str) -> Asset {
    serde_json::from_value(json!({
        "url": format!("https://api.github.com/assets/{id}"),
        "browser_download_url": url,
        "id": id,
        "node_id": format!("asset-{id}"),
        "name": name,
        "label": null,
        "state": "uploaded",
        "content_type": "application/octet-stream",
        "size": 1,
        "digest": null,
        "download_count": 0,
        "created_at": "2026-04-21T00:00:00Z",
        "updated_at": "2026-04-21T00:00:00Z",
        "uploader": null
    }))
    .expect("fixture asset should deserialize")
}

fn fixture_download_asset(asset: &FixtureAsset, id: u64) -> Asset {
    fixture_asset(id, &asset.name, &asset.value)
}

fn is_signature_asset(name: &str) -> bool {
    name.ends_with(".sig") || name.ends_with(".minisig")
}

fn target_variants(target: &str) -> [String; 3] {
    [
        target.to_ascii_lowercase(),
        target.replace('-', "_").to_ascii_lowercase(),
        target.replace('_', "-").to_ascii_lowercase(),
    ]
}

fn select_target_asset<'a>(assets: &'a [Asset], target: &str) -> Result<&'a Asset> {
    let variants = target_variants(target);
    assets
        .iter()
        .filter(|asset| !is_signature_asset(&asset.name))
        .find(|asset| {
            let name = asset.name.to_ascii_lowercase();
            variants.iter().any(|variant| name.contains(variant))
                && InstallerKind::from_path(Path::new(&asset.name)).is_ok()
        })
        .ok_or_else(|| Error::TargetNotFound(target.into()))
}

fn select_fixture_target_asset<'a>(
    assets: &'a [FixtureAsset],
    target: &str,
) -> Result<&'a FixtureAsset> {
    let variants = target_variants(target);
    assets
        .iter()
        .filter(|asset| !is_signature_asset(&asset.name))
        .find(|asset| {
            let name = asset.name.to_ascii_lowercase();
            variants.iter().any(|variant| name.contains(variant))
                && InstallerKind::from_path(Path::new(&asset.name)).is_ok()
        })
        .ok_or_else(|| Error::TargetNotFound(target.into()))
}

fn find_signature_asset<'a>(assets: &'a [Asset], name: &str) -> Option<&'a Asset> {
    let sig_name = format!("{name}.sig");
    let minisig_name = format!("{name}.minisig");
    assets
        .iter()
        .find(|asset| asset.name == sig_name || asset.name == minisig_name)
}

fn find_fixture_signature_asset<'a>(
    assets: &'a [FixtureAsset],
    name: &str,
) -> Option<&'a FixtureAsset> {
    let sig_name = format!("{name}.sig");
    let minisig_name = format!("{name}.minisig");
    assets
        .iter()
        .find(|asset| asset.name == sig_name || asset.name == minisig_name)
}

fn parse_release_version(version: &str) -> Result<Version> {
    Version::parse(version.trim_start_matches('v')).map_err(Error::Semver)
}

fn parse_pub_date(release: &Release) -> Result<Option<OffsetDateTime>> {
    release
        .published_at
        .as_ref()
        .map(|published_at| {
            OffsetDateTime::parse(
                &published_at.to_rfc3339(),
                &time::format_description::well_known::Rfc3339,
            )
            .map_err(Error::Time)
        })
        .transpose()
}

async fn load_signature(source: SignatureSource<'_>) -> Result<String> {
    match source {
        SignatureSource::Download(signature_asset) => {
            Ok(reqwest::get(signature_asset.browser_download_url.clone())
                .await?
                .error_for_status()?
                .text()
                .await?)
        }
        SignatureSource::Fixture(signature) => Ok(signature.to_string()),
    }
}

async fn build_remote_release_from_assets(
    target: &str,
    version: &str,
    notes: Option<String>,
    pub_date: Option<OffsetDateTime>,
    asset: &Asset,
    signature_source: SignatureSource<'_>,
) -> Result<RemoteRelease> {
    let signature = load_signature(signature_source).await?;
    let platforms = HashMap::from([(
        target.to_string(),
        ReleaseManifestPlatform {
            url: asset.browser_download_url.clone(),
            signature,
        },
    )]);

    Ok(RemoteRelease {
        version: parse_release_version(version)?,
        notes,
        pub_date,
        data: RemoteReleaseInner::Static { platforms },
    })
}
