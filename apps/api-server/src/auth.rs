use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
};
use subtle::ConstantTimeEq;
use tracing::warn;

pub struct Authenticated;

#[async_trait]
impl<S> FromRequestParts<S> for Authenticated
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        let expected_secret = std::env::var("API_SERVER_SECRET").unwrap_or_else(|_| {
            if cfg!(debug_assertions) {
                warn!("⚠️ [Auth] Using insecure 'dev_secret' fallback. SET API_SERVER_SECRET FOR PRODUCTION!");
                "dev_secret".to_string()
            } else {
                panic!("🚨 [Auth] FATAL: API_SERVER_SECRET must be set in release builds!");
            }
        });
        let expected = format!("Bearer {}", expected_secret);

        let is_valid = if auth_header.len() == expected.len() {
            bool::from(auth_header.as_bytes().ct_eq(expected.as_bytes()))
        } else {
            false
        };

        if is_valid {
            Ok(Authenticated)
        } else {
            Err((StatusCode::UNAUTHORIZED, "Unauthorized"))
        }
    }
}
