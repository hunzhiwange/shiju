//! Provider-family selection. Credentials are supplied only by the authorized owner.
use axum::http::{HeaderMap, HeaderValue, StatusCode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderType {
    Claude,
    ClaudeAuth,
    Codex,
    Gemini,
    GeminiCli,
    OpenRouter,
    GitHubCopilot,
    CodexOAuth,
    XaiOAuth,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Adapter {
    Claude,
    Codex,
    Gemini,
}

pub fn adapter(provider: ProviderType) -> Adapter {
    match provider {
        ProviderType::Codex => Adapter::Codex,
        ProviderType::Gemini | ProviderType::GeminiCli => Adapter::Gemini,
        ProviderType::Claude
        | ProviderType::ClaudeAuth
        | ProviderType::OpenRouter
        | ProviderType::GitHubCopilot
        | ProviderType::CodexOAuth
        | ProviderType::XaiOAuth => Adapter::Claude,
    }
}

/// Build auth without ever copying a raw invalid credential into an error.
pub fn auth_headers(provider: ProviderType, token: &str) -> Result<HeaderMap, StatusCode> {
    if token.is_empty() || token.len() > 16384 || !token.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let (name, value) = match provider {
        ProviderType::Claude => ("x-api-key", token.to_owned()),
        ProviderType::Gemini => ("x-goog-api-key", token.to_owned()),
        ProviderType::ClaudeAuth
        | ProviderType::Codex
        | ProviderType::GeminiCli
        | ProviderType::OpenRouter
        | ProviderType::GitHubCopilot
        | ProviderType::CodexOAuth
        | ProviderType::XaiOAuth => ("authorization", format!("Bearer {token}")),
    };
    let mut value = HeaderValue::from_str(&value).map_err(|_| StatusCode::UNAUTHORIZED)?;
    value.set_sensitive(true);
    let mut headers = HeaderMap::new();
    headers.insert(axum::http::HeaderName::from_static(name), value);
    if provider == ProviderType::Claude {
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
    }
    if provider == ProviderType::GeminiCli {
        headers.insert("x-goog-api-client", HeaderValue::from_static("GeminiCLI/1.0"));
    }
    if provider == ProviderType::CodexOAuth {
        headers.insert("originator", HeaderValue::from_static("codex_cli_rs"));
        headers.insert("version", HeaderValue::from_static("0.159.0"));
    }
    if provider == ProviderType::GitHubCopilot {
        for (name, value) in [
            ("editor-version", "vscode/1.110.1"),
            ("editor-plugin-version", "copilot-chat/0.38.2"),
            ("copilot-integration-id", "vscode-chat"),
            ("user-agent", "GitHubCopilotChat/0.38.2"),
            ("x-github-api-version", "2025-10-01"),
            ("openai-intent", "conversation-agent"),
            ("x-initiator", "user"),
            ("x-interaction-type", "conversation-agent"),
            ("x-vscode-user-agent-library-version", "electron-fetch"),
        ] {
            headers
                .insert(axum::http::HeaderName::from_static(name), HeaderValue::from_static(value));
        }
        headers.insert(
            "x-request-id",
            HeaderValue::from_str(&uuid::Uuid::new_v4().to_string())
                .map_err(|_| StatusCode::UNAUTHORIZED)?,
        );
    }
    if let Some(request_id) = headers.get("x-request-id").cloned() {
        headers.insert("x-agent-task-id", request_id);
    }
    Ok(headers)
}

/// Resolve only the existing owner-scoped grant on each invocation. Never logs in.
pub async fn gemini_auth_headers(
    store: &crate::agent_apps::auth_accounts::AuthAccountStore,
    refresh: &vw_agent_apps::auth::gemini_refresh::GeminiRefresh,
    scope: vw_agent_apps::auth::gemini_refresh::GeminiScope<'_>,
    account: &str,
) -> Result<HeaderMap, vw_api_types::agent_apps_auth::AuthErrorCode> {
    let token = store.gemini_access_token(refresh, scope, account).await?;
    auth_headers(ProviderType::GeminiCli, &token)
        .map_err(|_| vw_api_types::agent_apps_auth::AuthErrorCode::ReauthRequired)
}

/// Resolve a target without accepting credential-bearing or ambiguous URLs.
pub fn target_url(
    provider: ProviderType,
    base: Option<&str>,
    endpoint: &str,
) -> Result<reqwest::Url, StatusCode> {
    let default = match provider {
        ProviderType::Claude | ProviderType::ClaudeAuth => "https://api.anthropic.com",
        ProviderType::Codex => "https://api.openai.com",
        ProviderType::Gemini | ProviderType::GeminiCli => {
            "https://generativelanguage.googleapis.com"
        }
        ProviderType::OpenRouter => "https://openrouter.ai/api",
        ProviderType::GitHubCopilot => "https://api.githubcopilot.com",
        ProviderType::CodexOAuth => "https://chatgpt.com/backend-api/codex",
        ProviderType::XaiOAuth => "https://api.x.ai",
    };
    let mut url = reqwest::Url::parse(base.unwrap_or(default))
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    if !matches!(url.scheme(), "https" | "http")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !endpoint.starts_with('/')
        || endpoint.starts_with("//")
        || endpoint.contains(['?', '#', '\\'])
        || endpoint.chars().any(char::is_control)
        || endpoint.split('/').any(|part| part == "." || part == ".." || part.contains('%'))
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let path = url.path().trim_end_matches('/');
    let endpoint = if path.ends_with("/v1") && endpoint.starts_with("/v1/") {
        &endpoint[3..]
    } else {
        endpoint
    };
    if !path.ends_with(endpoint) {
        url.set_path(&format!("{path}{endpoint}"));
    }
    Ok(url)
}

/// The destination and credential are resolved together for each managed request.
pub struct SelectedTarget {
    pub url: reqwest::Url,
    pub headers: HeaderMap,
    pub adapter: Adapter,
}

/// Reuse the host's authenticated clients and their account-scoped caches.
pub enum AccountAdapter {
    Gemini(std::sync::Arc<vw_agent_apps::auth::gemini_refresh::GeminiRefresh>),
    Codex(std::sync::Arc<vw_agent_apps::auth::codex_refresh::CodexRefresh>),
    Copilot(std::sync::Arc<vw_agent_apps::auth::copilot::CopilotAuth>),
    Xai(std::sync::Arc<vw_agent_apps::auth::xai_refresh::XaiRefresh>),
}
impl AccountAdapter {
    /// Initialize once when publishing a target, never for every request.
    pub fn new(
        provider: ProviderType,
    ) -> Result<Self, vw_api_types::agent_apps_auth::AuthErrorCode> {
        use std::sync::Arc;
        use vw_agent_apps::auth::{
            codex_refresh::CodexRefresh, copilot::CopilotAuth, gemini_refresh::GeminiRefresh,
            xai_refresh::XaiRefresh,
        };
        Ok(match provider {
            ProviderType::GeminiCli => Self::Gemini(Arc::new(GeminiRefresh::new()?)),
            ProviderType::CodexOAuth => Self::Codex(Arc::new(CodexRefresh::new()?)),
            ProviderType::GitHubCopilot => Self::Copilot(Arc::new(CopilotAuth::new()?)),
            ProviderType::XaiOAuth => Self::Xai(Arc::new(XaiRefresh::new()?)),
            ProviderType::Claude
            | ProviderType::ClaudeAuth
            | ProviderType::Codex
            | ProviderType::Gemini
            | ProviderType::OpenRouter => {
                return Err(vw_api_types::agent_apps_auth::AuthErrorCode::Unsupported);
            }
        })
    }
    pub fn provider_type(&self) -> ProviderType {
        match self {
            Self::Gemini(_) => ProviderType::GeminiCli,
            Self::Codex(_) => ProviderType::CodexOAuth,
            Self::Copilot(_) => ProviderType::GitHubCopilot,
            Self::Xai(_) => ProviderType::XaiOAuth,
        }
    }
}

pub async fn managed_target(
    owner: &crate::agent_apps::auth_accounts::AuthAccountStore,
    account_adapter: &AccountAdapter,
    scope: vw_agent_apps::auth::gemini_refresh::GeminiScope<'_>,
    account: &str,
    endpoint: &str,
) -> Result<SelectedTarget, vw_api_types::agent_apps_auth::AuthErrorCode> {
    use vw_agent_apps::auth::flow::AuthAccountScope;
    use vw_api_types::agent_apps_auth::{AuthErrorCode, AuthService};
    let provider = account_adapter.provider_type();
    let auth_scope = |service| AuthAccountScope {
        tenant: scope.tenant,
        principal: scope.principal,
        host: scope.host,
        service,
    };
    let (headers, base) = match account_adapter {
        AccountAdapter::Gemini(client) => {
            (gemini_auth_headers(owner, client, scope, account).await?, None)
        }
        AccountAdapter::Codex(client) => {
            let token = owner
                .codex_access_token(client, auth_scope(AuthService::CodexOauth), account)
                .await?;
            let lease =
                owner.managed_official_lease(auth_scope(AuthService::CodexOauth), account)?;
            let vw_agent_apps::auth::credential_file::AccountCredential::CodexAuth(material) =
                lease.read().map_err(|_| AuthErrorCode::StorageUnavailable)?
            else {
                return Err(AuthErrorCode::ReauthRequired);
            };
            let account_id = material
                .pointer("/tokens/account_id")
                .and_then(serde_json::Value::as_str)
                .ok_or(AuthErrorCode::ReauthRequired)?;
            let mut value =
                HeaderValue::from_str(account_id).map_err(|_| AuthErrorCode::ReauthRequired)?;
            value.set_sensitive(true);
            let mut headers =
                auth_headers(provider, &token).map_err(|_| AuthErrorCode::ReauthRequired)?;
            headers.insert("chatgpt-account-id", value);
            (headers, None)
        }
        AccountAdapter::Xai(client) => {
            let token =
                owner.xai_access_token(client, auth_scope(AuthService::XaiOauth), account).await?;
            (auth_headers(provider, &token).map_err(|_| AuthErrorCode::ReauthRequired)?, None)
        }
        AccountAdapter::Copilot(client) => {
            let session = owner
                .copilot_session(client, auth_scope(AuthService::GithubCopilot), account)
                .await?;
            let token = session
                .authorization
                .to_str()
                .map_err(|_| AuthErrorCode::ReauthRequired)?
                .strip_prefix("Bearer ")
                .ok_or(AuthErrorCode::ReauthRequired)?;
            (
                auth_headers(provider, token).map_err(|_| AuthErrorCode::ReauthRequired)?,
                Some(session.endpoint),
            )
        }
    };
    Ok(SelectedTarget {
        url: target_url(provider, base.as_ref().map(reqwest::Url::as_str), endpoint)
            .map_err(|_| AuthErrorCode::InvalidInput)?,
        headers,
        adapter: adapter(provider),
    })
}

/// Publication proof supplied by the authorized configuration transaction.
pub struct ManagedCredential {
    pub owner: std::sync::Arc<crate::agent_apps::auth_accounts::AuthAccountStore>,
    pub account_adapter: AccountAdapter,
    pub tenant: vw_api_types::id::TenantScope,
    pub principal: String,
    pub host: String,
    pub account: String,
    pub endpoint: String,
    pub base: Option<reqwest::Url>,
}
impl ManagedCredential {
    pub fn resolve(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<SelectedTarget, vw_api_types::agent_apps_auth::AuthErrorCode>,
                > + Send
                + '_,
        >,
    > {
        // Keep account exchange state out of the connection's concrete future.
        Box::pin(async move {
            let provider = self.account_adapter.provider_type();
            if provider == ProviderType::CodexOAuth {
                let url = target_url(provider, self.base.as_ref().map(reqwest::Url::as_str), &self.endpoint)
                    .map_err(|_| vw_api_types::agent_apps_auth::AuthErrorCode::InvalidInput)?;
                crate::model_relay::codex_oauth_transport::validate_url(&url)
                    .map_err(|_| vw_api_types::agent_apps_auth::AuthErrorCode::InvalidInput)?;
            }
            if provider == ProviderType::XaiOAuth {
                let url = target_url(provider, self.base.as_ref().map(reqwest::Url::as_str), &self.endpoint)
                    .map_err(|_| vw_api_types::agent_apps_auth::AuthErrorCode::InvalidInput)?;
                crate::model_relay::xai_oauth_transport::validate_url(&url)
                    .map_err(|_| vw_api_types::agent_apps_auth::AuthErrorCode::InvalidInput)?;
            }
            let mut selected = managed_target(
                &self.owner,
                &self.account_adapter,
                vw_agent_apps::auth::gemini_refresh::GeminiScope {
                    tenant: &self.tenant,
                    principal: &self.principal,
                    host: &self.host,
                },
                &self.account,
                &self.endpoint,
            )
            .await?;
            if let Some(base) = &self.base {
                let url = target_url(provider, Some(base.as_str()), &self.endpoint)
                    .map_err(|_| vw_api_types::agent_apps_auth::AuthErrorCode::InvalidInput)?;
                if provider == ProviderType::GitHubCopilot && url != selected.url {
                    return Err(vw_api_types::agent_apps_auth::AuthErrorCode::InvalidInput);
                }
                selected.url = url;
            }
            Ok(selected)
        })
    }
}
