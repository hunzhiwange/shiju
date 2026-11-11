//! Deployment protocol facts. These declarations never own credentials or grant access.

use serde::{Deserialize, Deserializer, Serialize, de::Error};

/// Wire protocol shared by catalog selection, relay transport and diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelProtocol {
    Chat,
    Messages,
    Responses,
    Gemini,
}

/// Authentication shape only; the selected deployment retains credential ownership.
/// Unsupported schemes must be rejected by consumers, never treated as bearer keys.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProtocolAuthentication {
    None {},
    Bearer {},
    ApiKeyHeader {
        name: String,
    },
    #[serde(rename = "oauth_bearer")]
    OAuthBearer {},
    AwsSigv4 {},
    Unsupported {
        reason: String,
    },
}

/// Exact upstream model IDs, without alias, prefix or canonical-name inference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProtocolModelScope {
    All {},
    Models { ids: Vec<String> },
}

impl ProtocolModelScope {
    /// Whether the evidence explicitly covers this deployment's model ID.
    pub fn contains(&self, model: &str) -> bool {
        match self {
            Self::All {} => true,
            Self::Models { ids } => ids.iter().any(|id| id == model),
        }
    }

    fn overlaps(&self, other: &Self) -> bool {
        match self {
            Self::All {} => true,
            Self::Models { ids } => ids.iter().any(|id| other.contains(id)),
        }
    }
}

/// Official documentation, successful account execution, and explicit configuration
/// remain distinct signals. None implies the other two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolEvidenceKind {
    Official,
    Live,
    Configuration,
}

/// A reference into an auditable report (or explicit user configuration).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolEvidence {
    pub kind: ProtocolEvidenceKind,
    pub source: String,
    pub reference: String,
    pub checked_at: String,
}

/// One evidenced endpoint within a single provider/deployment/account boundary.
/// `url` is the actual operation URL, not an inferred base URL. Unresolved deployment
/// templates are facts only and must be resolved and validated before transport.
/// `deployment` identifies region/plan/credential scope; equal model names across
/// deployments never authorize credential transfer. Cross-origin use still requires
/// the existing binding/access checks and explicit service evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolEndpoint {
    pub protocol: ModelProtocol,
    pub url: String,
    pub deployment: String,
    pub authentication: ProtocolAuthentication,
    pub models: ProtocolModelScope,
    pub evidence: Vec<ProtocolEvidence>,
}

impl ProtocolEndpoint {
    fn validate(&self) -> Result<(), &'static str> {
        if self.url.trim().is_empty() || self.deployment.trim().is_empty() {
            return Err("protocol endpoint requires URL and deployment scope");
        }
        if let ProtocolModelScope::Models { ids } = &self.models
            && (ids.is_empty() || ids.iter().any(|id| id.trim().is_empty()))
        {
            return Err("protocol model scope requires explicit nonempty model IDs");
        }
        match &self.authentication {
            ProtocolAuthentication::ApiKeyHeader { name } if name.trim().is_empty() => {
                return Err("protocol authentication requires a header name");
            }
            ProtocolAuthentication::Unsupported { reason } if reason.trim().is_empty() => {
                return Err("unsupported authentication requires a reason");
            }
            _ => {}
        }
        if self.evidence.is_empty()
            || self.evidence.iter().any(|evidence| {
                evidence.source.trim().is_empty()
                    || evidence.reference.trim().is_empty()
                    || evidence.checked_at.trim().is_empty()
            })
        {
            return Err("protocol endpoint requires dated evidence references");
        }
        Ok(())
    }
}

/// Missing/null means unknown; an explicit empty array forbids inferred candidates.
/// Ambiguous declarations within the same deployment are rejected before selection.
pub fn deserialize_protocols<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<ProtocolEndpoint>>, D::Error>
where
    D: Deserializer<'de>,
{
    let endpoints = Option::<Vec<ProtocolEndpoint>>::deserialize(deserializer)?;
    if let Some(endpoints) = &endpoints {
        for (index, endpoint) in endpoints.iter().enumerate() {
            endpoint.validate().map_err(D::Error::custom)?;
            for previous in &endpoints[..index] {
                if endpoint.protocol == previous.protocol
                    && endpoint.deployment == previous.deployment
                    && endpoint.models.overlaps(&previous.models)
                    && (endpoint.url != previous.url
                        || endpoint.authentication != previous.authentication)
                {
                    return Err(D::Error::custom(
                        "conflicting protocol endpoints for deployment/model",
                    ));
                }
            }
        }
    }
    Ok(endpoints)
}
