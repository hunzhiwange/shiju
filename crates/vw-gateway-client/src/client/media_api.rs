//! Authenticated client for the session-scoped media job API.

use std::fmt;

use futures_util::{
    StreamExt,
    stream::{self, BoxStream},
};
use gateway_http::apply_request_context;
use reqwest::{Method, RequestBuilder, Response, StatusCode, header::CONTENT_TYPE};
use serde::de::DeserializeOwned;

use super::GatewayClient;
use MediaErrorCode as Code;
pub use vw_api_media::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaApiError {
    Service(Code),
    Transport,
    InvalidResponse,
    /// The POST might have reached the gateway. Query this ID in the same session.
    SubmissionUnknown {
        job_id: MediaJobId,
    },
}

impl fmt::Display for MediaApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Service(code) => f.write_str(code.message()),
            Self::Transport => f.write_str("Could not reach the media gateway."),
            Self::InvalidResponse => f.write_str("Invalid media gateway response."),
            Self::SubmissionUnknown { job_id } => {
                write!(f, "Media job {job_id} submission is unknown.")
            }
        }
    }
}

impl std::error::Error for MediaApiError {}

impl GatewayClient {
    pub async fn media_models(
        &self,
        session: &MediaSessionQuery,
    ) -> Result<MediaModels, MediaApiError> {
        session.validate().map_err(MediaApiError::Service)?;
        let response = self
            .media_request(Method::GET, MEDIA_MODELS_PATH)
            .query(session)
            .send()
            .await
            .map_err(|_| MediaApiError::Transport)?;
        media_json(response).await
    }

    pub async fn media_models_for_project(&self, project_id: &str) -> Result<MediaModels, MediaApiError> {
        let response = self
            .media_request(Method::GET, MEDIA_MODELS_PATH)
            .query(&[("project_id", project_id)])
            .send()
            .await
            .map_err(|_| MediaApiError::Transport)?;
        media_json(response).await
    }

    /// Submit once. Even a malformed success response may mean the job exists.
    pub async fn media_submit(&self, request: &MediaSubmit) -> Result<MediaJob, MediaApiError> {
        request.validate().map_err(MediaApiError::Service)?;
        let unknown = MediaApiError::SubmissionUnknown { job_id: request.request_id };
        let response = self
            .media_request(Method::POST, MEDIA_JOBS_PATH)
            .json(request)
            .send()
            .await
            .map_err(|_| unknown)?;
        let job: MediaJob = match media_json(response).await {
            Ok(job) => job,
            Err(error @ MediaApiError::Service(code)) if code.http_status() < 500 => {
                return Err(error);
            }
            Err(_) => return Err(unknown),
        };
        if job.id != request.request_id
            || job.session_id != request.session_id
            || job.operation != request.operation
            || !valid_job_state(&job.state)
        {
            return Err(unknown);
        }
        Ok(job)
    }

    pub async fn media_job(
        &self,
        session: &MediaSessionQuery,
        id: MediaJobId,
    ) -> Result<MediaJob, MediaApiError> {
        self.media_job_request(Method::GET, session, id).await
    }

    pub async fn media_cancel(
        &self,
        session: &MediaSessionQuery,
        id: MediaJobId,
    ) -> Result<MediaJob, MediaApiError> {
        self.media_job_request(Method::DELETE, session, id).await
    }

    async fn media_job_request(
        &self,
        method: Method,
        session: &MediaSessionQuery,
        id: MediaJobId,
    ) -> Result<MediaJob, MediaApiError> {
        let path = media_job_path(session, id)?;
        let response = self
            .media_request(method, &path)
            .query(session)
            .send()
            .await
            .map_err(|_| MediaApiError::Transport)?;
        let job: MediaJob = media_json(response).await?;
        if job.id != id || job.session_id != session.session_id || !valid_job_state(&job.state) {
            return Err(MediaApiError::InvalidResponse);
        }
        Ok(job)
    }

