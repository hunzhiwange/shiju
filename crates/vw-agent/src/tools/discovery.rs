//! Authorized tool snapshots and per-turn exposure, independent of registration.

use super::ToolSpec;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use vw_config_types::tools::is_mcp_dynamic_tool_id;

pub(crate) const DEFAULT_SEARCH_LIMIT: usize = 8;
pub(crate) const MAX_SEARCH_LIMIT: usize = 32;

/// Built only from the caller's authorized specs, never from the whole registry.
/// Cloning the catalog shares schemas and the fitted index with the same snapshot.
#[derive(Clone)]
pub(crate) struct ToolDiscoveryCatalog {
    snapshot: Arc<CatalogSnapshot>,
}

struct CatalogSnapshot {
    specs: BTreeMap<String, ToolSpec>,
    documents: Vec<SearchDocument>,
    document_frequencies: BTreeMap<String, usize>,
    average_length: f64,
}

struct SearchDocument {
    id: String,
    terms: BTreeMap<String, usize>,
    length: usize,
}

impl ToolDiscoveryCatalog {
    pub(crate) fn new(authorized_specs: Vec<ToolSpec>) -> Self {
        let mut specs = BTreeMap::new();
        for spec in authorized_specs {
            // Preserve the first authorized definition of a stable ID.
            specs.entry(spec.id.clone()).or_insert(spec);
        }
        let mut documents = Vec::new();
        let mut document_frequencies = BTreeMap::new();
        for spec in specs.values().filter(|spec| is_mcp_dynamic_tool_id(&spec.id)) {
            let mut terms = BTreeMap::new();
            for field in [&spec.id, &spec.display_name, &spec.name, &spec.description] {
                for token in tokenize(field) {
                    *terms.entry(token).or_insert(0) += 1;
                }
            }
            for term in terms.keys() {
                *document_frequencies.entry(term.clone()).or_insert(0) += 1;
            }
            let length = terms.values().sum();
            documents.push(SearchDocument { id: spec.id.clone(), terms, length });
        }
        let total_length: usize = documents.iter().map(|document| document.length).sum();
        let average_length =
            if documents.is_empty() { 1.0 } else { total_length as f64 / documents.len() as f64 };
        Self {
            snapshot: Arc::new(CatalogSnapshot {
                specs,
                documents,
                document_frequencies,
                average_length,
            }),
        }
    }

    pub(crate) fn has_deferred_tools(&self) -> bool {
        !self.snapshot.documents.is_empty()
    }

    pub(crate) fn contains(&self, id: &str) -> bool {
        self.snapshot.specs.contains_key(id)
    }

