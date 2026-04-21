use httpmock::Method::GET;
use httpmock::MockServer;
use minisign_verify::{PublicKey, Signature};
use release_hub::{EndpointSource, ReleaseSource, SourceRequest};
use url::Url;

#[tokio::test]
async fn endpoint_source_fetches_static_manifest() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/latest.json");
        then.status(200).body(
            r#"{
                "version": "1.2.3",
                "notes": "Stable",
                "pub_date": "2026-04-21T08:00:00Z",
                "platforms": {
                    "linux-x86_64": {
                        "url": "https://downloads.example/app.AppImage",
                        "signature": "sig-linux"
                    }
                }
            }"#,
        );
    });

    let source = EndpointSource::new(vec![Url::parse(&server.url("/latest.json")).unwrap()]);
    let release = source
        .fetch(&SourceRequest::new("linux-x86_64"))
        .await
        .unwrap();

    assert_eq!(release.signature("linux-x86_64").unwrap(), "sig-linux");
}

#[test]
fn verifier_accepts_known_good_minisign_fixture() {
    let public_key = PublicKey::decode(include_str!("fixtures/minisign/test.pub")).unwrap();
    let signature = Signature::decode(include_str!("fixtures/minisign/test.sig")).unwrap();

    public_key.verify(b"test", &signature, true).unwrap();
}
