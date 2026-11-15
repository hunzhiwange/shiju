//! Bounded Gemini token exchange without account storage orchestration.

use app_auth_core::response::{read_body_limited, send};
use reqwest::{Client, Request, StatusCode};
use serde::Deserialize;
use serde_json::Value;
use vw_api_app_auth::AuthErrorCode;

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;

pub struct GeminiOAuth {
    client: Client,
}

#[derive(Deserialize)]
pub struct RefreshResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<i64>,
}

impl GeminiOAuth {
    pub fn new() -> Result<Self, AuthErrorCode> {
        Ok(Self {
            client: Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(std::time::Duration::from_secs(30))
                .user_agent("ShiJu-Gemini-OAuth")
                .build()
                .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?,
        })
    }

    pub async fn exchange(
        &self,
        client_id: &str,
        client_secret: &str,
        refresh_token: &str,
        route: &mut impl FnMut(&mut Request),
    ) -> Result<RefreshResponse, AuthErrorCode> {
        let endpoint = "https://oauth2.googleapis.com/token";
        let response = send(
            &self.client,
            self.client.post(endpoint).form(&[
                ("grant_type", "refresh_token"),
                ("client_id", client_id),
                ("client_secret", client_secret),
                ("refresh_token", refresh_token),
            ]),
            route,
        )
        .await?;
        let status = response.status();
        if matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) {
            return Err(AuthErrorCode::ReauthRequired);
        }
        if status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS {
            return Err(AuthErrorCode::UpstreamFailure);
        }
        // Only a fully received malformed 400 is a Gemini rejection. A truncated,
        // oversized or unreadable response remains an upstream failure.
        let body = read_body_limited(response, MAX_RESPONSE_BYTES).await?;
        let payload: Value = serde_json::from_slice(&body).map_err(|_| {
            if status == StatusCode::BAD_REQUEST {
                AuthErrorCode::ReauthRequired
            } else {
                AuthErrorCode::UpstreamFailure
            }
        })?;
        match payload.get("error").and_then(Value::as_str) {
            Some("invalid_grant" | "invalid_token") => Err(AuthErrorCode::ReauthRequired),
            None if status.is_success() => {
                serde_json::from_value(payload).map_err(|_| AuthErrorCode::UpstreamFailure)
            }
            _ => Err(AuthErrorCode::UpstreamFailure),
        }
    }
}
