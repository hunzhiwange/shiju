use super::desktop_mapping::*;
use serde_json::{Value, json};
use vw_api_types::app_providers::desktop_mapping::*;
fn row(model: &str) -> Value {
    json!({"name":"Fixture","settingsConfig":{"env":{"ANTHROPIC_BASE_URL":"https://fixture.test","ANTHROPIC_AUTH_TOKEN":"fake-secret","ANTHROPIC_MODEL":model}},"notes":"keep","inFailoverQueue":true})
}
#[test]
fn cc04_desktop_mapping_imports_only_compatible_rows() {
    let direct = import_row(&row("claude-sonnet-5")).unwrap().unwrap();
    assert_eq!(direct["meta"]["claudeDesktopMode"], "direct");
    assert_eq!(direct["notes"], "keep");
    assert_eq!(direct["inFailoverQueue"], false);
    for invalid in [
        json!({"name":"Official","settingsConfig":{"env":{}}}),
        json!({"name":"Bad","settingsConfig":{"env":{"ANTHROPIC_BASE_URL":"https://fixture.test"}}}),
    ] {
        assert!(import_row(&invalid).unwrap().is_none());
    }
    let mut unsupported = row("claude-sonnet-5");
    unsupported["meta"] = json!({"authBinding":{"accountId":"fake"}});
    assert!(import_row(&unsupported).unwrap().is_none());
}
#[test]
fn cc04_desktop_mapping_rejects_invalid_model_id() {
    for (mode, id, model) in [
        (DesktopMappingMode::Proxy, "gpt-5", "kimi-k2"),
        (DesktopMappingMode::Proxy, "claude-sonnet-", "kimi-k2"),
        (DesktopMappingMode::Proxy, "claude-sonnet-5", "bad\nmodel"),
        (DesktopMappingMode::Direct, "claude-sonnet-5", "kimi-k2"),
        (DesktopMappingMode::Proxy, "claude-sonnet-5", "kimi[1M]"),
    ] {
        let mapping = DesktopMapping {
            mode,
            routes: [(
                id.into(),
                DesktopModelRoute { model: model.into(), label_override: None, supports_1m: false },
            )]
            .into(),
        };
        assert!(validate(&mapping).is_err(), "accepted {mapping:?}");
    }
    assert!(
        validate(&DesktopMapping { mode: DesktopMappingMode::Proxy, routes: Default::default() })
            .is_err()
    );
    assert!(
        validate(&DesktopMapping { mode: DesktopMappingMode::Direct, routes: Default::default() })
            .is_ok()
    );
}
#[test]
fn cc04_desktop_mapping_import_translates_marker_merges_duplicates_and_labels() {
    let mut source = row("unused");
    source["settingsConfig"]["env"]["ANTHROPIC_DEFAULT_SONNET_MODEL"] = json!("kimi-k2[1M]");
    source["settingsConfig"]["env"]["ANTHROPIC_DEFAULT_OPUS_MODEL"] = json!("kimi-k2");
    source["settingsConfig"]["env"]["ANTHROPIC_DEFAULT_OPUS_MODEL_NAME"] = json!("Kimi");
    let mapped = import_row(&source).unwrap().unwrap();
    assert_eq!(mapped["meta"]["claudeDesktopMode"], "proxy");
    let routes = &mapped["meta"]["claudeDesktopModelRoutes"];
    assert_eq!(routes.as_object().unwrap().len(), 1);
    assert_eq!(routes["claude-sonnet-5"]["model"], "kimi-k2");
    assert_eq!(routes["claude-sonnet-5"]["supports1m"], true);
    assert_eq!(routes["claude-sonnet-5"]["labelOverride"], "Kimi");
    assert_eq!(
        defaults().iter().map(|r| r.route_id.as_str()).collect::<Vec<_>>(),
        ["claude-sonnet-5", "claude-opus-5", "claude-haiku-4-5", "claude-fable-5"]
    );
}
#[test]
fn cc04_desktop_mapping_supports_generic_catalog_and_rejects_unsafe_imports() {
    let generic = json!({"notes":"retain generic notes","name":"Generic","protocol":"anthropic","apiKey":"fake-key","baseUrl":"https://fixture.test","model":"claude-opus-5"});
    let imported = import_row(&generic).unwrap().unwrap();
    assert_eq!(imported["meta"]["claudeDesktopMode"], "direct");
    assert_eq!(imported["notes"], "retain generic notes");
    assert_eq!(imported["claude_desktop"]["config"]["authToken"], "fake-key");
    let mut wrong_type = row("claude-sonnet-5");
    wrong_type["settingsConfig"]["env"]["ANTHROPIC_MODEL"] = json!(7);
    assert!(import_row(&wrong_type).is_err());
    wrong_type = row("claude-sonnet-5");
    wrong_type["meta"] = json!({"apiFormat":7});
    assert!(import_row(&wrong_type).is_err());
    let mut bad = row("bad\nmodel");
    assert!(import_row(&bad).is_err());
    bad = row("claude-sonnet-5");
    bad["settingsConfig"]["env"]["ANTHROPIC_BASE_URL"] = json!("http://remote.test");
    assert!(import_row(&bad).unwrap().is_none());
    bad = row("claude-sonnet-5");
    bad["settingsConfig"]["env"]["ANTHROPIC_AUTH_TOKEN"] = json!("[redacted]");
    assert!(import_row(&bad).unwrap().is_none());
    let mut api_key = row("claude-opus-5");
    let env = api_key["settingsConfig"]["env"].as_object_mut().unwrap();
    env.remove("ANTHROPIC_AUTH_TOKEN");
    env.insert("ANTHROPIC_API_KEY".into(), json!("fake-key"));
    assert_eq!(import_row(&api_key).unwrap().unwrap()["meta"]["claudeDesktopMode"], "proxy");
}

