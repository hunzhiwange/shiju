//! Ordered queue mutations for one application and one authorized provider snapshot.
use crate::{
    providers::failover::{ProviderAuth, ProviderSnapshot},
    routing::supports_app,
};
use vw_api_types::{agent_apps::AppId, app_providers::AppProviderSummary};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueError {
    UnsupportedApp,
    ProviderNotFound,
    IneligibleProvider,
}

pub struct Queue {
    app: AppId,
    members: Vec<String>,
}

impl Queue {
    pub fn new(app: AppId, members: Vec<String>) -> Self {
        Self { app, members }
    }

    pub fn members(&self) -> &[String] {
        &self.members
    }

    /// The caller supplies the authorized scope's current provider snapshot.
    pub fn add(&mut self, id: &str, providers: &[ProviderSnapshot]) -> Result<bool, QueueError> {
        self.require_supported_app()?;
        let provider = providers
            .iter()
            .find(|entry| entry.provider.app_id == self.app && entry.provider.id == id)
            .ok_or(QueueError::ProviderNotFound)?;
        if !Self::eligible(provider) {
            return Err(QueueError::IneligibleProvider);
        }
        if self.contains(id) {
            return Ok(false);
        }
        self.members.push(id.into());
        Ok(true)
    }

    /// Removing a stale member is allowed. Return only the removed provider ID.
    pub fn remove(&mut self, id: &str) -> Result<Option<String>, QueueError> {
        self.require_supported_app()?;
        let before = self.members.len();
        self.members.retain(|member| member != id);
        Ok((self.members.len() != before).then(|| id.to_owned()))
    }

    pub fn candidates<'a>(
        &self,
        providers: &'a [ProviderSnapshot],
    ) -> Result<Vec<&'a AppProviderSummary>, QueueError> {
        self.require_supported_app()?;
        Ok(providers
            .iter()
            .filter(|entry| {
                entry.provider.app_id == self.app
                    && Self::eligible(entry)
                    && !self.contains(&entry.provider.id)
            })
            .map(|entry| &entry.provider)
            .collect())
    }

    fn require_supported_app(&self) -> Result<(), QueueError> {
        supports_app(self.app).then_some(()).ok_or(QueueError::UnsupportedApp)
    }

    fn eligible(entry: &ProviderSnapshot) -> bool {
        entry.provider.configured && entry.auth == ProviderAuth::ThirdParty
    }

    fn contains(&self, id: &str) -> bool {
        self.members.iter().any(|member| member == id)
    }
}
