use serde::{de::Error as DeError, Deserialize, Deserializer};
use std::ffi::OsString;
use url::Url;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WindowsConfig {
    #[serde(default, alias = "installer-args", deserialize_with = "deserialize_os_string")]
    pub installer_args: Vec<OsString>,
}

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub dangerous_insecure_transport_protocol: bool,
    pub dangerous_accept_invalid_certs: bool,
    pub dangerous_accept_invalid_hostnames: bool,
    pub endpoints: Vec<Url>,
    pub pubkey: String,
    pub windows: Option<WindowsConfig>,
}

fn deserialize_os_string<'de, D>(deserializer: D) -> Result<Vec<OsString>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Vec::<String>::deserialize(deserializer)?
        .into_iter()
        .map(OsString::from)
        .collect())
}

impl<'de> Deserialize<'de> for Config {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct InnerConfig {
            #[serde(default, alias = "dangerous-insecure-transport-protocol")]
            dangerous_insecure_transport_protocol: bool,
            #[serde(default, alias = "dangerous-accept-invalid-certs")]
            dangerous_accept_invalid_certs: bool,
            #[serde(default, alias = "dangerous-accept-invalid-hostnames")]
            dangerous_accept_invalid_hostnames: bool,
            #[serde(default)]
            endpoints: Vec<Url>,
            pubkey: String,
            windows: Option<WindowsConfig>,
        }

        let config = InnerConfig::deserialize(deserializer)?;
        validate_endpoints(
            &config.endpoints,
            config.dangerous_insecure_transport_protocol,
        )
        .map_err(DeError::custom)?;

        Ok(Self {
            dangerous_insecure_transport_protocol: config.dangerous_insecure_transport_protocol,
            dangerous_accept_invalid_certs: config.dangerous_accept_invalid_certs,
            dangerous_accept_invalid_hostnames: config.dangerous_accept_invalid_hostnames,
            endpoints: config.endpoints,
            pubkey: config.pubkey,
            windows: config.windows,
        })
    }
}

pub(crate) fn validate_endpoints(
    endpoints: &[Url],
    dangerous_insecure_transport_protocol: bool,
) -> crate::Result<()> {
    if !dangerous_insecure_transport_protocol {
        for url in endpoints {
            if url.scheme() != "https" {
                return Err(crate::Error::InsecureTransportProtocol);
            }
        }
    }

    Ok(())
}