    /// Server names only: the search entry can describe sources without schemas.
    pub(crate) fn deferred_server_names(&self) -> Vec<&str> {
        self.snapshot
            .documents
            .iter()
            .filter_map(|document| document.id.strip_prefix("mcp__")?.split("__").next())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// Search only deferred, authorized tools; materialize schemas after ranking.
    pub(crate) fn search(
        &self,
        query: &str,
        limit: Option<usize>,
    ) -> anyhow::Result<Vec<ToolSpec>> {
        let limit = limit.unwrap_or(DEFAULT_SEARCH_LIMIT);
        if !(1..=MAX_SEARCH_LIMIT).contains(&limit) {
            anyhow::bail!("limit must be between 1 and {MAX_SEARCH_LIMIT}");
        }
        let query = query.trim().to_lowercase();
        let terms: BTreeSet<_> = tokenize(&query).into_iter().collect();
        if terms.is_empty() {
            anyhow::bail!("query must contain a searchable word or character");
        }
        let mut ranked = Vec::new();
        for document in &self.snapshot.documents {
            let spec = &self.snapshot.specs[&document.id];
            let exact_name = [&spec.id, &spec.name, &spec.display_name]
                .into_iter()
                .any(|name| name.to_lowercase() == query)
                || spec
                    .id
                    .split_once("__")
                    .and_then(|(_, rest)| rest.split_once("__"))
                    .is_some_and(|(_, tool)| tool.to_lowercase() == query);
            let score = self.bm25_score(document, &terms);
            if exact_name || score > 0.0 {
                ranked.push((document, exact_name, score));
            }
        }
        ranked.sort_by(|(a, a_exact, a_score), (b, b_exact, b_score)| {
            b_exact
                .cmp(a_exact)
                .then_with(|| b_score.total_cmp(a_score))
                .then_with(|| a.id.cmp(&b.id))
        });
        Ok(ranked
            .into_iter()
            .take(limit)
            .map(|(document, _, _)| self.snapshot.specs[&document.id].clone())
            .collect())
    }

    fn bm25_score(&self, document: &SearchDocument, query: &BTreeSet<String>) -> f64 {
        const K1: f64 = 1.2;
        const B: f64 = 0.75;
        let corpus_size = self.snapshot.documents.len() as f64;
        let length_ratio = document.length as f64 / self.snapshot.average_length;
        query
            .iter()
            .filter_map(|term| {
                let frequency = *document.terms.get(term)? as f64;
                let containing = *self.snapshot.document_frequencies.get(term)? as f64;
                let inverse_frequency =
                    ((corpus_size - containing + 0.5) / (containing + 0.5) + 1.0).ln();
                Some(
                    inverse_frequency * frequency * (K1 + 1.0)
                        / (frequency + K1 * (1.0 - B + B * length_ratio)),
                )
            })
            .sum()
    }

    /// Intersect loaded IDs with this snapshot on every request. If search is
    /// denied, expose authorized MCP tools directly so they remain reachable.
    pub(crate) fn exposed_specs(
        &self,
        state: &ToolExposureState,
        search_enabled: bool,
    ) -> Vec<ToolSpec> {
        self.snapshot
            .specs
            .values()
            .filter(|spec| {
                !search_enabled
                    || !is_mcp_dynamic_tool_id(&spec.id)
                    || state.loaded_ids.contains(&spec.id)
            })
            .cloned()
            .collect()
    }
}

/// Runtime-owned loaded IDs. Search results may update this after success;
/// model output and persisted history must never supply these IDs directly.
#[derive(Clone, Default)]
pub(crate) struct ToolExposureState {
    loaded_ids: BTreeSet<String>,
}

impl ToolExposureState {
    pub(crate) fn record_loaded<'a>(
        &mut self,
        catalog: &ToolDiscoveryCatalog,
        ids: impl IntoIterator<Item = &'a str>,
    ) {
        self.retain_authorized(catalog);
        self.loaded_ids.extend(
            ids.into_iter()
                .filter(|id| is_mcp_dynamic_tool_id(id) && catalog.snapshot.specs.contains_key(*id))
                .map(str::to_owned),
        );
    }

    pub(crate) fn retain_authorized(&mut self, catalog: &ToolDiscoveryCatalog) {
        self.loaded_ids.retain(|id| catalog.snapshot.specs.contains_key(id));
    }
}

fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut word = String::new();
    let mut previous_cjk = None;
    for character in text.chars().flat_map(char::to_lowercase) {
        if is_cjk(character) {
            if !word.is_empty() {
                tokens.push(std::mem::take(&mut word));
            }
            tokens.push(character.to_string());
            if let Some(previous) = previous_cjk {
                tokens.push(format!("{previous}{character}"));
            }
            previous_cjk = Some(character);
        } else {
            previous_cjk = None;
            if character.is_alphanumeric() {
                word.push(character);
            } else if !word.is_empty() {
                tokens.push(std::mem::take(&mut word));
            }
        }
    }
    if !word.is_empty() {
        tokens.push(word);
    }
    tokens
}

fn is_cjk(character: char) -> bool {
    matches!(character, '\u{3400}'..='\u{9fff}' | '\u{f900}'..='\u{faff}' | '\u{20000}'..='\u{323af}')
}

#[cfg(test)]
#[path = "discovery_tests.rs"]
mod tests;
