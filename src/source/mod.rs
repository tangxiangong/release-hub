pub mod endpoint;

pub mod github {
    pub use crate::github::*;
}

use crate::RemoteRelease;

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

#[async_trait::async_trait]
pub trait ReleaseSource: Send + Sync {
    async fn fetch(&self, request: &SourceRequest) -> crate::Result<RemoteRelease>;
}

pub use endpoint::EndpointSource;
