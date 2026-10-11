//! Raw-binary downloads must reject an HTTP error response instead of installing its body.
#![cfg(all(not(target_family = "wasm"), feature = "github-download"))]

use futures::future::BoxFuture;
use moon_http_client::github_download::download_server_raw_binary;
use moon_http_client::http::HeaderValue;
use moon_http_client::{AsyncBody, HttpClient, Request, Response, Url};

/// Answers every request with one fixed status code and body.
struct FixedResponseClient {
    status: u16,
    body: &'static [u8],
}

impl HttpClient for FixedResponseClient {
    fn send(
        &self,
        _req: Request<AsyncBody>,
    ) -> BoxFuture<'static, anyhow::Result<Response<AsyncBody>>> {
        let status = self.status;
        let body = self.body;
        Box::pin(async move {
            Ok(Response::builder()
                .status(status)
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

/// Catches dropping the status check in `github_download.rs:download_server_raw_binary`, so a
/// missing release asset (HTTP 404, body `Not Found`) downloaded without a digest is installed and
/// marked executable as the server binary, and the caller is told the download succeeded.
///
/// The expected outcome comes from HTTP semantics (RFC 9110 section 15.5): a 4xx response body
/// describes the error, it is not a representation of the requested resource.
#[test]
fn raw_binary_download_rejects_not_found_response() {
    futures::executor::block_on(async {
        let temp_dir = tempfile::tempdir().unwrap();
        let destination_path = temp_dir.path().join("v_1");
        let client = FixedResponseClient {
            status: 404,
            body: b"Not Found",
        };

        let result = download_server_raw_binary(
            &client,
            "https://example.com/agent-binary",
            None,
            &destination_path,
            "agent-binary",
        )
        .await;

        let error = result.expect_err("a 404 response must not count as a downloaded binary");
        assert!(
            error.to_string().contains("404"),
            "error should name the status: {error:#}"
        );
        assert!(
            !destination_path.exists(),
            "the error body must not be installed at the destination"
        );
        assert_eq!(
            std::fs::read_dir(temp_dir.path()).unwrap().count(),
            0,
            "the staging directory should be removed"
        );
    });
}
