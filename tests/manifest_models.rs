use release_hub::{Config, RemoteRelease, RemoteReleaseInner};
use semver::Version;
use url::Url;

#[test]
fn parses_static_manifest_release() {
    let json = r#"{
        "version": "1.2.3",
        "notes": "Bug fixes",
        "pub_date": "2026-04-21T08:00:00Z",
        "platforms": {
            "darwin-aarch64": {
                "url": "https://example.com/app.tar.gz",
                "signature": "sig-darwin"
            },
            "linux-x86_64": {
                "url": "https://example.com/app.AppImage",
                "signature": "sig-linux"
            }
        }
    }"#;

    let release: RemoteRelease = serde_json::from_str(json).unwrap();
    assert_eq!(release.version, Version::parse("1.2.3").unwrap());
    if let RemoteReleaseInner::Static { platforms } = &release.data {
        assert_eq!(platforms.len(), 2);
    } else {
        panic!("expected static manifest release");
    }
    assert_eq!(
        release.download_url("darwin-aarch64").unwrap(),
        &Url::parse("https://example.com/app.tar.gz").unwrap()
    );
    assert_eq!(release.signature("linux-x86_64").unwrap(), "sig-linux");
}

#[test]
fn rejects_http_endpoints_in_release_config() {
    let json = r#"{
        "endpoints": ["http://localhost:3000/latest.json"],
        "pubkey": "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3"
    }"#;

    let err = serde_json::from_str::<Config>(json).unwrap_err();
    assert!(err.to_string().contains("https"));
}
