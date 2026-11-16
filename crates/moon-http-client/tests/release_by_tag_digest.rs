//! A release fetched by tag must report asset digests in the same form as the latest release.
#![cfg(not(target_family = "wasm"))]

use std::sync::Arc;

use futures::future::BoxFuture;
use moon_http_client::github::get_release_by_tag_name;
use moon_http_client::http::HeaderValue;
use moon_http_client::{AsyncBody, HttpClient, Request, Response, Url};

/// Answers every request with HTTP 200 and one fixed JSON body.
struct FixedJsonClient {
    body: &'static str,
}

impl HttpClient for FixedJsonClient {
    fn send(
        &self,
        _req: Request<AsyncBody>,
    ) -> BoxFuture<'static, anyhow::Result<Response<AsyncBody>>> {
        let body = self.body;
        Box::pin(async move {
            Ok(Response::builder()
                .status(200)
                .body(AsyncBody::from(body))
                .unwrap())
        })
    }

    fn user_agent(&self) -> Option<&HeaderValue> {
        None
    }

    fn proxy(&self) -> Option<&Url> {
        None
    }
}

/// Catches `github.rs:get_release_by_tag_name` handing back the GitHub API's raw
/// `sha256:<hex>` asset digest. `latest_github_release` strips that prefix, and the download
/// functions in `github_download.rs` compare the digest against a bare lowercase hex SHA-256, so a
/// caller passing a tagged release's digest on would fail every download with a SHA-256 mismatch.
///
/// The expected value is the GitHub REST API's documented digest form, `sha256:` followed by the
/// hex hash, with the algorithm prefix removed.
#[test]
fn release_by_tag_strips_the_sha256_digest_prefix() {
    futures::executor::block_on(async {
        let client = FixedJsonClient {
            body: r#"{
                "tag_name": "v1.0.0",
                "prerelease": false,
                "assets": [{
                    "name": "server.tar.gz",
                    "browser_download_url": "https://example.com/server.tar.gz",
                    "digest": "sha256:9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
                }],
                "tarball_url": "https://example.com/tarball",
                "zipball_url": "https://example.com/zipball"
            }"#,
        };

        let release = get_release_by_tag_name("owner/repo", "v1.0.0", Arc::new(client))
            .await
            .unwrap();

        assert_eq!(
            release.assets[0].digest.as_deref(),
            Some("9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08")
        );
    });
}
