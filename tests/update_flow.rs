use httpmock::Method::GET;
use httpmock::MockServer;
use release_hub::{Config, EndpointSource, InstallerKind, Update, UpdaterBuilder};
use semver::Version;
use url::Url;

fn test_config(endpoint: Url) -> Config {
    Config {
        dangerous_insecure_transport_protocol: true,
        endpoints: vec![endpoint],
        pubkey: "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3".into(),
        ..Default::default()
    }
}

fn test_update(download_url: Url, signature: &str) -> Update {
    Update {
        current_version: Version::parse("1.0.0").unwrap(),
        version: Version::parse("1.0.1").unwrap(),
        date: None,
        body: Some("Bug fixes".into()),
        raw_json: serde_json::json!({}),
        download_url,
        signature: signature.into(),
        pubkey: include_str!("fixtures/minisign/test.pub").into(),
        target: "linux-x86_64".into(),
        installer_kind: InstallerKind::AppImage,
    }
}

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
    let config = test_config(endpoint.clone());

    let updater = UpdaterBuilder::new("ReleaseHub", "1.0.0", config)
        .target("linux-x86_64")
        .source(Box::new(EndpointSource::new(vec![endpoint])))
        .build()
        .unwrap();

    let update = updater.check().await.unwrap();
    assert!(update.is_some());
}

#[tokio::test]
async fn check_uses_default_endpoint_source_from_config() {
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
    let updater = UpdaterBuilder::new("ReleaseHub", "1.0.0", test_config(endpoint))
        .target("linux-x86_64")
        .build()
        .unwrap();

    let update = updater.check().await.unwrap();
    assert!(update.is_some());
}

#[test]
fn build_fails_when_default_config_has_no_endpoints() {
    let config = Config {
        pubkey: "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3".into(),
        ..Default::default()
    };

    match UpdaterBuilder::new("ReleaseHub", "1.0.0", config)
        .target("linux-x86_64")
        .build()
    {
        Ok(_) => panic!("expected build to fail without default endpoints"),
        Err(release_hub::Error::Network(message)) => {
            assert_eq!(message, "no endpoints configured");
        }
        Err(err) => panic!("unexpected error: {err}"),
    }
}

#[tokio::test]
async fn update_download_verifies_minisign_payload() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/release-hub.AppImage");
        then.status(200).body("test");
    });

    let update = test_update(
        Url::parse(&server.url("/release-hub.AppImage")).unwrap(),
        include_str!("fixtures/minisign/test.sig"),
    );

    let mut chunks = Vec::new();
    let bytes = update.download(|chunk| chunks.push(chunk)).await.unwrap();

    assert_eq!(bytes, b"test");
    assert_eq!(chunks, vec![4]);
}

#[tokio::test]
async fn update_download_rejects_invalid_signature() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/release-hub.AppImage");
        then.status(200).body("test");
    });

    let err = test_update(
        Url::parse(&server.url("/release-hub.AppImage")).unwrap(),
        "invalid-signature",
    )
    .download(|_| {})
    .await
    .unwrap_err();

    assert!(matches!(err, release_hub::Error::Minisign(_)));
}
