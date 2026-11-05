use super::{ToolDiscoveryCatalog, ToolExposureState};
use crate::tools::ToolSpec;
use serde_json::json;

fn spec(id: &str, description: &str) -> ToolSpec {
    let mut tool = ToolSpec::new(
        id,
        description,
        json!({"type":"object", "properties":{"query":{"type":"string"}}, "required":["query"]}),
    );
    if let Some((namespace, name)) = id.strip_prefix("mcp__").and_then(|rest| rest.split_once("__"))
    {
        tool.identity =
            vw_api_types::tools::ToolIdentity::new(Some(namespace.into()), name).unwrap();
        tool.source = vw_api_types::tools::ToolSource::Mcp;
        tool.exposure = vw_api_types::tools::ToolExposure::Deferred;
    }
    tool
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
    assert_eq!(catalog.deferred_namespace_names(), ["calendar", "drive"]);
    assert!(catalog.search("builtin", None).unwrap().is_empty());
    let empty = ToolDiscoveryCatalog::new(vec![spec("skill", "Read skills")]);
    assert!(!empty.has_deferred_tools());
    assert!(empty.deferred_namespace_names().is_empty());
    assert!(empty.search("anything", None).unwrap().is_empty());
}

#[test]
fn codex_discovery_catalog_uses_registered_exposure_and_preserves_builtin_names() {
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

#[test]
fn dynamic_tools_exposure_non_mcp_deferred_is_discoverable() {
    let mut tool = spec("client-stable", "Find invoices");
    tool.source = vw_api_types::tools::ToolSource::Client;
    tool.exposure = vw_api_types::tools::ToolExposure::Deferred;
    let catalog = ToolDiscoveryCatalog::new(vec![tool]);
    assert!(catalog.exposed_specs(&ToolExposureState::default(), true).is_empty());
    assert_eq!(ids(&catalog.search("invoices", None).unwrap()), ["client-stable"]);
    let mut state = ToolExposureState::default();
    state.record_loaded(&catalog, ["client-stable", "unauthorized"]);
    assert_eq!(ids(&catalog.exposed_specs(&state, true)), ["client-stable"]);
}

#[test]
fn dynamic_tools_exposure_policies_control_search_and_fallback() {
    use vw_api_types::tools::ToolExposure::*;
    let policies = [Direct, Deferred, DirectModelOnly, DeferredModelOnly, CodeModeOnly, Hidden];
    let tools = policies
        .into_iter()
        .enumerate()
        .map(|(i, exposure)| {
            let mut tool = spec(&format!("tool-{i}"), "invoice");
            tool.exposure = exposure;
            tool
        })
        .collect();
    let catalog = ToolDiscoveryCatalog::new(tools);
    let mut state = ToolExposureState::default();
    assert_eq!(ids(&catalog.exposed_specs(&state, true)), ["tool-0", "tool-2"]);
    assert_eq!(ids(&catalog.search("invoice", None).unwrap()), ["tool-1", "tool-3"]);
    state.record_loaded(&catalog, ["tool-1", "tool-3", "tool-4", "tool-5"]);
    assert_eq!(ids(&catalog.exposed_specs(&state, true)), ["tool-0", "tool-1", "tool-2", "tool-3"]);
    assert_eq!(
        ids(&catalog.exposed_specs(&ToolExposureState::default(), false)),
        ["tool-0", "tool-1", "tool-2", "tool-3"]
    );
}

#[test]
fn dynamic_tools_search_schema_fields_are_searchable() {
    let mut tool = spec("mcp__fixture__schema", "neutral");
    tool.input_schema = json!({"type":"object", "properties": {
        "invoice_number": {"type":"array", "items":{"anyOf":[
            {"type":"string", "description":"报销凭证 reimbursement"}
        ]}}
    }});
    let catalog = ToolDiscoveryCatalog::new(vec![tool]);
    for query in ["invoice", "number", "reimbursement", "凭证"] {
        assert_eq!(ids(&catalog.search(query, None).unwrap()), ["mcp__fixture__schema"], "{query}");
    }
}

#[test]
fn dynamic_tools_search_normalized_names_and_sources_are_searchable() {
    let mut tool = spec("opaque", "neutral");
    tool.identity.name = "lookup_invoice".into();
    tool.exposure = vw_api_types::tools::ToolExposure::Deferred;
    tool.source = vw_api_types::tools::ToolSource::Client;
    let catalog = ToolDiscoveryCatalog::new(vec![tool]);
    for query in ["lookup invoice", "client"] {
        assert_eq!(ids(&catalog.search(query, None).unwrap()), ["opaque"]);
    }
}

#[test]
fn dynamic_tools_search_cache_reuses_specs_and_invalidates_complete_versions() {
    use super::CatalogCache;
    use std::sync::Arc;
    let mut cache = CatalogCache::default();
    let tool = spec("mcp__cache__version", "neutral");
    let build = |tools: Vec<ToolSpec>, cache: &mut CatalogCache| {
        ToolDiscoveryCatalog::from_cache(
            tools.into_iter().map(|s| (s.id.clone(), s)).collect(),
            cache,
        )
    };
    let first = build(vec![tool.clone()], &mut cache);
    let second = build(vec![tool.clone()], &mut cache);
    assert_eq!(cache.rebuilds, 1);
    assert!(Arc::ptr_eq(&first.snapshot, &second.snapshot));
    assert!(Arc::ptr_eq(&first.snapshot.specs[&tool.id], &second.snapshot.specs[&tool.id]));
    let mut result = first.search("version", None).unwrap();
    result[0].description = "mutated".into();
    assert_eq!(second.search("version", None).unwrap()[0].description, "neutral");
    let mut changed = tool.clone();
    changed.input_schema = json!({"description":"newkeyword"});
    let schema = build(vec![changed.clone()], &mut cache);
    assert_eq!(cache.rebuilds, 2);
    assert!(first.search("newkeyword", None).unwrap().is_empty());
    assert_eq!(schema.search("newkeyword", None).unwrap().len(), 1);
    changed.source = vw_api_types::tools::ToolSource::Client;
    let source = build(vec![changed.clone()], &mut cache);
    assert_eq!(cache.rebuilds, 3);
    assert_eq!(source.search("client", None).unwrap().len(), 1);
    assert!(schema.search("client", None).unwrap().is_empty());
    changed.exposure = vw_api_types::tools::ToolExposure::Hidden;
    let hidden = build(vec![changed], &mut cache);
    assert_eq!(cache.rebuilds, 4);
    assert!(hidden.search("newkeyword", None).unwrap().is_empty());
    let empty = build(vec![], &mut cache);
    assert_eq!(cache.rebuilds, 5);
    assert!(empty.search("version", None).unwrap().is_empty());
    let mut mirror = tool.clone();
    mirror.parameters = json!({"description":"changed mirror"});
    build(vec![mirror], &mut cache);
    assert_eq!(cache.rebuilds, 6);
    let mut security = tool;
    security.destructive = true;
    let latest = build(vec![security], &mut cache);
    assert_eq!(cache.rebuilds, 7);
    assert!(latest.search("version", None).unwrap()[0].destructive);
    for index in 0..10 {
        build(vec![spec(&format!("mcp__eviction__{index}"), "neutral")], &mut cache);
    }
    assert_eq!(cache.entries.len(), 8);
}

#[test]
fn dynamic_tools_search_schema_recursion_and_payloads_are_bounded() {
    let mut tool = spec("mcp__bounds__tool", "neutral");
    let mut nested = json!({"description":"beyonddepth"});
    for _ in 0..20 {
        nested = json!({"items":nested});
    }
    tool.input_schema = json!({"items":nested, "default":"secretdefault", "examples":["secretexample"],
        "description":format!("{} tailkeyword", "x".repeat(4096))});
    let catalog = ToolDiscoveryCatalog::new(vec![tool]);
    for query in ["beyonddepth", "secretdefault", "secretexample", "tailkeyword"] {
        assert!(catalog.search(query, None).unwrap().is_empty(), "{query}");
    }
    let mut wide = spec("mcp__bounds__wide", "neutral");
    wide.input_schema = json!({"anyOf":vec![json!({"description":"bounded"}); 5000]});
    let catalog = ToolDiscoveryCatalog::new(vec![wide]);
    assert_eq!(catalog.snapshot.documents[0].terms["bounded"], 4095);
}

#[test]
fn dynamic_tools_search_depth_cutoff_still_consumes_node_budget() {
    let properties: serde_json::Map<String, serde_json::Value> = (0..5000)
        .map(|index| (format!("field{index:04}"), json!({"description":"cutoff"})))
        .collect();
    let mut schema = json!({"properties": properties});
    for _ in 0..15 {
        schema = json!({"items":schema});
    }
    let mut terms = std::collections::BTreeMap::new();
    let mut budget = 4096;
    super::append_schema_terms(&schema, 0, &mut budget, &mut terms);
    assert_eq!(budget, 0);
    assert_eq!(terms.len(), 4080);
    assert!(!terms.contains_key("cutoff"));
    assert!(!terms.contains_key("field4999"));
}
