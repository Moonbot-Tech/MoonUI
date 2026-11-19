//! Archive downloads must refuse an asset whose SHA-256 does not match the expected digest.
#![cfg(all(not(target_family = "wasm"), feature = "github-download"))]

use futures::future::BoxFuture;
use moon_http_client::github::AssetKind;
use moon_http_client::github_download::download_server_binary;
use moon_http_client::http::HeaderValue;
use moon_http_client::{AsyncBody, HttpClient, Request, Response, Url};

/// Answers every request with `200 OK` and one fixed body.
struct FixedBodyClient {
    body: &'static [u8],
}

impl HttpClient for FixedBodyClient {
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

/// Catches dropping or inverting the digest comparison in
/// `github_download.rs:extract_to_staging`, or leaking the staging directory on that error: a
/// tampered or truncated `.tar.gz` release asset would then be unpacked and installed as the
/// server, or each failed attempt would leave a `.tmp-github-download-*` directory beside it.
///
/// The body is not a valid archive and the expected digest is all zeros, which no SHA-256 of
/// `not a tarball` equals, so the only correct outcome is an error that names the mismatch and
/// an untouched parent directory.
#[test]
fn archive_download_with_wrong_digest_is_rejected_and_cleaned_up() {
    futures::executor::block_on(async {
        let temp_dir = tempfile::tempdir().unwrap();
        let destination_path = temp_dir.path().join("v_1");
        let client = FixedBodyClient {
            body: b"not a tarball",
        };

        let error = download_server_binary(
            &client,
            "https://example.com/server.tar.gz",
            Some("0000000000000000000000000000000000000000000000000000000000000000"),
            &destination_path,
            AssetKind::TarGz,
        )
        .await
        .expect_err("an asset with the wrong digest must not be installed");

        assert!(
            error.to_string().contains("SHA-256 mismatch"),
            "error should name the digest mismatch: {error:#}"
        );
        assert!(!destination_path.exists());
        assert_eq!(
            std::fs::read_dir(temp_dir.path()).unwrap().count(),
            0,
            "the staging directory should be removed"
        );
    });
}
