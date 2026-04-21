use release_hub::source::github::GitHubSource as ModuleGitHubSource;
use release_hub::{ReleaseSource, SourceRequest};

#[tokio::test]
async fn github_source_module_path_pairs_asset_with_signature() {
    let source = ModuleGitHubSource::from_assets(
        "owner",
        "repo",
        "1.2.3",
        vec![
            (
                "app-linux-x86_64.AppImage",
                "https://example.com/app.AppImage",
            ),
            (
                "app-linux-x86_64.AppImage.sig",
                "https://example.com/app.AppImage.sig",
            ),
        ],
    );

    let release = source
        .fetch(&SourceRequest::new("linux-x86_64"))
        .await
        .unwrap();
    assert_eq!(
        release.download_url("linux-x86_64").unwrap().as_str(),
        "https://example.com/app.AppImage"
    );
}

#[tokio::test]
async fn github_source_requires_matching_signature_asset() {
    let source = release_hub::source::GitHubSource::from_assets(
        "owner",
        "repo",
        "1.2.3",
        vec![(
            "app-linux-x86_64.AppImage",
            "https://example.com/app.AppImage",
        )],
    );

    let err = source
        .fetch(&SourceRequest::new("linux-x86_64"))
        .await
        .unwrap_err();

    assert!(
        matches!(err, release_hub::Error::MissingSignatureAsset(name) if name == "app-linux-x86_64.AppImage")
    );
}
