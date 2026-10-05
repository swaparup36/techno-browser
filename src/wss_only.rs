use hbb_common::ResultType;
#[cfg(all(
    feature = "wss-only",
    not(any(target_os = "android", target_os = "ios"))
))]
use hbb_common::{
    config::{Config, RENDEZVOUS_PORT},
    socket_client::IsResolvedSocketAddr,
    tokio::net::ToSocketAddrs,
    tokio_socks::IntoTargetAddr,
    Stream,
};
#[cfg(all(
    feature = "wss-only",
    not(any(target_os = "android", target_os = "ios"))
))]
use std::fmt::Display;

#[cfg(not(all(
    feature = "wss-only",
    not(any(target_os = "android", target_os = "ios"))
)))]
pub use hbb_common::socket_client::connect_tcp;

pub const fn enabled() -> bool {
    cfg!(all(
        feature = "wss-only",
        not(any(target_os = "android", target_os = "ios"))
    ))
}

pub fn configure() -> ResultType<()> {
    #[cfg(all(
        feature = "wss-only",
        not(any(target_os = "android", target_os = "ios"))
    ))]
    {
        use hbb_common::{bail, config::keys::OPTION_ALLOW_WEBSOCKET};

        let id_server = Config::get_option("custom-rendezvous-server");
        let id_url = url::Url::parse(&format!("https://{id_server}"))?;
        let host = id_url
            .host_str()
            .ok_or_else(|| hbb_common::anyhow::anyhow!("Set an ID server domain for WSS"))?;
        if id_server.is_empty()
            || host.parse::<std::net::IpAddr>().is_ok()
            || id_url
                .port()
                .is_some_and(|port| port != RENDEZVOUS_PORT as u16)
            || id_url.path() != "/"
            || id_url.query().is_some()
            || id_url.fragment().is_some()
            || !id_url.username().is_empty()
        {
            bail!("WSS-only mode requires an ID server domain (optionally :21116)");
        }
        let api_server = Config::get_option("api-server");
        if api_server.is_empty() {
            Config::set_option("api-server".into(), format!("https://{host}"));
        } else {
            let api_url = url::Url::parse(&api_server)?;
            if api_url.scheme() != "https" || api_url.port_or_known_default() != Some(443) {
                bail!("WSS-only mode requires an HTTPS API server on port 443");
            }
        }
        if Config::get_option(OPTION_ALLOW_WEBSOCKET) != "Y" {
            Config::set_option(OPTION_ALLOW_WEBSOCKET.into(), "Y".into());
        }
    }
    Ok(())
}

#[cfg(all(
    feature = "wss-only",
    not(any(target_os = "android", target_os = "ios"))
))]
pub async fn connect_tcp<'t, T>(target: T, timeout: u64) -> ResultType<Stream>
where
    T: IntoTargetAddr<'t> + ToSocketAddrs + IsResolvedSocketAddr + Display,
{
    use hbb_common::bail;

    configure()?;
    let endpoint = hbb_common::websocket::check_ws(&target.to_string());
    let url = url::Url::parse(&endpoint)?;
    if url.scheme() != "wss" || url.port_or_known_default() != Some(443) {
        bail!("WSS-only mode blocked non-443 server connection: {endpoint}");
    }
    hbb_common::socket_client::connect_tcp(endpoint.as_str(), timeout).await
}
