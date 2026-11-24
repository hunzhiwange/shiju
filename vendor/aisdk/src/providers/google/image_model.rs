//! Google 图片模型实现。

use crate::core::capabilities::{ImageOutputSupport, ModelName};
use crate::core::client::{merge_body, merge_headers};
use crate::core::image_model::{
    ImageModel, ImageModelOptions, ImageModelResponse, ImageModelUsage, ImageModelWarning,
};
use crate::core::messages::UserImage;
use crate::core::utils::join_url;
use crate::error::{Error, Result};
use crate::providers::google::Google;
use crate::providers::google::client::types::{
    Blob, Content, FileData, GenerateContentRequest, GenerateContentResponse, GenerationConfig,
    ImageConfig, Part, Role,
};
use async_trait::async_trait;

#[async_trait]
impl<M> ImageModel for Google<M>
where
    M: ModelName,
    Google<M>: ImageOutputSupport,
{
    async fn generate_image(&self, input: ImageModelOptions) -> Result<ImageModelResponse> {
        let warnings = google_warnings(&input);

        let mut parts = Vec::new();
        if let Some(prompt) = input.prompt.clone() {
            parts.push(Part {
                text: Some(prompt),
                ..Default::default()
            });
        }

        for file in &input.files {
            parts.push(user_image_to_google_part(file)?);
        }

        let request = GenerateContentRequest {
            contents: vec![Content {
                role: Role::User,
                parts,
            }],
            tools: None,
            tool_config: None,
            safety_settings: None,
            system_instruction: None,
            generation_config: Some(GenerationConfig {
                stop_sequences: None,
                response_mime_type: None,
                response_schema: None,
                candidate_count: input.n.map(|n| n as i32),
                max_output_tokens: None,
                temperature: None,
                top_p: None,
                top_k: None,
                presence_penalty: None,
                frequency_penalty: None,
                response_logprobs: None,
                logprobs: None,
                response_modalities: Some(vec!["IMAGE".to_string()]),
                image_config: input.aspect_ratio.clone().map(|aspect_ratio| ImageConfig {
                    aspect_ratio: Some(aspect_ratio),
                    image_size: None,
                }),
                seed: input.seed.map(|seed| seed as i32),
            }),
            cached_content: None,
        };

        let mut default_headers = reqwest::header::HeaderMap::new();
        default_headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        default_headers.insert("x-goog-api-key", self.settings.api_key.parse().unwrap());

        let headers = merge_headers(
            default_headers,
            self.settings.headers.as_ref(),
            input.headers.as_ref(),
        )?;
        let body = merge_body(&request, self.settings.body.as_ref(), input.body.as_ref())?;
        let path =
            self.settings.path.clone().unwrap_or_else(|| {
                format!("/v1beta/models/{}:generateContent", self.lm_options.model)
            });
        let url = join_url(&self.settings.base_url, &path)?;

        let response: GenerateContentResponse = reqwest::Client::new()
            .post(url)
            .headers(headers)
            .body(body)
            .send()
            .await
            .map_err(|error| Error::ApiError {
                status_code: error.status(),
                details: error.to_string(),
            })?
            .error_for_status()
            .map_err(|error| Error::ApiError {
                status_code: error.status(),
                details: error.to_string(),
            })?
            .json()
            .await
            .map_err(|error| Error::ApiError {
                status_code: None,
                details: format!("failed to parse google image response: {error}"),
            })?;

        let images = response
            .candidates
            .into_iter()
            .flat_map(|candidate| candidate.content.parts.into_iter())
            .filter_map(|part| {
                part.inline_data
                    .and_then(|data| data.mime_type.starts_with("image/").then_some(data.data))
            })
            .collect();

        Ok(ImageModelResponse {
            images,
            usage: response.usage_metadata.map(|usage| ImageModelUsage {
                input_tokens: Some(usage.prompt_token_count as usize),
                output_tokens: Some(usage.candidates_token_count as usize),
                total_tokens: Some(usage.total_token_count as usize),
            }),
            warnings,
        })
    }
}

fn google_warnings(input: &ImageModelOptions) -> Vec<ImageModelWarning> {
    let mut warnings = Vec::new();

    if input.size.is_some() {
        warnings.push(ImageModelWarning::Unsupported {
            feature: "size",
            details: Some("This model does not support size. Use aspect_ratio instead.".into()),
        });
    }

    warnings
}

