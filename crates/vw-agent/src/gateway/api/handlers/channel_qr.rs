//! Shared QR-code authorization contract for chat channels.

use std::collections::HashMap;
use std::io::Cursor;
use std::str::FromStr;
use std::sync::{Mutex, OnceLock};

use axum::extract::{Extension, Path, Query};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;
use vw_api_types::id::TenantScope;

use crate::app::agent::gateway::ApiError;

mod dingtalk;
mod feishu;
mod qq;
mod wechat;
mod wecom;

const HANDLE_TTL_MINUTES: i64 = 5;

pub(crate) fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/channels/{channel}/qrcode", post(channel_qr_fetch))
        .route("/channels/{channel}/qrcode/status", get(channel_qr_status))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChannelQrKind {
    Dingtalk,
    Feishu,
    Wecom,
    Qq,
    Wechat,
}

impl ChannelQrKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Dingtalk => "dingtalk",
            Self::Feishu => "feishu",
            Self::Wecom => "wecom",
            Self::Qq => "qq",
            Self::Wechat => "wechat",
        }
    }
}

impl FromStr for ChannelQrKind {
    type Err = ApiError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "dingtalk" => Ok(Self::Dingtalk),
            "feishu" => Ok(Self::Feishu),
            "wecom" => Ok(Self::Wecom),
            "qq" => Ok(Self::Qq),
            "wechat" => Ok(Self::Wechat),
            _ => Err(ApiError::bad_request("unsupported QR authorization channel")),
        }
    }
}

#[derive(Debug)]
struct ProviderFetch {
    scan_url: String,
    poll_state: String,
}

#[derive(Debug)]
struct ProviderPoll {
    status: ChannelQrStatus,
    credentials: Option<Map<String, Value>>,
}

#[derive(Clone, Debug)]
struct PendingHandle {
    tenant: TenantScope,
    channel: ChannelQrKind,
    poll_state: String,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ChannelQrFetchResponse {
    channel: String,
    qrcode_png_base64: String,
    handle: String,
    expires_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ChannelQrStatus {
    Pending,
    Scanned,
    Authorized,
    Expired,
    Failed,
}

#[derive(Debug, Serialize)]
pub(crate) struct ChannelQrStatusResponse {
    status: ChannelQrStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    credentials: Option<Map<String, Value>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ChannelQrStatusQuery {
    handle: String,
}

static HANDLES: OnceLock<Mutex<HashMap<String, PendingHandle>>> = OnceLock::new();

fn handles() -> &'static Mutex<HashMap<String, PendingHandle>> {
    HANDLES.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) async fn channel_qr_fetch(
    Extension(tenant): Extension<TenantScope>,
    Path(channel): Path<String>,
) -> Result<Json<ChannelQrFetchResponse>, ApiError> {
    let channel = ChannelQrKind::from_str(channel.trim())?;
    let provider = fetch_from_provider(channel).await?;
    let qrcode_png_base64 = render_qrcode_png(&provider.scan_url)?;
    let expires_at = Utc::now() + Duration::minutes(HANDLE_TTL_MINUTES);
    let handle = Uuid::new_v4().to_string();
    let pending = PendingHandle { tenant, channel, poll_state: provider.poll_state, expires_at };
    handles()
        .lock()
        .map_err(|_| ApiError::internal("QR authorization handle store is unavailable"))?
        .insert(handle.clone(), pending);

    Ok(Json(ChannelQrFetchResponse {
        channel: channel.as_str().to_string(),
        qrcode_png_base64,
        handle,
        expires_at,
    }))
}

pub(crate) async fn channel_qr_status(
    Extension(tenant): Extension<TenantScope>,
    Path(channel): Path<String>,
    Query(query): Query<ChannelQrStatusQuery>,
) -> Result<Json<ChannelQrStatusResponse>, ApiError> {
    let channel = ChannelQrKind::from_str(channel.trim())?;
    let handle = query.handle.trim();
    if handle.is_empty() {
        return Err(ApiError::bad_request("QR authorization handle must not be empty"));
    }

    let pending = {
        let mut entries = handles()
            .lock()
            .map_err(|_| ApiError::internal("QR authorization handle store is unavailable"))?;
        let Some(pending) = entries.get(handle) else {
            return Err(ApiError::not_found("QR authorization handle was not found"));
        };
        if pending.tenant != tenant || pending.channel != channel {
            return Err(ApiError::not_found("QR authorization handle was not found"));
        }
        if pending.expires_at <= Utc::now() {
            entries.remove(handle);
            return Ok(Json(ChannelQrStatusResponse {
                status: ChannelQrStatus::Expired,
                credentials: None,
            }));
        }
        pending.clone()
    };

    let mut result = poll_provider(channel, &pending.poll_state).await?;
    if result.status != ChannelQrStatus::Authorized {
        result.credentials = None;
    } else {
        handles()
            .lock()
            .map_err(|_| ApiError::internal("QR authorization handle store is unavailable"))?
            .remove(handle);
    }
    Ok(Json(ChannelQrStatusResponse { status: result.status, credentials: result.credentials }))
}

async fn fetch_from_provider(channel: ChannelQrKind) -> Result<ProviderFetch, ApiError> {
    match channel {
        ChannelQrKind::Dingtalk => dingtalk::fetch().await,
        ChannelQrKind::Feishu => feishu::fetch().await,
        ChannelQrKind::Wecom => wecom::fetch().await,
        ChannelQrKind::Qq => qq::fetch().await,
        ChannelQrKind::Wechat => wechat::fetch().await,
    }
}

async fn poll_provider(channel: ChannelQrKind, state: &str) -> Result<ProviderPoll, ApiError> {
    match channel {
        ChannelQrKind::Dingtalk => dingtalk::poll(state).await,
        ChannelQrKind::Feishu => feishu::poll(state).await,
        ChannelQrKind::Wecom => wecom::poll(state).await,
        ChannelQrKind::Qq => qq::poll(state).await,
        ChannelQrKind::Wechat => wechat::poll(state).await,
    }
}

fn render_qrcode_png(scan_url: &str) -> Result<String, ApiError> {
    let code = qrcode::QrCode::new(scan_url.as_bytes())
        .map_err(|_| ApiError::internal("QR code image generation failed"))?;
    let image = code.render::<image::Luma<u8>>().quiet_zone(true).build();
    let mut png = Cursor::new(Vec::new());
    image::DynamicImage::ImageLuma8(image)
        .write_to(&mut png, image::ImageFormat::Png)
        .map_err(|_| ApiError::internal("QR code image generation failed"))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(png.into_inner()))
}

#[cfg(test)]
pub(super) fn seed_pending_handle_for_test(tenant: &TenantScope, channel: &str) -> String {
    let channel = ChannelQrKind::from_str(channel).expect("supported test channel");
    let handle = Uuid::new_v4().to_string();
    handles().lock().expect("QR handle store").insert(
        handle.clone(),
        PendingHandle {
            tenant: tenant.clone(),
            channel,
            poll_state: "test-state".to_string(),
            expires_at: Utc::now() + Duration::minutes(HANDLE_TTL_MINUTES),
        },
    );
    handle
}

#[cfg(test)]
#[path = "channel_qr_tests.rs"]
mod channel_qr_tests;