#[test]
fn aggregation_t30_desktop_mapping_preserves_contract() {
    use catalog::DesktopModelCatalog;
    let mapping = mapped_routes(&[
        ("claude-opus-5", "claude-opus-5.5", None, false),
        ("claude-sonnet-5", "kimi-k2", None, false),
        ("claude-haiku-4-5", "glm-5", None, false),
    ]);
    let catalog = DesktopModelCatalog::new(member("primary", "Primary", &mapping), &[]).unwrap();
    assert_eq!(
        catalog.models().iter().map(|model| model.id.as_str()).collect::<Vec<_>>(),
        ["claude-sonnet-5", "claude-opus-5-5", "claude-haiku-5-5"]
    );
    for (request, upstream) in [
        ("claude-opus-5", "claude-opus-5.5"),
        ("claude-opus-5-5", "claude-opus-5.5"),
        ("claude-fable-5[1m]", "claude-opus-5.5"),
        ("claude-haiku-4-5-20251001", "glm-5"),
        ("anthropic/claude-sonnet-4-5-20250101", "kimi-k2"),
    ] {
        let mapped = catalog.resolve(request).unwrap();
        assert_eq!(mapped.provider_id, "primary");
        assert_eq!(mapped.upstream_model, upstream, "{request}");
    }
}

fn mapped_routes(routes: &[(&str, &str, Option<&str>, bool)]) -> DesktopMapping {
    DesktopMapping {
        mode: DesktopMappingMode::Proxy,
        routes: routes
            .iter()
            .map(|&(id, upstream, label, supports_1m)| {
                (
                    id.into(),
                    DesktopModelRoute {
                        model: upstream.into(),
                        label_override: label.map(str::to_owned),
                        supports_1m,
                    },
                )
            })
            .collect(),
    }
}

fn member<'a>(
    id: &'a str,
    name: &'a str,
    mapping: &'a DesktopMapping,
) -> catalog::DesktopMappingMember<'a> {
    catalog::DesktopMappingMember { provider_id: id, key: id, name, mapping }
}

