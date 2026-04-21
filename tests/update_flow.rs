use httpmock::Method::GET;
use httpmock::MockServer;
use release_hub::{Config, EndpointSource, UpdaterBuilder};
use url::Url;

#[tokio::test]
async fn check_returns_update_when_remote_version_is_newer() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/latest.json");
        then.status(200).body(
            r#"{
                "version": "1.0.1",
                "notes": "Bug fixes",
                "pub_date": "2026-04-21T08:00:00Z",
                "platforms": {
                    "linux-x86_64": {
                        "url": "https://example.com/release-hub.AppImage",
                        "signature": "sig-linux"
                    }
                }
            }"#,
        );
    });

    let endpoint = Url::parse(&server.url("/latest.json")).unwrap();
    let config = Config {
        dangerous_insecure_transport_protocol: true,
        endpoints: vec![endpoint.clone()],
        pubkey: "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3".into(),
        ..Default::default()
    };

    let updater = UpdaterBuilder::new("ReleaseHub", "1.0.0", config)
        .target("linux-x86_64")
        .source(Box::new(EndpointSource::new(vec![endpoint])))
        .build()
        .unwrap();

    let update = updater.check().await.unwrap();
    assert!(update.is_some());
}
