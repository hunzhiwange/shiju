use serde_json::{Value, json};

use super::claude_catalog::{ClaudeCatalogError, ClaudeCatalogMember, ClaudeModelCatalog};

fn member<'a>(id: &'a str, key: &'a str, record: &'a Value) -> ClaudeCatalogMember<'a> {
    ClaudeCatalogMember { provider_id: id, key, record }
}

#[test]
fn aggregation_t08_claude_catalog_preserves_contract() {
    let mapped = json!({
        "name": "Mapped",
        "settingsConfig": {"env": {
            "ANTHROPIC_MODEL": " vendor/shared ",
            "ANTHROPIC_DEFAULT_OPUS_MODEL": "vendor/shared[1m]",
            "ANTHROPIC_DEFAULT_OPUS_MODEL_NAME": "Shared",
            "ANTHROPIC_DEFAULT_FABLE_MODEL": "fallback",
            "CLAUDE_CODE_MAX_CONTEXT_TOKENS": "128000"
        }}
    });
    let explicit = json!({
        "name": "Listed",
        "meta": {"stackModels": [
            {"model": "  "},
            {"model": "vendor/shared", "displayName": " First "},
            {"model": "listed-second"},
            {"model": "vendor/shared[1m]", "displayName": "Ignored"},
            {"model": "listed-second", "oneM": true}
        ]},
        "settingsConfig": {"env": {"ANTHROPIC_MODEL": "ignored"}}
    });
    let catalog = ClaudeModelCatalog::new(
        &[member("mapped", "m", &mapped), member("listed", "l", &explicit)],
        Some("listed"),
    )
    .unwrap();
    let models = catalog.models();
    let ids: Vec<_> = models.iter().map(|model| model.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "ccs-claude-l--vendor/shared[1M]",
            "ccs-claude-m--vendor/shared[1M]",
            "ccs-claude-m--fallback",
            "ccs-claude-l--listed-second[1M]"
        ]
    );
    assert_eq!(catalog.default_model(), models.first());
    assert_eq!(models[0].provider_id, "listed");
    assert_eq!(models[0].label, "First（Listed）");
    assert_eq!(models[0].upstream_model, "vendor/shared[1M]");
    assert!(models[0].supports_1m);
    assert_eq!(models[1].name, "Shared");
    assert_eq!(models[2].description, "Mapped · fallback · 128K");
    assert_eq!(models[2].window, 128_000);
}

#[test]
fn aggregation_t08_explicit_empty_default_never_falls_back() {
    let empty = json!({
        "name": "Empty",
        "meta": {"stackModels": []},
        "settingsConfig": {"env": {"ANTHROPIC_MODEL": "ignored"}}
    });
    let unset = json!({
        "name": "Unset",
        "meta": {"stackModels": null},
        "settingsConfig": {"env": {"ANTHROPIC_MODEL": "mapped"}}
    });
    let members = [member("empty", "e", &empty), member("unset", "u", &unset)];
    for default in [Some("empty"), None] {
        let catalog = ClaudeModelCatalog::new(&members, default).unwrap();
        assert_eq!(catalog.default_model(), None);
        let [model] = catalog.models() else { panic!("expected only the mapped model") };
        assert_eq!(model.id, "ccs-claude-u--mapped");
        assert_eq!(model.window, 200_000);
    }
}

#[test]
fn aggregation_t08_invalid_explicit_list_rejects_publication() {
    let invalid = json!({
        "name": "Invalid",
        "meta": {"stackModels": {"model": "invalid"}},
        "settingsConfig": {"env": {"ANTHROPIC_MODEL": "ignored"}}
    });
    assert_eq!(
        ClaudeModelCatalog::new(&[member("invalid", "i", &invalid)], Some("invalid")),
        Err(ClaudeCatalogError::InvalidConfiguration)
    );
}

#[test]
fn aggregation_t08_rejects_ambiguous_members_and_unknown_default() {
    let record = json!({
        "name": "Provider",
        "settingsConfig": {"env": {"ANTHROPIC_MODEL": "model"}}
    });
    let colliding = [member("one", "key", &record), member("two", "key", &record)];
    assert_eq!(ClaudeModelCatalog::new(&colliding, None), Err(ClaudeCatalogError::InvalidMember));

    let members = [member("one", "key", &record)];
    assert_eq!(
        ClaudeModelCatalog::new(&members, Some("missing")),
        Err(ClaudeCatalogError::InvalidMember)
    );
}
