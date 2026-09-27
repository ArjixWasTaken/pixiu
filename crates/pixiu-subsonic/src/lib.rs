//! The OpenSubsonic REST API.
//!
//! Served by an axum [`Router`] under `/rest/{method}`, which the binary
//! mounts into the Topcoat app. Every method also answers at
//! `/rest/{method}.view`, the form older clients use.

mod params;
pub mod response;

use axum::{
    Router,
    body::Bytes,
    extract::{Path, RawQuery, State},
    http::{HeaderMap, StatusCode, header},
    routing::get,
};
use pixiu_db::Db;

pub use params::Params;
pub use response::{ApiError, Element, ErrorCode, Format, Payload, SubsonicResponse};

/// Shared state for API handlers.
#[derive(Clone)]
pub struct SubsonicState {
    pub db: Db,
}

/// OpenSubsonic extensions and the versions of each that píxiū implements.
const EXTENSIONS: &[(&str, &[u32])] = &[("formPost", &[1])];

/// Builds the API router. Paths are absolute (`/rest/...`), so mount it
/// without stripping the prefix.
pub fn router(state: SubsonicState) -> Router {
    Router::new()
        .route("/rest/{method}", get(handle).post(handle))
        .with_state(state)
}

enum Failure {
    Api(ApiError),
    UnknownMethod,
}

impl From<ApiError> for Failure {
    fn from(error: ApiError) -> Self {
        Failure::Api(error)
    }
}

async fn handle(
    State(state): State<SubsonicState>,
    Path(method): Path<String>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
    body: Bytes,
) -> SubsonicResponse {
    let form_body = is_form(&headers).then_some(&body[..]);
    let params = Params::parse(query.as_deref(), form_body);
    let format = params.format();
    let method = method.strip_suffix(".view").unwrap_or(&method);

    match dispatch(&state, method, &params).await {
        Ok(payload) => SubsonicResponse::ok(format, payload),
        Err(Failure::Api(error)) => SubsonicResponse::error(format, error),
        Err(Failure::UnknownMethod) => {
            tracing::debug!(method, "unknown Subsonic method");
            SubsonicResponse::error(
                format,
                ApiError::new(ErrorCode::Generic, format!("unknown method `{method}`")),
            )
            .with_status(StatusCode::NOT_FOUND)
        }
    }
}

async fn dispatch(
    _state: &SubsonicState,
    method: &str,
    _params: &Params,
) -> Result<Payload, Failure> {
    match method {
        "ping" => Ok(Payload::default()),
        "getLicense" => Ok(Element::new("license").attr("valid", true).into()),
        "getOpenSubsonicExtensions" => Ok(Payload::list(
            "openSubsonicExtensions",
            EXTENSIONS.iter().map(|(name, versions)| {
                Element::new("openSubsonicExtensions")
                    .attr("name", *name)
                    .values("versions", versions.iter().copied())
            }),
        )),
        _ => Err(Failure::UnknownMethod),
    }
}

fn is_form(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("application/x-www-form-urlencoded"))
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;

    use super::*;

    async fn app() -> Router {
        let db = pixiu_db::connect("sqlite::memory:").await.unwrap();
        router(SubsonicState { db })
    }

    async fn call(request: Request<Body>) -> (StatusCode, String, String) {
        let response = app().await.oneshot(request).await.unwrap();
        let status = response.status();
        let content_type = response.headers()[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .to_owned();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (
            status,
            content_type,
            String::from_utf8(body.to_vec()).unwrap(),
        )
    }

    fn get(uri: &str) -> Request<Body> {
        Request::get(uri).body(Body::empty()).unwrap()
    }

    #[tokio::test]
    async fn ping_answers_in_both_formats_and_with_view_suffix() {
        let (status, content_type, body) = call(get("/rest/ping.view")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(content_type, "text/xml; charset=utf-8");
        assert!(body.contains(r#"status="ok""#), "{body}");

        let (_, content_type, body) = call(get("/rest/ping?f=json")).await;
        assert_eq!(content_type, "application/json");
        assert!(body.contains(r#""status":"ok""#), "{body}");
    }

    #[tokio::test]
    async fn form_posts_are_accepted() {
        let request = Request::post("/rest/getLicense")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from("f=json&u=admin"))
            .unwrap();
        let (status, _, body) = call(request).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains(r#""license":{"valid":true}"#), "{body}");
    }

    #[tokio::test]
    async fn extensions_are_advertised() {
        let (_, _, body) = call(get("/rest/getOpenSubsonicExtensions?f=json")).await;
        insta::assert_snapshot!(body, @r#"{"subsonic-response":{"status":"ok","version":"1.16.1","type":"pixiu","serverVersion":"0.1.0","openSubsonic":true,"openSubsonicExtensions":[{"name":"formPost","versions":[1]}]}}"#);
    }

    #[tokio::test]
    async fn unknown_methods_fail_with_404() {
        let (status, _, body) = call(get("/rest/getEverything.view?f=json")).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body.contains(r#""status":"failed""#), "{body}");
        assert!(body.contains("getEverything"), "{body}");
    }
}
