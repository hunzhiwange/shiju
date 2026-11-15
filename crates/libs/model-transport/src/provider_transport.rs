//! Provider request orchestration after the host validates its current relay scope.
use agent_provider::provider;
use agent_auth as auth;
use axum::{body::Body, http::StatusCode, response::Response};
use tokio_util::sync::CancellationToken;
use model_relay_state::{RelayIdentity, References};
use model_protocols::ModelProtocol as P;
use provider_protocol_routes::RouteCapability as C;
use model_protocol::endpoint_urls::{official_deepseek_messages_url, official_alibaba_messages_url};

pub struct ProviderTransport {
    client: reqwest::Client,
    references: std::sync::Arc<References>,
}
impl ProviderTransport {
    pub fn new() -> Result<Self, reqwest::Error> {
        Ok(Self { client: crate::upstream::upstream_http_client()?, references: Default::default() })
    }
    pub async fn execute(&self, identity: &RelayIdentity, path: &str, body: &[u8], cancel: CancellationToken) -> Result<Response, StatusCode> {
        if !matches!(
            path,
            "/v1/chat/completions"
                | "/v1/messages"
                | "/v1/responses"
                | "/v1beta/generateContent"
                | "/v1beta/streamGenerateContent"
                | "/v1beta/models"
                | "/v1beta/model"
        ) {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let info = provider::get_provider(&identity.provider)
            .await
            .map_err(|_| StatusCode::FORBIDDEN)?
            .ok_or(StatusCode::FORBIDDEN)?;
        // Exact catalog key required: API-id alias lookup must not authorize another model.
        let model = info.models.get(&identity.model).ok_or(StatusCode::FORBIDDEN)?;
        if info.id != identity.provider || model.provider_id != identity.provider {
            return Err(StatusCode::FORBIDDEN);
        }
        let gemini_request = path.starts_with("/v1beta/");
        let gemini_streaming = path == "/v1beta/streamGenerateContent";

        if matches!(path, "/v1beta/models" | "/v1beta/model") {
            let capabilities = [C::Text, C::Streaming];
            let conversions = crate::routing_selection::relay_conversions(false);
            let route = crate::routing_selection::select(
                model,
                P::Gemini,
                &capabilities,
                &conversions,
                crate::routing_selection::Execution::Relay,
            )?;
            if !matches!(
                route.endpoint.authentication,
                model_protocols::ProtocolAuthentication::None {}
            ) {
                bearer(&info)?;
            }
            let metadata = serde_json::json!({"name":format!("models/{}",identity.model),"baseModelId":identity.model,"version":"relay","displayName":identity.model,"inputTokenLimit":model.limit.input.unwrap_or(model.limit.context),"outputTokenLimit":model.limit.output,"supportedGenerationMethods":["generateContent","streamGenerateContent"]});
            let output = if path == "/v1beta/models" {
                serde_json::json!({"models":[metadata]})
            } else {
                metadata
            };
            return Response::builder()
                .header("content-type", "application/json")
                .body(Body::from(output.to_string()))
                .map_err(|_| StatusCode::BAD_GATEWAY);
        }
        let messages_request = path == "/v1/messages";
        let responses_request = path == "/v1/responses";
        let source = if gemini_request {
            P::Gemini
        } else if messages_request {
            P::Messages
        } else if responses_request {
            P::Responses
        } else {
            P::Chat
        };
        let input: serde_json::Value =
            serde_json::from_slice(body).map_err(|_| StatusCode::BAD_REQUEST)?;
        let streaming_requested =
            if gemini_request { gemini_streaming } else { input["stream"] == true };
        let required = crate::routing_selection::required(&input, streaming_requested);
        let conversions = crate::routing_selection::relay_conversions(identity.session.ends_with(":codex"));
        let route = crate::routing_selection::select(
            model,
            source,
            &required,
            &conversions,
            crate::routing_selection::Execution::Relay,
        )?;
        let native_gemini = source == P::Gemini && !route.conversion_required;
        let native_responses = source == P::Responses && !route.conversion_required;
        let native = source == P::Messages && !route.conversion_required;
        let key = if matches!(
            route.endpoint.authentication,
            model_protocols::ProtocolAuthentication::None {}
        ) {
            String::new()
        } else {
            bearer(&info)?
        };
        let thinking = (messages_request
            && !native
            && route.target == P::Chat
            && model.capabilities.reasoning)
            .then(|| {
                // Bind receipts to the actual deployment and current credential without retaining it.
                let credential =
                    hex::encode(ring::digest::digest(&ring::digest::SHA256, key.as_bytes()));
                let deployment = serde_json::json!([
                    model.api.id,
                    model.api.url,
                    route.endpoint.url,
                    route.endpoint.deployment,
                    credential,
                    model.headers.iter().collect::<std::collections::BTreeMap<_, _>>(),
                    route.endpoint.authentication
                ])
                .to_string();
                model_protocol::messages_thinking::Context::new(
                    identity,
                    deployment,
                    self.references.clone(),
                )
            });
        let codex_chat =
            responses_request && !native_responses && identity.session.ends_with(":codex");
        let mut response_tools = model_protocol::responses::ChatTools::default();
        let mut body = if gemini_request {
            let input =
                serde_json::from_slice(body).map_err(|_| StatusCode::BAD_REQUEST)?;
            if native_gemini {
                model_protocol::gemini::native(&input, model)?
            } else {
                let converted = model_protocol::gemini::to_chat(&input)?;
                model_protocol::chat::prepare(
                    model,
                    &identity.model,
                    &serde_json::to_vec(&converted).map_err(|_| StatusCode::BAD_REQUEST)?,
                )?
            }
        } else if native_responses {
            model_protocol::responses::native(model, identity, body, &self.references)?
        } else if responses_request {
            let mut input =
                serde_json::from_slice(body).map_err(|_| StatusCode::BAD_REQUEST)?;
            let effort = if codex_chat {
                model_protocol::responses::codex_chat_effort(&mut input)?
            } else {
                model_protocol::responses::chat_effort(&mut input, model)?
            };
            let (mut converted, tools) = model_protocol::responses::to_chat(&input, codex_chat)?;
            response_tools = tools;
            // Only the Codex Responses -> Chat bridge needs Moonshot's draft-07 form.
            // Gate on the resolved endpoint, including any deployment URL override.
            if codex_chat
                && route.target == P::Chat
                && vw_agent_apps::routing::moonshot_schema::upstream_requires_ref_sibling_all_of(
                    &route.endpoint.url,
                )
            {
                vw_agent_apps::routing::moonshot_schema::wrap_ref_siblings_in_chat_tools(
                    &mut converted,
                );
            }
            if route.target == P::Chat && (codex_chat || effort.is_some()) {
                model_protocol::responses::configure_codex_chat(&mut converted, model, effort.as_deref());
            }
            model_protocol::chat::prepare(
                model,
                &identity.model,
                &serde_json::to_vec(&converted).map_err(|_| StatusCode::BAD_REQUEST)?,
            )?
        } else if native {
            let mut body = model_protocol::messages::native(model, &identity.model, body)?;
            if official_deepseek_messages_url(&model.api).is_some() {
                model_protocol::messages::prepare_deepseek_native(&mut body)?;
            } else if official_alibaba_messages_url(&model.api).is_some() {
                model_protocol::messages::prepare_hosted_native(&mut body)?;
            }
            body
        } else if messages_request {
            let input =
                serde_json::from_slice(body).map_err(|_| StatusCode::BAD_REQUEST)?;
            let converted = match &thinking {
                Some(context) => context.request(&input)?,
                None => model_protocol::messages::to_chat(&input)?,
            };
            model_protocol::chat::prepare(
                model,
                &identity.model,
                &serde_json::to_vec(&converted).map_err(|_| StatusCode::BAD_REQUEST)?,
            )?
        } else {
            model_protocol::chat::prepare(model, &identity.model, body)?
        };
        let streaming = streaming_requested;
        let mut url = reqwest::Url::parse(&route.endpoint.url)
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        if route.target == P::Gemini && streaming {
            let path = url
                .path()
                .strip_suffix(":generateContent")
                .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
            let path = format!("{path}:streamGenerateContent");
            url.set_path(&path);
            url.query_pairs_mut().append_pair("alt", "sse");
        }
        if messages_request && !native && route.target == P::Chat {
            model_protocol::messages_thinking::configure(&mut body, &input, model)?;
        }
        // This conversion carries text and tools, not replayable thinking. A missing
        // Gemini thinkingConfig must not enable a Chat deployment's default reasoning.
        // to_chat already rejects explicit thinking requests that cannot be preserved.
        if gemini_request && !native_gemini && route.target == P::Chat {
            model_protocol::messages_thinking::configure(
                &mut body,
                &serde_json::json!({"thinking":{"type":"disabled"}}),
                model,
            )?;
        }
        if route.conversion_required && route.target != P::Chat {
            if (messages_request && input["thinking"]["type"] == "disabled")
                || (responses_request && input["reasoning"]["effort"] == "none")
                || (gemini_request && model_protocol::gemini::thinking_disabled(&input))
            {
                body["reasoning_effort"] = "none".into();
            }
            body["stream"] = streaming.into();
            if route.target == P::Messages
                && body.get("max_tokens").is_none()
                && body.get("max_completion_tokens").is_none()
            {
                if model.limit.output == 0 {
                    return Err(StatusCode::UNPROCESSABLE_ENTITY);
                }
                body["max_tokens"] = model.limit.output.into();
            }
            body = model_protocol::protocol_bridge::request(route.target, &body)?;
        }
        let headers = model_protocol::generation::endpoint_headers(&route.endpoint, &key)?;
        let response = crate::execution::send(
            &self.client, url, headers, &body, &model.headers, &cancel,
        ).await?;
        let response = if route.conversion_required && route.target != P::Chat {
            model_protocol::protocol_bridge_stream::response(
                response,
                route.target,
                identity.model.clone(),
                streaming,
                cancel.clone(),
            )
            .await?
        } else {
            response
        };
        if gemini_request {
            return model_protocol::gemini_stream::response(
                response,
                identity.model.clone(),
                streaming,
                native_gemini,
                cancel,
            )
            .await;
        }
        if responses_request {
            return model_protocol::responses_stream::response(
                response,
                model_protocol::responses_stream::Output {
                    identity: identity.clone(),
                    native: native_responses,
                    codex_chat,
                    tool_names: response_tools,
                    streaming,
                    store: native_responses && body["store"] != false,
                    references: self.references.clone(),
                },
                cancel,
            )
            .await;
        }
        if messages_request {
            if let Some(context) = thinking {
                return model_protocol::messages_thinking::response(
                    response,
                    context,
                    &identity.model,
                    streaming,
                    cancel,
                )
                .await;
            }
            return model_protocol::messages_stream::response(
                response,
                identity.model.clone(),
                streaming,
                native,
                cancel,
            )
            .await;
        }
        crate::execution::chat_response(response, streaming, cancel)
    }
}

fn bearer(info: &provider::Info) -> Result<String, StatusCode> {
    crate::credential::credential(info, auth::get(&info.id))
}
