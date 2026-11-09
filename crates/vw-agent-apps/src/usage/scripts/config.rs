//! Selects native or JavaScript usage queries before execution.

use std::fmt::{self, Write};

use serde::{Deserialize, Serialize};

use crate::usage::detect::NativeUsageCapability;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageTemplate {
    Custom,
    General,
    #[serde(rename = "newapi")]
    NewApi,
    #[serde(rename = "sub2api")]
    Sub2Api,
    #[serde(rename = "github_copilot")]
    GitHubCopilot,
    #[serde(rename = "token_plan")]
    CodingPlan,
    Balance,
    OfficialSubscription,
}

/// Account usage resolved from the provider's trusted identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountUsage {
    GitHubCopilot,
    OfficialSubscription,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeTemplate {
    GitHubCopilot,
    CodingPlan,
    Balance,
    OfficialSubscription,
}

/// Script-specific inputs already resolved on the authorized host.
#[derive(Clone, Copy, Default)]
pub struct ScriptParameters<'a> {
    pub api_key: Option<&'a str>,
    pub base_url: Option<&'a str>,
    pub access_token: Option<&'a str>,
    pub user_id: Option<&'a str>,
}

/// Fallback values from the authorized provider record.
#[derive(Clone, Copy, Default)]
pub struct ProviderDefaults<'a> {
    pub api_key: Option<&'a str>,
    pub base_url: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptParameter {
    ApiKey,
    BaseUrl,
    AccessToken,
    UserId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateError {
    EmptySource,
    MissingParameter(ScriptParameter),
}

impl fmt::Display for TemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySource => f.write_str("usage script source is empty"),
            Self::MissingParameter(parameter) => {
                write!(f, "missing usage script parameter: {parameter:?}")
            }
        }
    }
}

impl std::error::Error for TemplateError {}

pub enum PreparedUsage<'a> {
    Native(NativeTemplate),
    Script(PreparedUsageScript<'a>),
}

/// The source and base URL can contain credentials. Keep them on the execution host.
pub struct PreparedUsageScript<'a> {
    template: UsageTemplate,
    source: String,
    base_url: Option<&'a str>,
    #[cfg(target_os = "macos")]
    pub(super) credentials: [Option<&'a str>; 2],
}

impl PreparedUsageScript<'_> {
    pub fn template(&self) -> UsageTemplate {
        self.template
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn base_url(&self) -> Option<&str> {
        self.base_url
    }
}

impl UsageTemplate {
    /// Keep native account queries on their dedicated path, then honor an explicit
    /// template. Legacy New API fields take precedence over URL-based detection.
    pub fn select(
        saved: Option<Self>,
        account: Option<AccountUsage>,
        detected: Option<NativeUsageCapability>,
        has_legacy_new_api_parameters: bool,
    ) -> Self {
        match account {
            Some(AccountUsage::GitHubCopilot) => return Self::GitHubCopilot,
            Some(AccountUsage::OfficialSubscription) => return Self::OfficialSubscription,
            None => {}
        }
        if let Some(saved) = saved {
            return saved;
        }
        if has_legacy_new_api_parameters {
            return Self::NewApi;
        }
        match detected {
            Some(NativeUsageCapability::Balance(_)) => Self::Balance,
            Some(NativeUsageCapability::CodingPlan(_)) => Self::CodingPlan,
            None => Self::General,
        }
    }

