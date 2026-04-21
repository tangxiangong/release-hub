/// Endpoint-backed release source implementation.
pub mod endpoint;
/// GitHub Release-backed source implementation.
pub mod github;

use crate::RemoteRelease;
use std::{future::Future, pin::Pin};

/// Parameters supplied to a release source when resolving update metadata.
#[derive(Debug, Clone)]
pub struct SourceRequest {
    /// Requested platform target such as `linux-x86_64`.
    pub target: String,
}

impl SourceRequest {
    /// Creates a new source request for the given target.
    pub fn new(target: impl Into<String>) -> Self {
        Self {
            target: target.into(),
        }
    }
}

/// Boxed future returned by [`ReleaseSource::fetch`].
pub type SourceFuture<'a> = Pin<Box<dyn Future<Output = crate::Result<RemoteRelease>> + Send + 'a>>;

/// Pluggable source of release metadata for the updater pipeline.
pub trait ReleaseSource: Send + Sync {
    /// Fetches release metadata for the requested target.
    fn fetch<'a>(&'a self, request: &'a SourceRequest) -> SourceFuture<'a>;
}

pub use endpoint::EndpointSource;
pub use github::GitHubSource;
