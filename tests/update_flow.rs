use release_hub::{Config, ReleaseSource, RemoteRelease, SourceRequest, UpdaterBuilder};
use url::Url;

struct StaticSource;

#[async_trait::async_trait]
impl ReleaseSource for StaticSource {
    async fn fetch(&self, _request: &SourceRequest) -> release_hub::Result<RemoteRelease> {
        Ok(serde_json::from_value(serde_json::json!({
            "version": "1.0.1",
            "notes": "Bug fixes",
            "pub_date": "2026-04-21T08:00:00Z",
            "platforms": {
                "linux-x86_64": {
                    "url": "https://example.com/release-hub.AppImage",
                    "signature": "sig-linux"
                }
            }
        }))?)
    }
}

#[tokio::test]
async fn check_returns_update_when_remote_version_is_newer() {
    let config = Config {
        endpoints: vec![Url::parse("https://example.com/latest.json").unwrap()],
        pubkey: "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3".into(),
        ..Default::default()
    };

    let updater = UpdaterBuilder::new("ReleaseHub", "1.0.0", config)
        .target("linux-x86_64")
        .source(Box::new(StaticSource))
        .build()
        .unwrap();

    let update = updater.check().await.unwrap();
    assert!(update.is_some());
}
