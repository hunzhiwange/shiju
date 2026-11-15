use super::codex::{ProjectionError, project};
use serde_json::{Value, json};
fn row(extra: &str) -> Value {
    json!({"auth":{"OPENAI_API_KEY":"fake-route-key"},"config":format!("model_provider='vendor'\nmodel='new'\n[model_providers.vendor]\nbase_url='https://fixture.test/v1'\nwire_api='responses'\n{extra}")})
}
fn doc(text: &str) -> Value {
    toml::from_str(text).unwrap()
}
#[test]
fn cc04_codex_key_is_in_active_route_table() {
    let live = "# retain comment\nmodel_provider='old'\nexperimental_bearer_token='old-top'\n[model_providers.user]\nbase_url='https://user.test'\n[projects.fixture]\ntrust_level='trusted'\n";
    let input = row("");
    let result = project(live, &input, None, false).unwrap();
    let output = doc(result.config());
    assert_eq!(output["model_provider"], "custom");
    assert_eq!(output["model_providers"]["custom"]["experimental_bearer_token"], "fake-route-key");
    assert!(output.get("experimental_bearer_token").is_none());
    assert!(output["model_providers"].get("vendor").is_none());
    assert_eq!(output["model_providers"]["user"], doc(live)["model_providers"]["user"]);
    assert_eq!(output["projects"], doc(live)["projects"]);
    assert_eq!(result.config().matches("fake-route-key").count(), 1);
    assert!(!format!("{result:?}").contains("fake-route-key"));
    assert_eq!(input, row(""));
}
#[test]
fn cc04_codex_profile_override_rejects_without_side_effects() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("config.toml");
    for override_ in [
        "model_provider='other'",
        "openai_base_url='https://override.test'",
        "experimental_bearer_token='profile-key'",
    ] {
        let live = format!("profile='active'\n[profiles.active]\n{override_}\n");
        std::fs::write(&file, &live).unwrap();
        let input = row("");
        assert_eq!(
            project(&std::fs::read_to_string(&file).unwrap(), &input, None, false).unwrap_err(),
            ProjectionError::ProfileOverride
        );
        assert_eq!(std::fs::read_to_string(&file).unwrap(), live);
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
        assert_eq!(input, row(""));
    }
}
#[test]
fn cc04_codex_native_rejects_keyless_fallback() {
    let mut input = row("requires_openai_auth=true\n");
    input["auth"] = json!({});
    assert_eq!(project("", &input, None, false).unwrap_err(), ProjectionError::MissingCredential);
}
#[test]
fn cc04_codex_native_preserves_table_auth_without_injecting_key() {
    for extra in [
        "env_key='VENDOR_KEY'",
        "http_headers={Authorization='Bearer fake-header'}",
        "env_http_headers={Authorization='VENDOR_AUTH'}",
        "auth={command='fixture'}",
        "aws={region='fixture'}",
    ] {
        let input = row(extra);
        let result = project("", &input, None, true).unwrap();
        let output = doc(result.config());
        let selected = &output["model_providers"]["custom"];
        assert!(selected.get("experimental_bearer_token").is_none());
        let original = doc(input["config"].as_str().unwrap());
        for (key, value) in original["model_providers"]["vendor"].as_object().unwrap() {
            assert_eq!(selected[key], *value);
        }
        assert!(!result.config().contains("fake-route-key"));
    }
}

#[test]
fn cc04_codex_native_normalizes_legacy_inline_and_embedded_credentials() {
    for config in [
        "openai_base_url='https://fixture.test/v1'\nexperimental_bearer_token='fake-embedded'\n",
        "base_url='https://fixture.test/v1'\nexperimental_bearer_token='fake-embedded'\n",
        "model_provider='vendor'\nmodel_providers={vendor={base_url='https://fixture.test/v1',experimental_bearer_token='fake-embedded'}}\n",
        "model_provider='openai'\n[model_providers.openai]\nbase_url='https://fixture.test/v1'\nexperimental_bearer_token='fake-embedded'\n",
    ] {
        let input = json!({"auth":{}, "config":config});
        for login in [false, true] {
            let result = project("", &input, None, login).unwrap();
            let output = doc(result.config());
            assert_eq!(output["model_provider"], "custom");
            assert_eq!(
                output["model_providers"]["custom"]["experimental_bearer_token"],
                "fake-embedded"
            );
            assert_eq!(output["model_providers"]["custom"]["requires_openai_auth"], login);
            assert_eq!(result.config().matches("fake-embedded").count(), 1);
            assert!(output.get("experimental_bearer_token").is_none());
            assert!(output.get("openai_base_url").is_none());
            assert!(output.get("base_url").is_none());
        }
    }
}
#[test]
fn cc04_codex_native_rejects_display_placeholders_and_invalid_shape() {
    for key in ["CC_SWITCH_PENDING_KEY", "[redacted]", "<redacted>", "***", "•••"] {
        let mut input = row("");
        input["auth"]["OPENAI_API_KEY"] = json!(key);
        assert_eq!(
            project("", &input, None, false).unwrap_err(),
            ProjectionError::MissingCredential
        );
        input["auth"] = json!({});
        input["config"] = json!(format!(
            "model_provider='vendor'\n[model_providers.vendor]\nbase_url='https://fixture.test'\nexperimental_bearer_token='{key}'\n"
        ));
        assert_eq!(
            project("", &input, None, false).unwrap_err(),
            ProjectionError::MissingCredential
        );
    }
    for input in [
        json!({"auth":{},"config":false}),
        json!({"auth":5,"config":""}),
        json!({"auth":{"OPENAI_API_KEY":5},"config":""}),
    ] {
        assert_eq!(project("", &input, None, false).unwrap_err(), ProjectionError::InvalidConfig);
    }
    assert_eq!(
        project("model=", &row(""), None, false).unwrap_err(),
        ProjectionError::InvalidConfig
    );
}
#[test]
fn cc04_codex_native_preserves_inactive_profiles_and_same_route() {
    let live = "profile='active'\n[profiles.active]\nmodel_provider='custom'\n[profiles.inactive]\nmodel_provider='other'\n[model_providers.other]\nbase_url='https://other.test'\n";
    let result = project(live, &row(""), None, false).unwrap();
    assert_eq!(doc(result.config())["profiles"], doc(live)["profiles"]);
    assert_eq!(
        doc(result.config())["model_providers"]["other"],
        doc(live)["model_providers"]["other"]
    );
}

#[test]
fn cc04_codex_native_default_profile_does_not_override_route() {
    for profile in [
        "model_provider='openai'\n",
        "model_provider=' openai '\nopenai_base_url=' '\nexperimental_bearer_token=''\n",
    ] {
        let live = format!("profile='active'\n[profiles.active]\n{profile}");
        let input = json!({"auth":{},"config":"model='new'\n"});
        let result = project(&live, &input, None, false).unwrap();
        let output = doc(result.config());
        assert!(output.get("model_provider").is_none());
        assert_eq!(output["profiles"], doc(&live)["profiles"]);
        assert_eq!(output["model"], "new");
    }
    // Codex trims the selected profile name before looking up its route override.
    let live = "profile=' active '\n[profiles.active]\nmodel_provider='other'\n";
    assert_eq!(project(live, &row(""), None, false).unwrap_err(), ProjectionError::ProfileOverride);
}
