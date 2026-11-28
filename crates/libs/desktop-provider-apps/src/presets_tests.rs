use super::presets::{Availability, Preset, entries};
use super::state::{codex, desktop, edit};
use app_model_catalog::preset_search::search_presets;
use vw_api_app_identity::AppId;
use vw_api_app_provider_base::basic::ProviderBasic;

fn preset(app: AppId, name: &str) -> Preset {
    entries(app).into_iter().find(|entry| entry.name == name).unwrap().preset
}

#[test]
fn claude_desktop_groups_versions_and_keeps_routing_separate_from_direct_creation() {
    let entries = entries(AppId::ClaudeDesktop);
    let matches = search_presets(&entries, "moonshot", |_| None);
    assert!(matches.iter().any(|hit| {
        hit.group.versions.iter().any(|entry| {
            entry.name == "Kimi" && entry.preset.availability() == Availability::Routing
        }) && hit.group.versions.len() > 1
    }));
    let official = preset(AppId::ClaudeDesktop, "Claude Desktop Official");
    assert!(official.availability() == Availability::Official("claude-desktop-official"));
    assert_eq!(official.account_label(), Some("用 Claude 账号登录"));
    for (name, label) in [
        ("Codex", "用 ChatGPT 账号登录"),
        ("GitHub Copilot", "用 GitHub 账号登录"),
        ("xAI (Grok)", "用 xAI 账号登录"),
    ] {
        let routed = preset(AppId::ClaudeDesktop, name);
        assert_eq!(routed.account_label(), Some(label));
        assert!(routed.availability() == Availability::Routing);
    }

    let direct = preset(AppId::ClaudeDesktop, "DigitalOcean");
    assert!(direct.availability() == Availability::Direct);
    let mut form = desktop::Form::new("fixture".into());
    direct.fill_desktop(&mut form);
    assert_eq!(form.name, "DigitalOcean");
    assert!(form.base_url.starts_with("https://"));
    assert!(form.key.is_empty());
    assert!(!form.apply);
}

#[test]
fn codex_preset_prefills_a_credential_free_native_draft() {
    let preset = preset(AppId::Codex, "Kimi");
    assert!(preset.availability() == Availability::Direct);
    let mut form = edit::EditForm::new(
        edit::Mode::Create,
        "fixture".into(),
        ProviderBasic { name: String::new(), category: None, notes: None, website_url: None },
        None,
    );
    form.codex = Some(codex::Draft::new());
    preset.fill_editor(&mut form);
    let draft = form.codex.unwrap();
    let config: toml::Value = toml::from_str(&draft.toml.text()).unwrap();
    assert_eq!(config["model_providers"]["vendor"]["wire_api"].as_str(), Some("responses"));
    assert_eq!(
        config["model_providers"]["vendor"]["base_url"].as_str(),
        Some("https://api.moonshot.cn/v1")
    );
    assert!(!draft.models.unwrap().is_empty());
    assert!(form.key.is_empty());
    assert!(!draft.has_saved_key);
    assert!(!draft.apply);
}

#[test]
fn claude_desktop_cloudflare_requires_a_concrete_account_id() {
    let cloudflare = preset(AppId::ClaudeDesktop, "Cloudflare AI Gateway");
    assert!(cloudflare.availability() == Availability::Direct);
    assert!(cloudflare.desktop_endpoint_needs_account_id());
    let mut form = desktop::Form::new("fixture".into());
    cloudflare.fill_desktop(&mut form);
    assert!(form.endpoint_needs_account_id());
    form.base_url = form.base_url.replace("YOUR_ACCOUNT_ID", "fixture-account");
    assert!(!form.endpoint_needs_account_id());
    assert!(form.key.is_empty());
}

#[test]
fn loopback_presets_keep_app_specific_routing_requirements() {
    for (name, port) in
        [("llama.cpp", 9931), ("LM Studio", 1234), ("Ollama", 11434), ("vLLM", 8000)]
    {
        for app in [AppId::Claude, AppId::ClaudeDesktop, AppId::Codex] {
            let preset = preset(app, name);
            let endpoint = url::Url::parse(preset.local_endpoint().unwrap()).unwrap();
            assert_eq!(endpoint.port(), Some(port));
            let expected = if app == AppId::ClaudeDesktop {
                Availability::Routing
            } else {
                Availability::Direct
            };
            assert!(preset.availability() == expected);
        }
    }
}
