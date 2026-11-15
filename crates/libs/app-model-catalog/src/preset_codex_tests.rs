use super::{
    preset_catalog::{codex_preset, codex_presets},
    preset_codex::{CodexPreset, CodexPresetApiFormat, CodexPresetAuthProvider},
};
use vw_api_types::agent_apps::AppId;

fn preset(name: &str) -> &'static CodexPreset {
    codex_preset(AppId::Codex, name).unwrap_or_else(|| panic!("missing Codex preset {name}"))
}

#[test]
fn catalog_presets_codex_keeps_plan_vision_declaration() {
    let modalities = |name, endpoint, model| {
        codex_preset(AppId::Codex, name)
            .and_then(|row| row.model_at_endpoint(endpoint, model))
            .and_then(|row| row.input_modalities.as_deref())
    };
    assert_eq!(
        modalities(
            "Baidu Qianfan Token Plan",
            "https://qianfan.baidubce.com/v2/tokenplan/personal",
            "deepseek-v4-flash",
        )
        .map(|values| values == ["text"]),
        Some(true),
    );
    for name in [
        "Baidu Qianfan Token Plan",
        "FluxA Token Plan",
        "Tencent Token Plan Enterprise Pro",
        "Tencent Token Plan Enterprise Pro (Intl)",
    ] {
        let plan = preset(name);
        let mut rows = plan.model_catalog.iter().filter(|row| row.model.starts_with("deepseek-v4"));
        let first = rows.next().expect("token plan includes DeepSeek V4");
        for row in std::iter::once(first).chain(rows) {
            assert_eq!(row.input_modalities.as_deref().unwrap(), ["text"]);
        }
        assert!(
            plan.model_at_endpoint(plan.base_url.as_deref().unwrap(), "glm-5.3-flash").is_none()
        );
    }

    for (name, endpoint) in [
        ("Zhipu GLM", "https://open.bigmodel.cn/api/v1"),
        ("Zhipu GLM en", "https://api.z.ai/api/v1"),
    ] {
        let zhipu = preset(name);
        let flash = zhipu.model_at_endpoint(endpoint, "glm-5.3-flash").unwrap();
        assert_eq!(flash.input_modalities.as_deref().unwrap(), ["text", "image"]);
        assert_eq!(flash.context_window, Some(1_048_576));
        assert_eq!(flash.reasoning_levels.as_deref().unwrap(), ["low", "high", "max"]);
        assert_eq!(flash.default_reasoning_level.as_deref(), Some("max"));
        assert_eq!(flash.supports_parallel_tool_calls, Some(true));
        let standard = zhipu.model_at_endpoint(endpoint, "glm-5.3").unwrap();
        assert_eq!(standard.input_modalities.as_deref().unwrap(), ["text"]);
        assert!(zhipu.model_at_endpoint(endpoint, "deepseek-v4-flash").is_none());
    }

    let deepseek = preset("DeepSeek");
    assert_eq!(deepseek.model_catalog.len(), 1);
    let flash = deepseek.model_at_endpoint("https://api.deepseek.com", "deepseek-flash").unwrap();
    assert_eq!(flash.input_modalities.as_deref().unwrap(), ["text", "image"]);
}