    /// Read an authorized asset incrementally. The expected MIME and length come
    /// from the successful job result; dropping the stream closes the read.
    pub async fn media_asset(
        &self,
        session: &MediaSessionQuery,
        id: MediaJobId,
        asset: &MediaAsset,
    ) -> Result<BoxStream<'static, Result<Vec<u8>, MediaApiError>>, MediaApiError> {
        asset.validate().map_err(MediaApiError::Service)?;
        let path = format!("{}/assets/{}", media_job_path(session, id)?, asset.id);
        let response = self
            .media_request(Method::GET, &path)
            .query(session)
            .send()
            .await
            .map_err(|_| MediaApiError::Transport)?;
        if !response.status().is_success() {
            return Err(media_error(response).await);
        }
        let mime = response.headers().get(CONTENT_TYPE).and_then(|value| value.to_str().ok());
        if response.status() != StatusCode::OK
            || mime != Some(asset.mime.as_str())
            || response.content_length().is_some_and(|length| length != asset.byte_len)
        {
            return Err(MediaApiError::InvalidResponse);
        }

        Ok(stream::try_unfold(
            (Box::pin(response.bytes_stream()), asset.byte_len),
            |(mut stream, remaining)| async move {
                match stream.next().await {
                    Some(Ok(bytes)) if bytes.len() as u64 <= remaining => {
                        let remaining = remaining - bytes.len() as u64;
                        Ok(Some((bytes.to_vec(), (stream, remaining))))
                    }
                    Some(Err(_)) => Err(MediaApiError::Transport),
                    None if remaining == 0 => Ok(None),
                    _ => Err(MediaApiError::InvalidResponse),
                }
            },
        )
        .boxed())
    }

    fn media_request(&self, method: Method, path: &str) -> RequestBuilder {
        let endpoint = self.endpoint();
        // The native transport disables both redirects and automatic POST retries.
        let request = self
            .transport()
            .dynamic_host_client()
            .request(method, format!("{}{path}", endpoint.base_url()));
        apply_request_context(request, endpoint)
    }
}

fn media_job_path(session: &MediaSessionQuery, id: MediaJobId) -> Result<String, MediaApiError> {
    session.validate().map_err(MediaApiError::Service)?;
    if id.is_nil() {
        return Err(MediaApiError::Service(Code::InvalidRequest));
    }
    Ok(format!("{MEDIA_JOBS_PATH}/{id}"))
}

fn valid_job_state(state: &MediaJobState) -> bool {
    match state {
        MediaJobState::Running { progress_percent } => {
            progress_percent.is_none_or(|value| value <= 100)
        }
        MediaJobState::Succeeded { result } => result.validate().is_ok(),
        _ => true,
    }
}

async fn media_json<T: DeserializeOwned>(response: Response) -> Result<T, MediaApiError> {
    if !response.status().is_success() {
        return Err(media_error(response).await);
    }
    let bytes = media_json_body(response).await?;
    serde_json::from_slice(&bytes).map_err(|_| MediaApiError::InvalidResponse)
}

async fn media_json_body(response: Response) -> Result<Vec<u8>, MediaApiError> {
    if response.content_length().is_some_and(|length| length > MAX_MEDIA_JSON_BYTES as u64) {
        return Err(MediaApiError::InvalidResponse);
    }
    let mut stream = response.bytes_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| MediaApiError::Transport)?;
        if chunk.len() > MAX_MEDIA_JSON_BYTES - bytes.len() {
            return Err(MediaApiError::InvalidResponse);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

async fn media_error(response: Response) -> MediaApiError {
    let status = response.status();
    // Authentication middleware and body limits can reject before the media handler.
    let fallback = match status {
        StatusCode::UNAUTHORIZED => Some(Code::Unauthorized),
        StatusCode::FORBIDDEN => Some(Code::Forbidden),
        StatusCode::PAYLOAD_TOO_LARGE => Some(Code::PayloadTooLarge),
        _ => None,
    };
    if let Some(code) = fallback {
        return MediaApiError::Service(code);
    }
    match media_json_body(response).await {
        Ok(body) => match serde_json::from_slice::<MediaErrorResponse>(&body) {
            Ok(response) if response.error.code.http_status() == status.as_u16() => {
                MediaApiError::Service(response.error.code)
            }
            _ => MediaApiError::InvalidResponse,
        },
        Err(error) => error,
    }
}

#[cfg(test)]
#[path = "media_api_tests.rs"]
mod tests;
