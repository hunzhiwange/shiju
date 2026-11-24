//! OpenAI 图片模型实现。

use crate::core::capabilities::{ImageOutputSupport, ModelName};
use crate::core::client::{merge_body, merge_headers};
use crate::core::image_model::{
    ImageModel, ImageModelOptions, ImageModelResponse, ImageModelUsage, ImageModelWarning,
};
use crate::core::messages::UserImage;
use crate::core::utils::join_url;
use crate::error::{Error, Result};
use crate::providers::openai::OpenAI;
use async_trait::async_trait;
use base64::Engine as _;
use reqwest::header::CONTENT_TYPE;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
struct OpenAIImageGenerationRequest {
    model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    n: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAIImageResponse {
    data: Vec<OpenAIImageData>,
    usage: Option<OpenAIImageUsage>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAIImageData {
    b64_json: String,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAIImageUsage {
    input_tokens: Option<usize>,
    output_tokens: Option<usize>,
    total_tokens: Option<usize>,
}

#[async_trait]
impl<M> ImageModel for OpenAI<M>
where
    M: ModelName,
    OpenAI<M>: ImageOutputSupport,
{
    async fn generate_image(&self, input: ImageModelOptions) -> Result<ImageModelResponse> {
        let model_name = self.lm_options.model.clone();
        let warnings = openai_warnings(&input);

        let response: OpenAIImageResponse = if input.files.is_empty() {
            let request = OpenAIImageGenerationRequest {
                model: model_name.clone(),
                prompt: input.prompt.clone(),
                n: input.n,
                size: input.size.clone(),
                response_format: (!has_default_response_format(&model_name))
                    .then_some("b64_json".to_string()),
            };

            let mut default_headers = reqwest::header::HeaderMap::new();
            default_headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
            default_headers.insert(
                "Authorization",
                format!("Bearer {}", self.settings.api_key).parse().unwrap(),
            );

            let headers = merge_headers(
                default_headers,
                self.settings.headers.as_ref(),
                input.headers.as_ref(),
            )?;
            let body = merge_body(&request, self.settings.body.as_ref(), input.body.as_ref())?;
            let url = join_url(
                &self.settings.base_url,
                self.settings
                    .path
                    .as_deref()
                    .unwrap_or("/v1/images/generations"),
            )?;

            send_json_request(url, headers, body).await?
        } else {
            let mut default_headers = reqwest::header::HeaderMap::new();
            default_headers.insert(
                "Authorization",
                format!("Bearer {}", self.settings.api_key).parse().unwrap(),
            );

            let headers = merge_headers(
                default_headers,
                self.settings.headers.as_ref(),
                input.headers.as_ref(),
            )?;

            let mut form = reqwest::multipart::Form::new().text("model", model_name);
            if let Some(prompt) = input.prompt.clone() {
                form = form.text("prompt", prompt);
            }
            if let Some(n) = input.n {
                form = form.text("n", n.to_string());
            }
            if let Some(size) = input.size.clone() {
                form = form.text("size", size);
            }

            for file in &input.files {
                form = form.part("image", image_to_multipart_part(file, "image").await?);
            }

            if let Some(mask) = &input.mask {
                form = form.part("mask", image_to_multipart_part(mask, "mask").await?);
            }

            for (key, value) in merge_extra_body(self.settings.body.as_ref(), input.body.as_ref()) {
                if value.is_null() {
                    continue;
                }
                form = form.text(key, form_value_to_string(value));
            }

            let url = join_url(
                &self.settings.base_url,
                self.settings.path.as_deref().unwrap_or("/v1/images/edits"),
            )?;

            send_multipart_request(url, headers, form).await?
        };

        Ok(ImageModelResponse {
            images: response
                .data
                .into_iter()
                .map(|item| item.b64_json)
                .collect(),
            usage: response.usage.map(|usage| ImageModelUsage {
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
                total_tokens: usage.total_tokens,
            }),
            warnings,
        })
    }
}

fn openai_warnings(input: &ImageModelOptions) -> Vec<ImageModelWarning> {
    let mut warnings = Vec::new();

    if input.aspect_ratio.is_some() {
        warnings.push(ImageModelWarning::Unsupported {
            feature: "aspect_ratio",
            details: Some("This model does not support aspect_ratio. Use size instead.".into()),
        });
    }

    if input.seed.is_some() {
        warnings.push(ImageModelWarning::Unsupported {
            feature: "seed",
            details: None,
        });
    }

    warnings
}

fn has_default_response_format(model_name: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "chatgpt-image-",
        "gpt-image-1-mini",
        "gpt-image-1.5",
        "gpt-image-1",
        "gpt-image-2",
    ];

    PREFIXES.iter().any(|prefix| model_name.starts_with(prefix))
}

async fn image_to_multipart_part(
    image: &UserImage,
    fallback_name: &str,
) -> Result<reqwest::multipart::Part> {
    let (bytes, media_type) = load_image_bytes(image).await?;
    let file_name = infer_file_name(image, fallback_name, &media_type);

    reqwest::multipart::Part::bytes(bytes)
        .file_name(file_name)
        .mime_str(&media_type)
        .map_err(|error| Error::InvalidInput(format!("invalid image media type: {error}")))
}

async fn load_image_bytes(image: &UserImage) -> Result<(Vec<u8>, String)> {
    if let Some((media_type, data)) = image.inline_data() {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(data)
            .map_err(|error| {
                Error::InvalidInput(format!("failed to decode data url image payload: {error}"))
            })?;
        return Ok((bytes, media_type));
    }

    let media_type = image.resolved_media_type().ok_or_else(|| {
        Error::InvalidInput(
            "input image requires an explicit media type or a URL with a known image extension"
                .to_string(),
        )
    })?;

    let response = reqwest::Client::new()
        .get(&image.image_url)
        .send()
        .await
        .map_err(|error| Error::ApiError {
            status_code: error.status(),
            details: error.to_string(),
        })?;
    let status = response.status();
    let bytes = response.bytes().await.map_err(|error| Error::ApiError {
        status_code: Some(status),
        details: format!("failed to read image bytes: {error}"),
    })?;

    if !status.is_success() {
        return Err(Error::ApiError {
            status_code: Some(status),
            details: String::from_utf8_lossy(&bytes).into_owned(),
        });
    }

    Ok((bytes.to_vec(), media_type))
}

fn infer_file_name(image: &UserImage, fallback_name: &str, media_type: &str) -> String {
    let path = image
        .image_url
        .split(['?', '#'])
        .next()
        .unwrap_or(&image.image_url);
    let candidate = path.rsplit('/').next().unwrap_or_default();

    if !candidate.is_empty() && candidate.contains('.') {
        return candidate.to_string();
    }

    let extension = match media_type {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "bin",
    };

    format!("{fallback_name}.{extension}")
}

fn merge_extra_body(
    provider_body: Option<&serde_json::Map<String, serde_json::Value>>,
    request_body: Option<&serde_json::Map<String, serde_json::Value>>,
) -> serde_json::Map<String, serde_json::Value> {
    let mut merged = serde_json::Map::new();

    if let Some(provider_body) = provider_body {
        merged.extend(provider_body.clone());
    }
    if let Some(request_body) = request_body {
        merged.extend(request_body.clone());
    }

    merged
}

fn form_value_to_string(value: serde_json::Value) -> String {
    match value {
        serde_json::Value::String(value) => value,
        other => other.to_string(),
    }
}

async fn send_json_request<T>(
    url: reqwest::Url,
    headers: reqwest::header::HeaderMap,
    body: reqwest::Body,
) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let response = reqwest::Client::new()
        .post(url)
        .headers(headers)
        .body(body)
        .send()
        .await
        .map_err(|error| Error::ApiError {
            status_code: error.status(),
            details: error.to_string(),
        })?;

    parse_json_response(response).await
}

async fn send_multipart_request<T>(
    url: reqwest::Url,
    headers: reqwest::header::HeaderMap,
    form: reqwest::multipart::Form,
) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let response = reqwest::Client::new()
        .post(url)
        .headers(headers)
        .multipart(form)
        .send()
        .await
        .map_err(|error| Error::ApiError {
            status_code: error.status(),
            details: error.to_string(),
        })?;