#[test]
fn catalog_presets_codex_keeps_model_metadata_within_the_selected_deployment() {
    let qianfan = preset("Baidu Qianfan Token Plan");
    let fluxa = preset("FluxA Token Plan");
    let qianfan_url = "https://qianfan.baidubce.com/v2/tokenplan/personal";
    let fluxa_url = "https://api.baiduqianfan.ai/v2/tokenplan/team";
    assert_eq!(
        qianfan.model_at_endpoint(qianfan_url, "glm-5.2").unwrap().context_window,
        Some(1_048_576)
    );
    assert_eq!(
        fluxa.model_at_endpoint(fluxa_url, "glm-5.2").unwrap().context_window,
        Some(500_000)
    );
    assert!(qianfan.model_at_endpoint(fluxa_url, "glm-5.2").is_none());
    assert!(fluxa.model_at_endpoint(qianfan_url, "glm-5.2").is_none());

    let pro = preset("Tencent Token Plan Enterprise Pro");
    let lite = preset("Tencent Token Plan Enterprise Lite");
    let alternate = "https://tokenhub-intl.tencentmaas.com/plan/v3";
    assert!(pro.endpoint_candidates.iter().any(|endpoint| endpoint == alternate));
    assert!(lite.model_at_endpoint(alternate, "auto").is_some());
    assert!(lite.model_at_endpoint(alternate, "glm-5.3").is_none());
    let pro_model = pro.model_at_endpoint(alternate, "glm-5.3").unwrap();
    assert_eq!(pro_model.default_reasoning_level.as_deref(), Some("high"));
    assert!(pro_model.input_modalities.is_none());
}

#[test]
fn catalog_presets_codex_keeps_managed_oauth_keyless() {
    let managed: Vec<_> = codex_presets(AppId::Codex).filter(|row| row.requires_oauth).collect();
    assert_eq!(
        managed.iter().map(|row| row.name.as_str()).collect::<Vec<_>>(),
        ["GitHub Copilot", "xAI (Grok) OAuth"]
    );
    assert!(managed.iter().all(|row| row.requires_openai_auth == Some(false)));
    let copilot = preset("GitHub Copilot");
    assert_eq!(copilot.auth_provider, Some(CodexPresetAuthProvider::GithubCopilot));
    assert_eq!(copilot.api_format, Some(CodexPresetApiFormat::OpenaiResponses));
    assert_eq!(copilot.default_model.as_deref(), Some("gpt-6-astra"));
    assert_eq!(copilot.default_reasoning_effort.as_deref(), Some("high"));
    assert_eq!(copilot.model_catalog.len(), 5);
    let astra = copilot.model_at_endpoint("https://api.githubcopilot.com", "gpt-6-astra").unwrap();
    assert_eq!(astra.context_window, Some(272_000));
    assert_eq!(astra.input_modalities.as_deref().unwrap(), ["text"]);
    assert_eq!(
        astra.reasoning_levels.as_deref().unwrap(),
        ["low", "medium", "high", "xhigh", "max"]
    );
    assert_eq!(astra.supports_parallel_tool_calls, Some(true));
    assert_eq!(preset("xAI (Grok) OAuth").auth_provider, Some(CodexPresetAuthProvider::XaiOauth));
    assert_eq!(preset("DeepSeek").requires_openai_auth, Some(true));
}

#[test]
fn catalog_presets_codex_lists_source_rows_only_in_codex() {
    let all: Vec<_> = codex_presets(AppId::Codex).collect();
    assert_eq!(all.len(), 137);
    assert_eq!(all.iter().map(|row| row.model_catalog.len()).sum::<usize>(), 213);
    assert_eq!(all.first().unwrap().name, "OpenAI Official");
    let official = preset("OpenAI Official");
    assert!(official.is_official);
    assert_eq!(official.auth_provider, Some(CodexPresetAuthProvider::CodexOauth));
    assert!(official.base_url.is_none());
    assert!(official.default_model.is_none());
    assert!(official.model_catalog.is_empty());
    assert!(preset("Mistral AI").hidden);
    assert!(codex_preset(AppId::Gemini, "OpenAI Official").is_none());
    assert!(codex_presets(AppId::GrokBuild).next().is_none());

    let ppio = preset("PPIO");
    let endpoint = "https://api.ppio.com/openai/v1";
    assert_eq!(ppio.api_format, Some(CodexPresetApiFormat::OpenaiChat));
    assert!(ppio.model_at_endpoint(endpoint, "deepseek/deepseek-v4.1-flash").is_some());
    assert!(ppio.model_at_endpoint(endpoint, "deepseek-v4.1-flash").is_none());
}
