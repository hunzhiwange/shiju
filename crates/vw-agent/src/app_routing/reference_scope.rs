//! In-memory ownership of opaque response and item references.

use crate::model_relay::RelayIdentity;
use axum::http::StatusCode;
use vw_api_types::{agent_apps::AppId, id::ProjectScope};

/// The application fields stay private to the routing module until a verified request supplies them.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ApplicationReferenceScope {
    pub(super) project: ProjectScope,
    pub(super) host: String,
    pub(super) app: AppId,
    pub(super) account: String,
    pub(super) provider: String,
    pub(super) credential_generation: u64,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum ReferenceScope {
    Session(RelayIdentity),
    Application(ApplicationReferenceScope),
}

const REFERENCE_LIMIT: usize = 4096;
struct Reference {
    scope: ReferenceScope,
    public: String,
    upstream: String,
    item: bool,
}
/// No Debug/Serialize: this is transient provider state, never persisted as session history.
#[derive(Default)]
pub(crate) struct References(parking_lot::Mutex<Vec<Reference>>);
impl References {
    pub(crate) fn resolve(
        &self,
        scope: &ReferenceScope,
        public: &str,
    ) -> Result<String, StatusCode> {
        let entries = self.0.lock();
        entries
            .iter()
            .find(|entry| &entry.scope == scope && entry.public == public && !entry.item)
            .map(|entry| entry.upstream.clone())
            .ok_or(StatusCode::FORBIDDEN)
    }
    pub(crate) fn public(
        &self,
        scope: &ReferenceScope,
        upstream: &str,
    ) -> Result<String, StatusCode> {
        let entries = self.0.lock();
        entries
            .iter()
            .find(|entry| &entry.scope == scope && entry.upstream == upstream && !entry.item)
            .map(|entry| entry.public.clone())
            .ok_or(StatusCode::BAD_GATEWAY)
    }
    pub(crate) fn remember(
        &self,
        scope: &ReferenceScope,
        public: &str,
        upstream: &str,
    ) -> Result<(), StatusCode> {
        self.insert(scope, public, upstream, false)
    }
    pub(crate) fn check_item(&self, scope: &ReferenceScope, id: &str) -> Result<(), StatusCode> {
        let entries = self.0.lock();
        if entries.iter().any(|entry| entry.item && &entry.scope == scope && entry.upstream == id) {
            Ok(())
        } else {
            Err(StatusCode::FORBIDDEN)
        }
    }
    pub(crate) fn remember_item(&self, scope: &ReferenceScope, id: &str) -> Result<(), StatusCode> {
        self.insert(scope, id, id, true)
    }

    fn insert(
        &self,
        scope: &ReferenceScope,
        public: &str,
        upstream: &str,
        item: bool,
    ) -> Result<(), StatusCode> {
        if public.len() > 1024 || upstream.is_empty() || upstream.len() > 1024 {
            return Err(StatusCode::BAD_GATEWAY);
        }
        let mut entries = self.0.lock();
        if item
            && entries
                .iter()
                .any(|entry| entry.item && &entry.scope == scope && entry.upstream == upstream)
        {
            return Ok(());
        }
        if entries.len() >= REFERENCE_LIMIT {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
        entries.push(Reference {
            scope: scope.clone(),
            public: public.into(),
            upstream: upstream.into(),
            item,
        });
        Ok(())
    }
}
