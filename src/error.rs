#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    GitHub(#[from] octocrab::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Semver(#[from] semver::Error),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Minisign(#[from] minisign_verify::Error),
    #[error(transparent)]
    Http(#[from] http::Error),
    #[error(transparent)]
    InvalidHeaderValue(#[from] http::header::InvalidHeaderValue),
    #[error(transparent)]
    InvalidHeaderName(#[from] http::header::InvalidHeaderName),
    #[error(
        "Unsupported application architecture, expected one of `x86`, `x86_64`, `arm` or `aarch64`."
    )]
    UnsupportedArch,
    #[error("Unsupported OS, expected one of `linux`, `darwin` or `windows`.")]
    UnsupportedOs,
    #[error("Asset not found.")]
    AssetNotFound,
    #[error("Failed to determine updater package extract path.")]
    FailedToDetermineExtractPath,
    #[error("The configured updater endpoint must use a secure protocol like `https`.")]
    InsecureTransportProtocol,
    #[error("the platform `{0}` was not found on the response `platforms` object")]
    TargetNotFound(String),
    #[error("missing signature asset for `{0}`")]
    MissingSignatureAsset(String),
    #[error("`{0}`")]
    Network(String),
    #[error("invalid updater binary format")]
    InvalidUpdaterFormat,
    #[error("failed to create temporary directory")]
    TempDirNotFound,
    #[error("Installation failed: insufficient privileges. Please run as administrator.")]
    InsufficientPrivileges,
    #[error("Installation failed: file in use. Please close the application and try again.")]
    FileInUse,
    #[error("Installation failed: installer execution error. Error code: {0}")]
    InstallerExecutionFailed(i32),
    #[error("Installation cancelled: User declined administrator privileges.")]
    UserCancelledElevation,
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Time(#[from] time::error::Parse),
    #[cfg(target_os = "macos")]
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
}

pub type Result<T> = std::result::Result<T, Error>;
