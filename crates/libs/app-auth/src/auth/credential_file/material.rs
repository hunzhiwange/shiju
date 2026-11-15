use super::*;

/// Explicit imported material. No Debug/Serialize implementation: never log or return secrets.
/// Only the credential owner can construct this path, from a scoped account record.
pub struct ClaudeAccountDirectory {
    pub(super) path: PathBuf,
}
impl ClaudeAccountDirectory {
    pub fn path(&self) -> &Path {
        &self.path
    }
}
#[cfg(unix)]
pub(super) const DIRECTORY_MARKER: &[u8] = b"shiju-managed-claude-directory-v1\n";

/// Host-private xAI token material. Store only under the authorized xAI account lease.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct XaiCredential {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: TimestampMs,
    #[serde(default)]
    pub requires_reauth: bool,
}

/// Host-private Kimi tokens and the device identity used to obtain them.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KimiCredential {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: TimestampMs,
    pub region: AuthRegion,
    pub device_id: String,
}

/// Imported OAuth grant; no Debug implementation exposes token material.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeminiCredential {
    pub access_token: String,
    pub refresh_token: String,
    pub client_id: String,
    pub client_secret: String,
    pub expires_at: TimestampMs,
    #[serde(default)]
    pub requires_reauth: bool,
}

pub enum AccountCredential {
    GeminiOauth(GeminiCredential),
    ClaudeToken(String),
    ClaudeDirectory(ClaudeAccountDirectory),
    CodexAuth(serde_json::Value),
    CopilotToken(String),
    XaiOauth(XaiCredential),
    KimiOauth(KimiCredential),
}

#[cfg(unix)]
#[derive(Clone, Copy)]
pub(in crate::auth) enum CredentialKind {
    App(AppId),
    Copilot,
    Xai,
    Kimi,
}
impl AccountCredential {
    pub fn validate(&self, app: AppId) -> Result<(), String> {
        match (app, self) {
            (AppId::Claude, Self::ClaudeDirectory(directory)) => {
                #[cfg(unix)]
                {
                    PrivateDirectory::open(directory.path()).map(|_| ())
                }
                #[cfg(not(unix))]
                {
                    let _ = directory;
                    Err(UNSUPPORTED.into())
                }
            }
            (AppId::Claude, Self::ClaudeToken(token))
                if !token.is_empty()
                    && token.len() <= 16384
                    && !token.chars().any(char::is_whitespace)
                    && !token.chars().any(char::is_control) =>
            {
                Ok(())
            }
            (AppId::Codex, Self::CodexAuth(auth)) => {
                if auth.to_string().len() > MAX_BYTES {
                    return Err(INVALID_MATERIAL.into());
                }
                let object = auth.as_object().ok_or(INVALID_MATERIAL)?;
                if object.get("last_refresh").is_some_and(|v| {
                    !v.is_null()
                        && !v
                            .as_str()
                            .is_some_and(|s| s.len() <= 128 && !s.chars().any(char::is_control))
                }) {
                    return Err(INVALID_MATERIAL.into());
                }
                if object.keys().any(|k| {
                    !matches!(
                        k.as_str(),
                        "auth_mode" | "OPENAI_API_KEY" | "tokens" | "last_refresh"
                    )
                }) || object.get("OPENAI_API_KEY").is_some_and(|v| !v.is_null())
                    || object.get("auth_mode").is_some_and(|v| v != "chatgpt")
                {
                    return Err(INVALID_MATERIAL.into());
                }
                let tokens =
                    object.get("tokens").and_then(|v| v.as_object()).ok_or(INVALID_MATERIAL)?;
                if tokens.keys().any(|k| {
                    !matches!(
                        k.as_str(),
                        "access_token" | "refresh_token" | "id_token" | "account_id"
                    )
                }) || ["access_token", "refresh_token", "id_token", "account_id"].iter().any(
                    |key| {
                        !tokens.get(*key).and_then(|v| v.as_str()).is_some_and(|s| {
                            !s.is_empty()
                                && s.len() <= 16384
                                && !s.chars().any(|c| c.is_whitespace() || c.is_control())
                        })
                    },
                ) {
                    return Err(INVALID_MATERIAL.into());
                }
                Ok(())
            }
            (AppId::Gemini, Self::GeminiOauth(tokens))
                if tokens.expires_at.0 > 0
                    && [
                        &tokens.access_token,
                        &tokens.refresh_token,
                        &tokens.client_id,
                        &tokens.client_secret,
                    ]
                    .into_iter()
                    .all(|s| {
                        !s.is_empty() && s.len() <= 16384 && s.bytes().all(|b| b.is_ascii_graphic())
                    }) =>
            {
                Ok(())
            }
            (AppId::Agy, _) => {
                Err("agy official accounts require an isolated OS keychain execution host".into())
            }
            _ => Err("official account material does not match application".into()),
        }
    }
    #[cfg(unix)]
    pub(super) fn bytes(&self) -> Result<Vec<u8>, String> {
        Ok(match self {
            Self::ClaudeToken(s) | Self::CopilotToken(s) => s.as_bytes().to_vec(),
            Self::ClaudeDirectory(_) => DIRECTORY_MARKER.to_vec(),
            Self::CodexAuth(v) => v.to_string().into_bytes(),
            Self::XaiOauth(tokens) => serde_json::to_vec(tokens)
                .ok()
                .filter(|bytes| bytes.len() <= MAX_BYTES)
                .ok_or(INVALID_MATERIAL)?,
            Self::GeminiOauth(tokens) => {
                serde_json::to_vec(tokens).map_err(|_| INVALID_MATERIAL)?
            }
            Self::KimiOauth(tokens) => serde_json::to_vec(tokens)
                .ok()
                .filter(|bytes| bytes.len() <= MAX_BYTES)
                .ok_or(INVALID_MATERIAL)?,
        })
    }
}

#[cfg(not(unix))]
pub(super) const UNSUPPORTED: &str = "official credential storage requires a reviewed host ACL adapter";
pub(super) const INVALID_MATERIAL: &str = "invalid official account material";
pub(super) const MAX_BYTES: usize = 100_000;
