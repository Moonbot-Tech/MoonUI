//! A proxy taken from the environment must keep the host and port the user wrote.

/// Catches `http_client.rs:read_proxy_from_env` parsing a scheme-less proxy value such as
/// `proxy.example:3128` as a URL whose *scheme* is `proxy.example` and whose path is `3128`, with
/// no host at all, so `HttpClient::proxy()` reports a proxy URL nobody can connect to.
///
/// curl, whose `http_proxy` / `HTTPS_PROXY` / `ALL_PROXY` conventions these variables follow,
/// documents `[protocol://]host[:port]` and treats a value with no scheme as an HTTP proxy.
#[test]
fn schemeless_proxy_env_value_is_an_http_proxy() {
    for var in [
        "ALL_PROXY",
        "all_proxy",
        "HTTPS_PROXY",
        "https_proxy",
        "HTTP_PROXY",
        "http_proxy",
    ] {
        // SAFETY: this test binary runs this single test, so no other thread reads the environment.
        unsafe { std::env::remove_var(var) };
    }
    // SAFETY: as above.
    unsafe { std::env::set_var("HTTPS_PROXY", "proxy.example:3128") };

    let proxy = moon_http_client::read_proxy_from_env().expect("a proxy is configured");

    assert_eq!(proxy.host_str(), Some("proxy.example"));
    assert_eq!(proxy.port_or_known_default(), Some(3128));
}
