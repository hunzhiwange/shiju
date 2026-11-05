//! Authorized tool snapshots and per-turn exposure, independent of registration.

use super::ToolSpec;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};

pub(crate) const DEFAULT_SEARCH_LIMIT: usize = 8;
pub(crate) const MAX_SEARCH_LIMIT: usize = 32;

/// Built only from the caller's authorized specs, never from the whole registry.
/// Cloning the catalog shares schemas and the fitted index with the same snapshot.
#[derive(Clone)]
pub(crate) struct ToolDiscoveryCatalog {
    snapshot: Arc<CatalogSnapshot>,
}

struct CatalogSnapshot {
    specs: BTreeMap<String, Arc<ToolSpec>>,
    documents: Vec<SearchDocument>,
    document_frequencies: BTreeMap<String, usize>,
    average_length: f64,
}

// Bounded process-local cache. Only immutable authorized specifications/indexes
// are shared; loaded IDs remain owned by each user turn.
#[derive(Default)]
struct CatalogCache {
    entries: VecDeque<(Vec<u8>, Arc<CatalogSnapshot>)>,
    #[cfg(test)]
    rebuilds: usize,
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
        static CACHE: OnceLock<Mutex<CatalogCache>> = OnceLock::new();
        let mut cache = CACHE
            .get_or_init(|| Mutex::new(CatalogCache::default()))
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        Self::from_cache(specs, &mut cache)
    }

    fn from_cache(specs: BTreeMap<String, ToolSpec>, cache: &mut CatalogCache) -> Self {
        // Exact serialized full specifications form the authorization-directory
        // fingerprint: no hash collision or omitted schema/security field can reuse it.
        let fingerprint = serde_json::to_vec(&specs).expect("tool specs serialize to JSON");
        if let Some(index) = cache.entries.iter().position(|(key, _)| *key == fingerprint) {
            let entry = cache.entries.remove(index).expect("matched cache entry");
            let snapshot = Arc::clone(&entry.1);
            cache.entries.push_back(entry);
            return Self { snapshot };
        }
        let catalog = Self::build(specs);
        #[cfg(test)]
        {
            cache.rebuilds += 1;
        }
        if cache.entries.len() == 8 {
            cache.entries.pop_front();
        }
        cache.entries.push_back((fingerprint, Arc::clone(&catalog.snapshot)));
        catalog
    }

    fn build(specs: BTreeMap<String, ToolSpec>) -> Self {
        let specs: BTreeMap<_, _> =
            specs.into_iter().map(|(id, spec)| (id, Arc::new(spec))).collect();
        let mut documents = Vec::new();
        let mut document_frequencies = BTreeMap::new();
        for spec in specs.values().filter(|spec| spec.exposure.is_deferred()) {
            let mut terms = BTreeMap::new();
            for field in [
                spec.id.as_str(),
                spec.display_name.as_str(),
                spec.name.as_str(),
                spec.description.as_str(),
                spec.identity.name.as_str(),
                spec.identity.namespace.as_deref().unwrap_or_default(),
            ] {
                for token in tokenize(field) {
                    *terms.entry(token).or_insert(0) += 1;
                }
            }
            let source = match spec.source {
                vw_api_types::tools::ToolSource::Builtin => "builtin",
                vw_api_types::tools::ToolSource::Mcp => "mcp",
                vw_api_types::tools::ToolSource::Client => "client",
            };
            for token in tokenize(source) {
                *terms.entry(token).or_insert(0) += 1;
            }
            let mut budget = 4096;
            append_schema_terms(&spec.input_schema, 0, &mut budget, &mut terms);
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

    /// Authorized namespaces only, without materializing tool schemas.
    pub(crate) fn deferred_namespace_names(&self) -> Vec<&str> {
        self.snapshot
            .documents
            .iter()
            .filter_map(|document| self.snapshot.specs[&document.id].identity.namespace.as_deref())
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
                || spec.identity.name.to_lowercase() == query;
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
            .map(|(document, _, _)| self.snapshot.specs[&document.id].as_ref().clone())
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
    /// denied, expose only authorized tools that allow model calls.
    pub(crate) fn exposed_specs(
        &self,
        state: &ToolExposureState,
        search_enabled: bool,
    ) -> Vec<ToolSpec> {
        self.snapshot
            .specs
            .values()
            .filter(|spec| {
                spec.exposure.is_direct()
                    || (spec.exposure.is_deferred()
                        && (!search_enabled || state.loaded_ids.contains(&spec.id)))
            })
            .map(|spec| spec.as_ref().clone())
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
                .filter(|id| {
                    catalog.snapshot.specs.get(*id).is_some_and(|spec| spec.exposure.is_deferred())
                })
                .map(str::to_owned),
        );
    }

    pub(crate) fn retain_authorized(&mut self, catalog: &ToolDiscoveryCatalog) {
        self.loaded_ids.retain(|id| {
            catalog.snapshot.specs.get(id).is_some_and(|spec| spec.exposure.is_deferred())
        });
    }
}

// Walk schema-bearing keywords only; never follow references or index defaults,
// examples, arbitrary extension payloads, or an unbounded recursive structure.
fn append_schema_terms(
    schema: &serde_json::Value,
    depth: usize,
    budget: &mut usize,
    terms: &mut BTreeMap<String, usize>,
) {
    if *budget == 0 {
        return;
    }
    *budget -= 1;
    if depth >= 16 {
        return;
    }
    let mut add = |text: &str| {
        // Bound text before tokenization, including non-ASCII descriptions.
        let text: String = text.chars().take(4096).collect();
        for token in tokenize(&text) {
            *terms.entry(token).or_insert(0) += 1;
        }
    };
    for key in ["title", "description"] {
        if let Some(text) = schema.get(key).and_then(serde_json::Value::as_str) {
            add(text);
        }
    }
    for key in ["properties", "$defs", "definitions", "patternProperties"] {
        if let Some(properties) = schema.get(key).and_then(serde_json::Value::as_object) {
            for (name, child) in properties {
                if *budget == 0 {
                    break;
                }
                for token in tokenize(&name.chars().take(4096).collect::<String>()) {
                    *terms.entry(token).or_insert(0) += 1;
                }
                append_schema_terms(child, depth + 1, budget, terms);
            }
        }
    }
    for key in ["items", "additionalProperties", "contains", "not", "if", "then", "else"] {
        if let Some(child) = schema.get(key) {
            if let Some(children) = child.as_array() {
                for child in children {
                    if *budget == 0 {
                        break;
                    }
                    append_schema_terms(child, depth + 1, budget, terms);
                }
            } else {
                append_schema_terms(child, depth + 1, budget, terms);
            }
        }
    }
    for key in ["anyOf", "allOf", "oneOf", "prefixItems"] {
        if let Some(children) = schema.get(key).and_then(serde_json::Value::as_array) {
            for child in children {
                if *budget == 0 {
                    break;
                }
                append_schema_terms(child, depth + 1, budget, terms);
            }
        }
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
