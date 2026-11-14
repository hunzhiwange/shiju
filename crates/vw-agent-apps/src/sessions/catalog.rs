//! Metadata-only directory snapshots. Callers scan authorized target-host instances
//! and pass their results here; this layer never discovers roots from requests.
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use uuid::Uuid;
use vw_api_types::external_session::{ExternalSessionErrorCode, SessionMeta, SessionProvider};
use vw_api_types::ProjectScope;

const SNAPSHOT_TTL: Duration = Duration::from_secs(300);
const MAX_SESSIONS: usize = 50_000;
const MAX_CURSORS: usize = 512;
pub const MAX_PAGE_SIZE: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogQuery {
    pub provider: Option<SessionProvider>,
    pub target_host: Option<String>,
    pub instance_id: Option<String>,
    pub project_dir: Option<String>,
    pub search: String,
    pub page_size: usize,
}

#[derive(Debug, Clone)]
pub struct CatalogPage {
    pub sessions: Vec<SessionMeta>,
    pub total: usize,
    pub next_cursor: Option<String>,
}

struct Position {
    query: CatalogQuery,
    offset: usize,
}

/// One project scope, one immutable scan generation, five-minute lifetime.
/// Store this on the authorized server side; do not rebuild it between pages.
/// Replacing/dropping the snapshot invalidates all of its opaque cursors.
/// The gateway must authenticate and verify scope again on every page request.
pub struct Catalog {
    scope: ProjectScope,
    sessions: Vec<SessionMeta>,
    created: Instant,
    cursors: HashMap<String, Position>,
}

impl Catalog {
    /// Aggregate all adapter batches without suppressing a failed scan. Adapter
    /// roots and execution contexts must already be resolved by the gateway.
    /// Exact duplicate identities collapse; conflicting copies are rejected.
    pub fn from_batches(
        scope: ProjectScope,
        batches: impl IntoIterator<Item = Result<Vec<SessionMeta>, String>>,
    ) -> Result<Self, ExternalSessionErrorCode> {
        let mut unique = HashMap::new();
        for batch in batches {
            for session in batch.map_err(|_| ExternalSessionErrorCode::StorageUnavailable)? {
                if session.identity.scope != scope {
                    return Err(ExternalSessionErrorCode::Forbidden);
                }
                super::identity::validate_identity(&session.identity)?;
                if let Some(existing) = unique.get(&session.identity) {
                    if existing != &session {
                        return Err(ExternalSessionErrorCode::Conflict);
                    }
                    continue;
                }
                if unique.len() >= MAX_SESSIONS {
                    return Err(ExternalSessionErrorCode::StorageUnavailable);
                }
                unique.insert(session.identity.clone(), session);
            }
        }
        // The complete serialized identity is the final tie-breaker, including
        // host/instance/provider/source. Native IDs alone are not globally unique.
        let mut keyed = Vec::with_capacity(unique.len());
        for session in unique.into_values() {
            let identity = serde_json::to_string(&session.identity)
                .map_err(|_| ExternalSessionErrorCode::StorageUnavailable)?;
            keyed.push((session, identity));
        }
        keyed.sort_by(|(a, a_key), (b, b_key)| {
            activity(b).cmp(&activity(a)).then_with(|| a_key.cmp(b_key))
        });
        Ok(Self {
            scope,
            sessions: keyed.into_iter().map(|(session, _)| session).collect(),
            created: Instant::now(),
            cursors: HashMap::new(),
        })
    }

    pub fn page(
        &mut self,
        authorized_scope: &ProjectScope,
        query: &CatalogQuery,
        cursor: Option<&str>,
    ) -> Result<CatalogPage, ExternalSessionErrorCode> {
        if authorized_scope != &self.scope {
            return Err(ExternalSessionErrorCode::Forbidden);
        }
        if self.created.elapsed() >= SNAPSHOT_TTL {
            self.cursors.clear();
            return Err(ExternalSessionErrorCode::CursorExpired);
        }
        if query.page_size == 0 || query.page_size > MAX_PAGE_SIZE || query.search.len() > 4096 {
            return Err(ExternalSessionErrorCode::InvalidInput);
        }
        let offset = match cursor {
            Some(token) => {
                let position = self.cursors.get(token)
                    .ok_or(ExternalSessionErrorCode::CursorExpired)?;
                if &position.query != query {
                    return Err(ExternalSessionErrorCode::CursorExpired);
                }
                position.offset
            }
            None => 0,
        };
        let words = search_words(&query.search);
        let matching: Vec<_> = self.sessions.iter().filter(|session| {
            query.provider.is_none_or(|provider| session.identity.provider == provider)
                && query.target_host.as_ref().is_none_or(|host| &session.identity.target_host == host)
                && query.instance_id.as_ref().is_none_or(|instance| &session.identity.instance_id == instance)
                && query.project_dir.as_ref().is_none_or(|dir| session.project_dir.as_ref() == Some(dir))
                && matches_words(session, &words)
        }).collect();
        let total = matching.len();
        let end = offset.saturating_add(query.page_size).min(total);
        if offset > total { return Err(ExternalSessionErrorCode::CursorExpired); }
        let sessions = matching[offset..end].iter().map(|session| (**session).clone()).collect();
        let next_cursor = if end < total {
            // Reuse an existing position so retries produce the same cursor.
            if let Some((token, _)) = self.cursors.iter()
                .find(|(_, position)| position.offset == end && &position.query == query)
            {
                Some(token.clone())
            } else {
                if self.cursors.len() >= MAX_CURSORS {
                    return Err(ExternalSessionErrorCode::CursorExpired);
                }
                let token = Uuid::new_v4().to_string();
                self.cursors.insert(token.clone(), Position { query: query.clone(), offset: end });
                Some(token)
            }
        } else { None };
        Ok(CatalogPage { sessions, total, next_cursor })
    }
}

fn activity(session: &SessionMeta) -> i64 {
    session.last_active_at.or(session.created_at).unwrap_or(0)
}

fn search_words(query: &str) -> HashSet<String> {
    query.split(|c: char| !c.is_alphanumeric()).filter(|word| !word.is_empty())
        .map(str::to_lowercase).collect()
}

fn matches_words(session: &SessionMeta, words: &HashSet<String>) -> bool {
    let fields = [
        Some(session.identity.session_id.0.as_str()),
        session.title.as_deref(), session.summary.as_deref(),
        session.project_dir.as_deref(), session.source_path.as_deref(),
    ];
    let fields: Vec<_> = fields.into_iter().flatten().map(str::to_lowercase).collect();
    // Full-token substring matching preserves partial paths/IDs and unspaced CJK.
    // Each search word must occur in at least one metadata field; never read body.
    words.iter().all(|word| fields.iter().any(|field| field.contains(word)))
}