#[test]
fn aggregation_t30_desktop_numeric_id_routes_only_to_its_member() {
    use catalog::{DesktopMappingError, DesktopModelCatalog};
    let primary = mapped_routes(&[
        ("claude-opus-5", "opus-remote", None, false),
        ("claude-fable-5", "fable-remote", None, false),
    ]);
    let relay = mapped_routes(&[
        ("claude-sonnet-5", "kimi-k2", Some("Kimi"), false),
        ("claude-haiku-4-5", "kimi-k2", Some("Duplicate"), true),
    ]);
    let catalog = DesktopModelCatalog::new(
        member("primary", "Primary", &primary),
        &[member("relay", "Relay", &relay)],
    )
    .unwrap();
    let discovered = &catalog.models()[2];
    assert_eq!(catalog.models().len(), 3);
    assert_eq!(discovered.id, "claude-fable-ccs-081702");
    assert_eq!(discovered.label, "Relay / Kimi");
    assert!(discovered.supports_1m);
    for request in ["claude-fable-ccs-081702", "claude-fable-ccs-081702[1M]"] {
        assert_eq!(catalog.resolve(request), Ok(discovered));
        assert_eq!(discovered.provider_id, "relay");
        assert_eq!(discovered.upstream_model, "kimi-k2");
    }
    for request in ["claude-fable-ccs-000000", "gpt-5", "claude-haiku-5", "claude-fable-unknown"] {
        assert_eq!(catalog.resolve(request), Err(DesktopMappingError::UnknownModel), "{request}");
    }
    for request in [
        "claude-fable-ccs-12345",
        "CLAUDE-FABLE-CCS-081702",
        "anthropic/claude-fable-ccs-081702",
        "",
    ] {
        assert_eq!(catalog.resolve(request), Err(DesktopMappingError::InvalidModelId), "{request}");
    }

    let without_relay =
        DesktopModelCatalog::new(member("primary", "Primary", &primary), &[]).unwrap();
    assert_eq!(without_relay.resolve(&discovered.id), Err(DesktopMappingError::UnknownModel));
    let cached_default = without_relay.resolve("claude-fable-ccs-441717").unwrap();
    assert_eq!(
        (cached_default.provider_id.as_str(), cached_default.upstream_model.as_str()),
        ("primary", "opus-remote")
    );
}

#[test]
fn aggregation_t30_desktop_collision_uses_discovery_order_for_requests() {
    use catalog::DesktopModelCatalog;
    let primary = mapped_routes(&[("claude-sonnet-5", "default-model", None, false)]);
    let first = mapped_routes(&[("claude-sonnet-5", "model-4710", None, false)]);
    let second = mapped_routes(&[("claude-sonnet-5", "model-911", None, true)]);
    let catalog = DesktopModelCatalog::new(
        member("primary", "Primary", &primary),
        &[member("relay-a", "A", &first), member("relay-b", "B", &second)],
    )
    .unwrap();
    let code = "claude-fable-ccs-957548";
    assert_eq!(catalog.models().len(), 2);
    let winner = &catalog.models()[1];
    assert_eq!(winner.id, code);
    assert_eq!(winner.provider_id, "relay-a");
    assert!(!winner.supports_1m);
    assert_eq!(catalog.resolve(code), Ok(winner));

    let catalog = DesktopModelCatalog::new(
        member("relay-a", "A", &first),
        &[member("relay-b", "B", &second)],
    )
    .unwrap();
    assert_eq!(catalog.resolve(code).unwrap().provider_id, "relay-b");
}

#[test]
fn aggregation_t30_desktop_catalog_rejects_implicit_or_reserved_configuration() {
    use catalog::{DesktopMappingError, DesktopModelCatalog};
    let empty = DesktopMapping { mode: DesktopMappingMode::Direct, routes: Default::default() };
    let reserved = mapped_routes(&[("claude-fable-ccs-123456", "remote", None, true)]);
    for mapping in [&empty, &reserved] {
        assert_eq!(
            DesktopModelCatalog::new(member("primary", "Primary", mapping), &[]).unwrap_err(),
            DesktopMappingError::InvalidConfiguration
        );
    }
}

#[test]
fn aggregation_t30_legacy_opus_id_prefers_its_compatible_role_id() {
    use catalog::DesktopModelCatalog;

    let mapping = mapped_routes(&[
        ("claude-opus-4-7", "older-opus", None, false),
        ("claude-opus-5", "current-opus", None, false),
    ]);
    let catalog = DesktopModelCatalog::new(member("primary", "Primary", &mapping), &[]).unwrap();

    assert_eq!(catalog.resolve("claude-opus-4-8").unwrap().upstream_model, "current-opus");
}
