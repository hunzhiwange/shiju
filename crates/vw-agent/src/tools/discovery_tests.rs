use super::{ToolDiscoveryCatalog, ToolExposureState};
use crate::tools::ToolSpec;
use serde_json::json;

fn spec(id: &str, description: &str) -> ToolSpec {
    ToolSpec::new(
        id,
        description,
        json!({"type":"object", "properties":{"query":{"type":"string"}}, "required":["query"]}),
    )
}

fn ids(specs: &[ToolSpec]) -> Vec<&str> {
    specs.iter().map(|spec| spec.id.as_str()).collect()
}

#[test]
fn codex_discovery_catalog_exact_name_precedes_description_matches() {
    let catalog = ToolDiscoveryCatalog::new(vec![
        spec("mcp__records__lookup", "Find records"),
        spec("mcp__records__other", "lookup lookup lookup"),
    ]);
    let hits = catalog.search("lookup", None).unwrap();
    assert_eq!(ids(&hits), ["mcp__records__lookup", "mcp__records__other"]);
    assert_eq!(hits[0].input_schema, spec("unused", "").input_schema);
    assert_eq!(hits[0].description, "Find records");
}

#[test]
fn codex_discovery_catalog_searches_descriptions_servers_and_chinese() {
    let catalog = ToolDiscoveryCatalog::new(vec![
        spec("mcp__calendar__lookup", "Find appointments"),
        spec("mcp__drive__lookup", "搜索团队文档"),
    ]);
    assert_eq!(ids(&catalog.search("appointments", None).unwrap()), ["mcp__calendar__lookup"]);
    assert_eq!(ids(&catalog.search("calendar", None).unwrap()), ["mcp__calendar__lookup"]);
    assert_eq!(ids(&catalog.search("文档", None).unwrap()), ["mcp__drive__lookup"]);
    assert!(catalog.search("volcano", None).unwrap().is_empty());
}

#[test]
fn codex_discovery_catalog_ties_are_stable_and_limits_are_enforced() {
    let catalog = ToolDiscoveryCatalog::new(
        (0..40)
            .rev()
            .map(|i| spec(&format!("mcp__records__item_{i:02}"), "Find records"))
            .collect(),
    );
    let hits = catalog.search("records", None).unwrap();
    assert_eq!(hits.len(), 8);
    assert_eq!(hits[0].id, "mcp__records__item_00");
    assert_eq!(catalog.search("records", Some(2)).unwrap().len(), 2);
    assert_eq!(catalog.search("records", Some(32)).unwrap().len(), 32);
    assert!(catalog.search("records", Some(0)).is_err());
    assert!(catalog.search("records", Some(33)).is_err());
    assert!(catalog.search(" \n ", None).is_err());
    assert!(catalog.search("...", None).is_err());
}

#[test]
fn codex_discovery_catalog_filters_unauthorized_and_duplicate_tools() {
    let registered = [
        spec("mcp__records__allowed", "Find records"),
        spec("mcp__records__forbidden", "Find records"),
    ];
    let catalog = ToolDiscoveryCatalog::new(vec![registered[0].clone(), registered[0].clone()]);
    assert_eq!(ids(&catalog.search("records", None).unwrap()), ["mcp__records__allowed"]);
}

#[test]
fn codex_discovery_catalog_bm25_prefers_relevant_short_descriptions() {
    let catalog = ToolDiscoveryCatalog::new(vec![
        spec("mcp__records__a", &format!("appointments {}", "unrelated ".repeat(100))),
        spec("mcp__records__b", "appointments"),
    ]);
    assert_eq!(
        ids(&catalog.search("APPOINTMENTS", None).unwrap()),
        ["mcp__records__b", "mcp__records__a"]
    );
    assert_eq!(ids(&catalog.search("MCP__RECORDS__A", Some(1)).unwrap()), ["mcp__records__a"]);
}

#[test]
fn codex_discovery_catalog_sources_and_search_include_only_deferred_tools() {
    let catalog = ToolDiscoveryCatalog::new(vec![
        spec("skill", "builtin skills"),
        spec("mcp__calendar__first", "Find events"),
        spec("mcp__calendar__second", "Find events"),
        spec("mcp__drive__lookup", "Find files"),
    ]);
    assert!(catalog.has_deferred_tools());
    assert_eq!(catalog.deferred_server_names(), ["calendar", "drive"]);
    assert!(catalog.search("builtin", None).unwrap().is_empty());
    let empty = ToolDiscoveryCatalog::new(vec![spec("skill", "Read skills")]);
    assert!(!empty.has_deferred_tools());
    assert!(empty.deferred_server_names().is_empty());
    assert!(empty.search("anything", None).unwrap().is_empty());
}

#[test]
fn codex_discovery_catalog_defers_only_mcp_ids_and_preserves_builtin_names() {
    let mut builtin = spec("lookup", "Find records");
    builtin.display_name = "mcp__calendar__lookup".into();
    let catalog = ToolDiscoveryCatalog::new(vec![
        builtin,
        spec("mcp__calendar__lookup", "Find records"),
        spec("skill", "Read skills"),
        spec("tool_search", "Find tools"),
        spec("create_thread", "Create thread"),
        spec("mcp__incomplete", "Builtin"),
    ]);
    let state = ToolExposureState::default();
    assert_eq!(
        ids(&catalog.exposed_specs(&state, true)),
        ["create_thread", "lookup", "mcp__incomplete", "skill", "tool_search"]
    );
    assert_eq!(catalog.exposed_specs(&state, false).len(), 6);
}

#[test]
fn codex_discovery_catalog_exposure_intersects_current_authorization_and_is_isolated() {
    let catalog = ToolDiscoveryCatalog::new(vec![
        spec("skill", "Read skills"),
        spec("mcp__records__lookup", "Find records"),
    ]);
    let mut state = ToolExposureState::default();
    state.record_loaded(&catalog, ["mcp__records__lookup", "mcp__records__forbidden"]);
    state.record_loaded(&catalog, ["mcp__records__lookup"]);
    assert_eq!(catalog.exposed_specs(&state, true).len(), 2);
    assert_eq!(catalog.exposed_specs(&ToolExposureState::default(), true).len(), 1);
    let rebuilt = ToolDiscoveryCatalog::new(vec![spec("skill", "Read skills")]);
    assert_eq!(ids(&rebuilt.exposed_specs(&state, true)), ["skill"]);
    state.retain_authorized(&rebuilt);
    assert_eq!(ids(&catalog.exposed_specs(&state, true)), ["skill"]);
}
