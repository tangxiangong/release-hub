pub mod endpoint;
pub mod github;

use crate::RemoteRelease;
use std::{future::Future, pin::Pin};

#[derive(Debug, Clone)]
pub struct SourceRequest {
    pub target: String,
}

impl SourceRequest {
    pub fn new(target: impl Into<String>) -> Self {
        Self {
            target: target.into(),
        }
    }
}

pub type SourceFuture<'a> = Pin<Box<dyn Future<Output = crate::Result<RemoteRelease>> + Send + 'a>>;

pub trait ReleaseSource: Send + Sync {
    fn fetch<'a>(&'a self, request: &'a SourceRequest) -> SourceFuture<'a>;
}

pub use endpoint::EndpointSource;
pub use github::GitHubSource;
