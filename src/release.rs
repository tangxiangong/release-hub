use semver::Version;
use serde::{Deserialize, Deserializer, Serialize, de::Error as DeError};
use std::collections::HashMap;
use time::OffsetDateTime;
use url::Url;

use crate::InstallerKind;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ReleaseManifestPlatform {
    pub url: Url,
    pub signature: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum RemoteReleaseInner {
    Dynamic(ReleaseManifestPlatform),
    Static {
        platforms: HashMap<String, ReleaseManifestPlatform>,
    },
}

#[derive(Debug, Serialize, Clone)]
pub struct RemoteRelease {
    pub version: Version,
    pub notes: Option<String>,
    pub pub_date: Option<OffsetDateTime>,
    #[serde(flatten)]
    pub data: RemoteReleaseInner,
}

impl<'de> Deserialize<'de> for RemoteRelease {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct InnerRemoteRelease {
            #[serde(alias = "name")]
            version: Version,
            notes: Option<String>,
            pub_date: Option<String>,
            platforms: Option<HashMap<String, ReleaseManifestPlatform>>,
            url: Option<Url>,
            signature: Option<String>,
        }

        let release = InnerRemoteRelease::deserialize(deserializer)?;
        let pub_date = match release.pub_date {
            Some(date) => Some(
                OffsetDateTime::parse(&date, &time::format_description::well_known::Rfc3339)
                    .map_err(|error| {
                        DeError::custom(format!("invalid value for `pub_date`: {error}"))
                    })?,
            ),
            None => None,
        };

        let data = match release.platforms {
            Some(platforms) => RemoteReleaseInner::Static { platforms },
            None => RemoteReleaseInner::Dynamic(ReleaseManifestPlatform {
                url: release.url.ok_or_else(|| {
                    DeError::custom("the `url` field was not set on the updater response")
                })?,
                signature: release.signature.ok_or_else(|| {
                    DeError::custom("the `signature` field was not set on the updater response")
                })?,
            }),
        };

        Ok(Self {
            version: release.version,
            notes: release.notes,
            pub_date,
            data,
        })
    }
}

impl RemoteRelease {
    pub fn download_url(&self, target: &str) -> crate::Result<&Url> {
        match &self.data {
            RemoteReleaseInner::Dynamic(platform) => Ok(&platform.url),
            RemoteReleaseInner::Static { platforms } => platforms
                .get(target)
                .map(|platform| &platform.url)
                .ok_or_else(|| crate::Error::TargetNotFound(target.into())),
        }
    }

    pub fn signature(&self, target: &str) -> crate::Result<&String> {
        match &self.data {
            RemoteReleaseInner::Dynamic(platform) => Ok(&platform.signature),
            RemoteReleaseInner::Static { platforms } => platforms
                .get(target)
                .map(|platform| &platform.signature)
                .ok_or_else(|| crate::Error::TargetNotFound(target.into())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Update {
    pub current_version: Version,
    pub version: Version,
    pub date: Option<OffsetDateTime>,
    pub body: Option<String>,
    pub raw_json: serde_json::Value,
    pub download_url: Url,
    pub signature: String,
    pub pubkey: String,
    pub target: String,
    pub installer_kind: InstallerKind,
}
