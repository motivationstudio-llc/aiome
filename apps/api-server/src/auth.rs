use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
    response::{IntoResponse, Response},
};
use subtle::ConstantTimeEq;
use tracing::warn;

pub struct Authenticated;

#[async_trait]
impl<S> FromRequestParts<S> for Authenticated
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .unwrap_or_default();

        let query_token =
            axum::extract::Query::<std::collections::HashMap<String, String>>::try_from_uri(
                &parts.uri,
            )
            .ok()
            .and_then(|q| q.get("token").cloned());

        let expected_secret = std::env::var("API_SERVER_SECRET").unwrap_or_else(|_| {
            if cfg!(debug_assertions) {
                "dev_secret".to_string()
            } else {
                panic!("🚨 [Auth] FATAL: API_SERVER_SECRET must be set in release builds!");
            }
        });
        let expected_bearer = format!("Bearer {}", expected_secret);

        let is_valid = if auth_header.len() == expected_bearer.len() {
            bool::from(auth_header.as_bytes().ct_eq(expected_bearer.as_bytes()))
        } else if let Some(token) = query_token {
            bool::from(token.as_bytes().ct_eq(expected_secret.as_bytes()))
        } else {
            false
        };

        if is_valid {
            Ok(Authenticated)
        } else {
            let mut resp = (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
            if !auth_header.is_empty() && auth_header.starts_with("Bearer ") {
                use axum::http::HeaderValue;
                resp.headers_mut()
                    .insert("X-Token-Expired", HeaderValue::from_static("true"));
            }
            Err(resp)
        }
    }
}
