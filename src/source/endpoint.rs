use crate::{ReleaseSource, RemoteRelease, Result, SourceFuture, SourceRequest};
use url::Url;

#[derive(Debug, Clone)]
pub struct EndpointSource {
    endpoints: Vec<Url>,
}

impl EndpointSource {
    pub fn new(endpoints: Vec<Url>) -> Self {
        Self { endpoints }
    }

    pub(crate) async fn release_source_impl(
        &self,
        _request: &SourceRequest,
    ) -> Result<RemoteRelease> {
        let endpoint = self
            .endpoints
            .first()
            .cloned()
            .ok_or_else(|| crate::Error::Network("no endpoints configured".into()))?;
        let body = reqwest::get(endpoint)
            .await?
            .error_for_status()?
            .text()
            .await?;
        Ok(serde_json::from_str(&body)?)
    }
}

impl ReleaseSource for EndpointSource {
    fn fetch<'a>(&'a self, request: &'a SourceRequest) -> SourceFuture<'a> {
        Box::pin(async move { self.release_source_impl(request).await })
    }
}
