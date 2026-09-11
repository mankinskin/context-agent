use axum::{
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tower_http::services::{ServeDir, ServeFile};

#[derive(Debug, Deserialize)]
pub struct EchoRequest {
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EchoResponse {
    pub message: String,
}

/// Builds the application router: the unauthenticated health check, a stub
/// echo endpoint (replace with your own `/api/*` handlers), and same-origin
/// static frontend serving for every other path.
pub fn router(static_dir: &str) -> Router {
    let serve_dir = ServeDir::new(static_dir)
        .not_found_service(ServeFile::new(format!("{static_dir}/index.html")));
    Router::new()
        .route("/health", get(health))
        .route("/api/echo", post(echo))
        .fallback_service(serve_dir)
}

async fn health() -> &'static str {
    "ok"
}

async fn echo(Json(req): Json<EchoRequest>) -> Json<EchoResponse> {
    Json(EchoResponse {
        message: req.message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_returns_200() {
        let app = router("frontend/dist");
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], b"ok");
    }

    #[tokio::test]
    async fn echo_returns_submitted_message() {
        let app = router("frontend/dist");
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/echo")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"message":"hi"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let parsed: EchoResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed.message, "hi");
    }
}