    /// Saved source can customize any script template. Native templates bypass
    /// source and parameter handling entirely.
    pub fn prepare<'a>(
        self,
        saved_source: Option<&str>,
        parameters: ScriptParameters<'a>,
        provider: ProviderDefaults<'a>,
    ) -> Result<PreparedUsage<'a>, TemplateError> {
        let preset = match self {
            Self::Custom => CUSTOM,
            Self::General => GENERAL,
            Self::NewApi => NEW_API,
            Self::Sub2Api => SUB2API,
            Self::GitHubCopilot => return Ok(PreparedUsage::Native(NativeTemplate::GitHubCopilot)),
            Self::CodingPlan => return Ok(PreparedUsage::Native(NativeTemplate::CodingPlan)),
            Self::Balance => return Ok(PreparedUsage::Native(NativeTemplate::Balance)),
            Self::OfficialSubscription => {
                return Ok(PreparedUsage::Native(NativeTemplate::OfficialSubscription));
            }
        };
        let source = saved_source.unwrap_or(preset);
        if source.trim().is_empty() {
            return Err(TemplateError::EmptySource);
        }

        let api_key = trimmed(parameters.api_key).or_else(|| trimmed(provider.api_key));
        let base_url = normalized_base_url(parameters.base_url)
            .or_else(|| normalized_base_url(provider.base_url));
        let access_token = trimmed(parameters.access_token);
        let values = [
            ("apiKey}}", ScriptParameter::ApiKey, api_key),
            ("baseUrl}}", ScriptParameter::BaseUrl, base_url),
            ("accessToken}}", ScriptParameter::AccessToken, access_token),
            ("userId}}", ScriptParameter::UserId, trimmed(parameters.user_id)),
        ];

        let mut rendered = String::with_capacity(source.len());
        let mut remaining = source;
        while let Some((prefix, suffix)) = remaining.split_once("{{") {
            rendered.push_str(prefix);
            if let Some((tail, parameter, value)) =
                values.iter().find_map(|&(marker, parameter, value)| {
                    suffix.strip_prefix(marker).map(|tail| (tail, parameter, value))
                })
            {
                let value = value.ok_or(TemplateError::MissingParameter(parameter))?;
                // The same escapes work in single quotes, double quotes and template
                // literals. No value can introduce a delimiter or a ${...} expression.
                for unit in value.encode_utf16() {
                    write!(rendered, "\\u{unit:04x}").expect("writing to String cannot fail");
                }
                remaining = tail;
            } else {
                // Preserve arbitrary JavaScript, including nested blocks.
                rendered.push('{');
                remaining = &remaining[prefix.len() + 1..];
            }
        }
        rendered.push_str(remaining);
        Ok(PreparedUsage::Script(PreparedUsageScript {
            template: self,
            source: rendered,
            base_url,
            #[cfg(target_os = "macos")]
            credentials: [api_key, access_token],
        }))
    }
}

fn trimmed(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn normalized_base_url(value: Option<&str>) -> Option<&str> {
    trimmed(value).map(|url| url.trim_end_matches('/')).filter(|url| !url.is_empty())
}

const CUSTOM: &str = r#"({
  request: {
    url: "",
    method: "GET",
    headers: {}
  },
  extractor: function(response) {
    return { remaining: 0, unit: "USD" };
  }
})"#;

const GENERAL: &str = r#"({
  request: {
    url: "{{baseUrl}}/user/balance",
    method: "GET",
    headers: {
      "Authorization": "Bearer {{apiKey}}",
      "User-Agent": "cc-switch/1.0"
    }
  },
  extractor: function(response) {
    return {
      isValid: response.is_active || true,
      remaining: response.balance,
      unit: "USD"
    };
  }
})"#;

const NEW_API: &str = r#"({
  request: {
    url: "{{baseUrl}}/api/user/self",
    method: "GET",
    headers: {
      "Content-Type": "application/json",
      "Authorization": "Bearer {{accessToken}}",
      "User-Agent": "cc-switch/1.0",
      "New-Api-User": "{{userId}}"
    }
  },
  extractor: function(response) {
    if (response.success && response.data) {
      return {
        planName: response.data.group || "Default Plan",
        remaining: response.data.quota / 500000,
        used: response.data.used_quota / 500000,
        total: (response.data.quota + response.data.used_quota) / 500000,
        unit: "USD"
      };
    }
    return {
      isValid: false,
      invalidMessage: response.message || "Query failed"
    };
  }
})"#;

const SUB2API: &str = r#"({
  request: {
    url: "{{baseUrl}}".replace(/\/+$/, "").replace(/\/v1$/, "") + "/v1/usage",
    method: "GET",
    headers: {
      "Authorization": "Bearer {{apiKey}}",
      "User-Agent": "cc-switch/1.0"
    }
  },
  extractor: function(response) {
    var quota = response.quota || {};
    var result = {
      isValid: response.isValid !== false,
      unit: response.unit || quota.unit || "USD"
    };
    if (typeof response.planName === "string") result.planName = response.planName;
    if (typeof response.remaining === "number" && response.remaining >= 0) {
      result.remaining = response.remaining;
    }
    if (quota.limit > 0) {
      result.used = quota.used;
      result.total = quota.limit;
    }
    if (!result.isValid) {
      result.invalidMessage = response.status ? String(response.status) : "Query failed";
    }
    return result;
  }
})"#;
