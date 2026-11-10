use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;
use vw_api_types::id::TenantScope;

/// Trusted identity supplied by the session owner, never reconstructed from HTTP headers.
#[derive(Clone, PartialEq, Eq)]
pub struct RelayIdentity {
    pub tenant: TenantScope,
    pub project: String,
    pub session: String,
    pub provider: String,
    pub model: String,
    pub generation: u64,
}

/// Runtime capability. Deliberately neither Debug nor Serialize.
pub struct RelayBinding {
    pub(crate) identity: RelayIdentity,
    token: String,
    token_key: ring::hmac::Key,
    token_tag: ring::hmac::Tag,
    pub(crate) expires: Option<Instant>,
    pub(crate) cancelled: CancellationToken,
}
impl RelayBinding {
    pub fn new(identity: RelayIdentity, ttl: Duration) -> anyhow::Result<Arc<Self>> {
        anyhow::ensure!(
            !ttl.is_zero() && ttl <= Duration::from_secs(900),
            "invalid relay lifetime"
        );
        Self::issue(identity, Some(Instant::now() + ttl))
    }

    /// The owning ACP launch profile revokes the capability on drop.
    #[cfg(unix)]
    pub(crate) fn for_prompt(identity: RelayIdentity) -> anyhow::Result<Arc<Self>> {
        Self::issue(identity, None)
    }

    fn issue(identity: RelayIdentity, expires: Option<Instant>) -> anyhow::Result<Arc<Self>> {
        anyhow::ensure!(
            [&identity.project, &identity.session, &identity.provider, &identity.model]
                .iter()
                .all(|s| !s.trim().is_empty()),
            "incomplete relay identity"
        );
        let mut bytes = [0; 32];
        getrandom::fill(&mut bytes).map_err(|_| anyhow::anyhow!("relay entropy unavailable"))?;
        let token = hex::encode(bytes);
        let token_key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, &bytes);
        let token_tag = ring::hmac::sign(&token_key, token.as_bytes());
        Ok(Arc::new(Self {
            identity,
            token,
            token_key,
            token_tag,
            expires,
            cancelled: CancellationToken::new(),
        }))
    }
    pub fn bearer_token(&self) -> &str {
        &self.token
    }
    pub fn revoke(&self) {
        self.cancelled.cancel();
    }
    pub fn accepts(&self, token: &str, identity: &RelayIdentity) -> bool {
        !self.cancelled.is_cancelled()
            && self.expires.is_none_or(|expires| Instant::now() < expires)
            && &self.identity == identity
            && token.len() == self.token.len()
            && ring::hmac::verify(&self.token_key, token.as_bytes(), self.token_tag.as_ref())
                .is_ok()
    }
}