    parse_json_response(response).await
}

async fn parse_json_response<T>(response: reqwest::Response) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let status = response.status();
    let body = response.text().await.map_err(|error| Error::ApiError {
        status_code: error.status(),
        details: format!("failed to read response body: {error}"),
    })?;

    if !status.is_success() {
        return Err(Error::ApiError {
            status_code: Some(status),
            details: body,
        });
    }

    serde_json::from_str(&body).map_err(|error| Error::ApiError {
        status_code: Some(status),
        details: format!("failed to parse response: {error}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{DynamicModel, ImageModelRequest};
    use serde_json::json;
    use std::collections::HashMap;
    use wiremock::matchers::{
        body_partial_json, body_string_contains, header, header_regex, method, path,
    };
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_model(base_url: String) -> OpenAI<DynamicModel> {
        let mut model = OpenAI::model_name("gpt-image-1");
        model.settings.base_url = base_url;
        model.settings.api_key = "test-key".to_string();
        model
    }

    fn image_response() -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(json!({
            "data": [{ "b64_json": "Zm9v" }],
            "usage": {
                "input_tokens": 11,
                "output_tokens": 22,
                "total_tokens": 33
            }
        }))
    }

    #[tokio::test]
    async fn test_generate_image_sends_json_request() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/images/generations"))
            .and(header("authorization", "Bearer test-key"))
            .and(header("x-trace-id", "image-123"))
            .and(body_partial_json(json!({
                "model": "gpt-image-1",
                "prompt": "draw a lighthouse at sunrise",
                "n": 2,
                "size": "1024x1024",
                "quality": "high"
            })))
            .respond_with(image_response())
            .expect(1)
            .mount(&server)
            .await;

        let response = ImageModelRequest::builder()
            .model(test_model(server.uri()))
            .prompt("draw a lighthouse at sunrise")
            .n(2)
            .size("1024x1024")
            .headers(HashMap::from([(
                "x-trace-id".to_string(),
                "image-123".to_string(),
            )]))
            .body(json!({
                "quality": "high"
            }))
            .build()
            .generate_image()
            .await
            .expect("image generation should succeed");

        assert_eq!(response.images, vec!["Zm9v".to_string()]);
        assert_eq!(response.usage.unwrap().total_tokens, Some(33));
    }

    #[tokio::test]
    async fn test_edit_image_sends_multipart_request() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/images/edits"))
            .and(header("authorization", "Bearer test-key"))
            .and(header_regex(
                "content-type",
                "multipart/form-data; boundary=.*",
            ))
            .and(body_string_contains("name=\"model\""))
            .and(body_string_contains("name=\"prompt\""))
            .and(body_string_contains("name=\"image\""))
            .and(body_string_contains("name=\"mask\""))
            .and(body_string_contains("name=\"background\""))
            .respond_with(image_response())
            .expect(1)
            .mount(&server)
            .await;

        let response = ImageModelRequest::builder()
            .model(test_model(server.uri()))
            .prompt("replace the sky with aurora")
            .file(UserImage::new("data:image/png;base64,aGVsbG8="))
            .mask(UserImage::new("data:image/png;base64,d29ybGQ="))
            .body(json!({
                "background": "transparent"
            }))
            .build()
            .generate_image()
            .await
            .expect("image edit should succeed");

        assert_eq!(response.images, vec!["Zm9v".to_string()]);

        let requests = server
            .received_requests()
            .await
            .expect("server should expose received requests");
        let body = String::from_utf8_lossy(&requests[0].body);
        assert!(body.contains("gpt-image-1"));
        assert!(body.contains("replace the sky with aurora"));
        assert!(body.contains("transparent"));
    }

    #[tokio::test]
    async fn test_generate_image_returns_unsupported_warnings() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/images/generations"))
            .respond_with(image_response())
            .expect(1)
            .mount(&server)
            .await;

        let response = ImageModelRequest::builder()
            .model(test_model(server.uri()))
            .prompt("draw a fox")
            .aspect_ratio("16:9")
            .seed(7u32)
            .build()
            .generate_image()
            .await
            .expect("image generation should succeed");

        assert_eq!(response.warnings.len(), 2);
    }
}