fn user_image_to_google_part(image: &UserImage) -> Result<Part> {
    if let Some((mime_type, data)) = image.inline_data() {
        return Ok(Part {
            inline_data: Some(Blob { mime_type, data }),
            ..Default::default()
        });
    }

    let mime_type = image.resolved_media_type().ok_or_else(|| {
        Error::InvalidInput(
            "input image requires an explicit media type or a URL with a known image extension"
                .to_string(),
        )
    })?;

    Ok(Part {
        file_data: Some(FileData {
            mime_type,
            file_uri: image.image_url.clone(),
        }),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{DynamicModel, ImageModelRequest};
    use serde_json::json;
    use std::collections::HashMap;
    use wiremock::matchers::{body_partial_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_model(base_url: String) -> Google<DynamicModel> {
        let mut model = Google::model_name("gemini-2.5-flash-image");
        model.settings.base_url = base_url;
        model.settings.api_key = "test-key".to_string();
        model
    }

    fn image_response() -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(json!({
            "candidates": [
                {
                    "content": {
                        "role": "model",
                        "parts": [
                            {
                                "inlineData": {
                                    "mimeType": "image/png",
                                    "data": "Zm9v"
                                }
                            }
                        ]
                    }
                }
            ],
            "usageMetadata": {
                "promptTokenCount": 5,
                "candidatesTokenCount": 7,
                "totalTokenCount": 12
            }
        }))
    }

    #[tokio::test]
    async fn test_generate_image_sends_google_image_request() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path(
                "/v1beta/models/gemini-2.5-flash-image:generateContent",
            ))
            .and(header("x-goog-api-key", "test-key"))
            .and(header("x-trace-id", "google-image-123"))
            .and(body_partial_json(json!({
                "contents": [
                    {
                        "role": "user",
                        "parts": [
                            { "text": "draw a watercolor whale" }
                        ]
                    }
                ],
                "generationConfig": {
                    "candidateCount": 2,
                    "responseModalities": ["IMAGE"],
                    "imageConfig": {
                        "aspectRatio": "16:9"
                    },
                    "seed": 42
                }
            })))
            .respond_with(image_response())
            .expect(1)
            .mount(&server)
            .await;

        let response = ImageModelRequest::builder()
            .model(test_model(server.uri()))
            .prompt("draw a watercolor whale")
            .n(2)
            .aspect_ratio("16:9")
            .seed(42u32)
            .headers(HashMap::from([(
                "x-trace-id".to_string(),
                "google-image-123".to_string(),
            )]))
            .build()
            .generate_image()
            .await
            .expect("google image generation should succeed");

        assert_eq!(response.images, vec!["Zm9v".to_string()]);
        assert_eq!(response.usage.unwrap().total_tokens, Some(12));
    }

    #[tokio::test]
    async fn test_edit_image_sends_input_parts() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path(
                "/v1beta/models/gemini-2.5-flash-image:generateContent",
            ))
            .and(body_partial_json(json!({
                "contents": [
                    {
                        "role": "user",
                        "parts": [
                            { "text": "make this poster more cinematic" },
                            {
                                "fileData": {
                                    "mimeType": "image/png",
                                    "fileUri": "https://example.com/poster.png"
                                }
                            }
                        ]
                    }
                ],
                "generationConfig": {
                    "responseModalities": ["IMAGE"]
                }
            })))
            .respond_with(image_response())
            .expect(1)
            .mount(&server)
            .await;

        let response = ImageModelRequest::builder()
            .model(test_model(server.uri()))
            .prompt("make this poster more cinematic")
            .file(UserImage::new("https://example.com/poster.png").media_type("image/png"))
            .build()
            .generate_image()
            .await
            .expect("google image edit should succeed");

        assert_eq!(response.images, vec!["Zm9v".to_string()]);
    }

    #[tokio::test]
    async fn test_generate_image_returns_size_warning() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path(
                "/v1beta/models/gemini-2.5-flash-image:generateContent",
            ))
            .respond_with(image_response())
            .expect(1)
            .mount(&server)
            .await;

        let response = ImageModelRequest::builder()
            .model(test_model(server.uri()))
            .prompt("draw a fox")
            .size("1024x1024")
            .build()
            .generate_image()
            .await
            .expect("google image generation should succeed");

        assert_eq!(response.warnings.len(), 1);
    }
}
