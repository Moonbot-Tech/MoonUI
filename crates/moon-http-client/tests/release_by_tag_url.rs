//! A release fetched by tag must be requested under exactly that tag name.
#![cfg(not(target_family = "wasm"))]

use std::sync::{Arc, Mutex};

use futures::future::BoxFuture;
use moon_http_client::github::get_release_by_tag_name;
use moon_http_client::http::HeaderValue;
use moon_http_client::{AsyncBody, HttpClient, Request, Response, Url};

/// Records the URI of every request and answers it with HTTP 404.
struct RecordingClient {
    uris: Arc<Mutex<Vec<String>>>,
}

impl HttpClient for RecordingClient {
    fn send(
        &self,
        req: Request<AsyncBody>,
    ) -> BoxFuture<'static, anyhow::Result<Response<AsyncBody>>> {
        self.uris.lock().unwrap().push(req.uri().to_string());
        Box::pin(async move {
            Ok(Response::builder()
                .status(404)
                .body(AsyncBody::from("{}"))
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

fn requested_uri(tag: &str) -> String {
    let uris = Arc::new(Mutex::new(Vec::new()));
    let client = RecordingClient { uris: uris.clone() };
    let _ =
        futures::executor::block_on(get_release_by_tag_name("owner/repo", tag, Arc::new(client)));
    let uris = uris.lock().unwrap();
    assert_eq!(uris.len(), 1, "exactly one request expected: {uris:?}");
    uris[0].clone()
}

/// Catches `github.rs:get_release_by_tag_name` pasting the tag into the request URL unescaped:
/// the valid Git tag `v1#2` then asks the GitHub API for the release `v1`, because `#` starts the
/// URL fragment and everything after it is never sent, so a caller can silently receive another
/// release's assets.
///
/// `git check-ref-format` allows `#` in a tag name, and RFC 3986 section 3.5 makes an unescaped
/// `#` end the path, so the tag segment has to travel as `v1%232`. A tag with `/`, such as
/// `release/2.3.5`, keeps the request it gets today, its slash a path separator.
#[test]
fn release_by_tag_requests_the_tag_it_was_given() {
    assert_eq!(
        requested_uri("v1#2"),
        "https://api.github.com/repos/owner/repo/releases/tags/v1%232"
    );
    assert_eq!(
        requested_uri("release/2.3.5"),
        "https://api.github.com/repos/owner/repo/releases/tags/release/2.3.5"
    );
}
