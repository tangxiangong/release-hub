use crate::{RemoteRelease, Result, SourceRequest};
use url::Url;

pub struct EndpointSource {
    endpoints: Vec<Url>,
}

impl EndpointSource {
    pub fn new(endpoints: Vec<Url>) -> Self {
        Self { endpoints }
    }
}

#[async_trait::async_trait]
impl crate::ReleaseSource for EndpointSource {
    async fn fetch(&self, _request: &SourceRequest) -> Result<RemoteRelease> {
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
