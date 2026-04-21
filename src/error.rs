#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("The configured updater endpoint must use a secure protocol like `https`.")]
    InsecureTransportProtocol,
    #[error("the platform `{0}` was not found on the response `platforms` object")]
    TargetNotFound(String),
    #[error("missing signature asset for `{0}`")]
    MissingSignatureAsset(String),
    #[error("invalid updater binary format")]
    InvalidUpdaterFormat,
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Time(#[from] time::error::Parse),
}

pub type Result<T> = std::result::Result<T, Error>;
