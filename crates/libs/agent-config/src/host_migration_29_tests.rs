use crate::catalog::load_agent_configs;
use serde_json::json;
use std::collections::HashSet;
use config_agent_profile::AgentRuntime;
use config_agent::BUILTIN_AGENT_SPECS;

#[test]
fn agent_catalog_covers_thirty_unique_backends() {
    let catalog = load_agent_configs(&json!({}));
    let backends = catalog
        .values()
        .filter_map(|entry| match &entry.runtime {
            AgentRuntime::Acp { backend } => Some(backend.as_str()),
            AgentRuntime::Native => None,
        })
        .collect::<HashSet<_>>();
    assert_eq!(backends.len(), 33);
    assert!(catalog["main"].builtin);
    assert!(!catalog["acp-snow"].enabled);
    assert_eq!(
        catalog
            .values()
            .filter(|entry| matches!(entry.runtime, AgentRuntime::Acp { .. }) && entry.enabled)
            .count(),
        6
    );
    for backend in acp_agent_registry::built_in_agent_specs().keys() {
        let entry = &catalog[&format!("acp-{backend}")];
        assert!(entry.builtin);
        assert!(!entry.recommended_prompts.is_empty());
    }
}

#[test]
fn agent_catalog_preserves_native_role_limits() {
    let catalog = load_agent_configs(&json!({}));
    for spec in BUILTIN_AGENT_SPECS {
        let entry = &catalog[spec.key];
        assert_eq!(entry.mode, spec.mode);
        assert_eq!(entry.allowed_tools, spec.allowed_tools);
        assert_eq!(entry.allow_as_subagent, spec.allow_as_subagent);
        assert_eq!(entry.runtime, AgentRuntime::Native);
    }
}

#[test]
fn agent_catalog_preserves_official_identity_and_editable_overrides() {
    let config = json!({"agents": {"acp-codex": {
        "provider": "", "model": "", "builtin": false,
        "runtime": {"kind": "native"}, "enabled": false,
        "label": "My CLI", "description": "Edited", "system_prompt": "Rules",
        "recommended_prompts": ["Review"],
        "defaults": {"skills": {"mode": "fixed", "value": []}}
    }}});
    let catalog = load_agent_configs(&config);
    let entry = &catalog["acp-codex"];
    assert!(entry.builtin);
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "codex".into() });
    assert!(!entry.enabled);
    assert_eq!(entry.label.as_deref(), Some("My CLI"));
    assert_eq!(entry.description.as_deref(), Some("Edited"));
    assert_eq!(
        serde_json::to_value(&entry.defaults).unwrap()["skills"],
        json!({"mode": "fixed", "value": []})
    );
    assert_eq!(entry.system_prompt.as_deref(), Some("Rules"));
    assert_eq!(entry.recommended_prompts, ["Review"]);
    let reloaded = load_agent_configs(&json!({"agents": catalog}));
    assert_eq!(
        serde_json::to_value(&reloaded["acp-codex"]).unwrap(),
        serde_json::to_value(entry).unwrap()
    );
}

#[test]
fn agent_catalog_keeps_all_backends_without_installed_clis() {
    const CHILD: &str = "SHIJU_CATALOG_EMPTY_CLI_ENV";
    if std::env::var_os(CHILD).is_some() {
        let catalog = load_agent_configs(&json!({}));
        assert_eq!(catalog.keys().filter(|key| key.starts_with("acp-")).count(), 33);
        assert_eq!(catalog["acp-snow"].runtime, AgentRuntime::Acp { backend: "snow".into() });
        return;
    }
    // Isolate executable lookup from this process and the developer's installed CLIs.
    let empty_home =
        std::env::temp_dir().join(format!("shiju-catalog-empty-{}", std::process::id()));
    std::fs::create_dir_all(&empty_home).unwrap();
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["agent_catalog_keeps_all_backends_without_installed_clis", "--nocapture"])
        .env_clear()
        .env(CHILD, "1")
        .env("PATH", &empty_home)
        .env("HOME", &empty_home)
        .env("USERPROFILE", &empty_home)
        .env("XDG_BIN_HOME", &empty_home)
        .output()
        .unwrap();
    std::fs::remove_dir_all(&empty_home).unwrap();
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stdout));
}

#[test]
fn acp_onboarding_autohand_builtin_preset_metadata() {
    let catalog = load_agent_configs(&json!({}));
    let entry = catalog.get("acp-autohand").expect("official Autohand preset");
    assert!(entry.builtin);
    assert!(!entry.enabled);
    assert!(!entry.allow_as_subagent);
    assert_eq!(entry.label.as_deref(), Some("Autohand Code"));
    assert!(!entry.description.as_ref().unwrap().is_empty());
    assert!(!entry.recommended_prompts.is_empty());
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "autohand".into() });
}

#[test]
fn acp_onboarding_autohand_catalog_binding() {
    let catalog = load_agent_configs(&json!({"agents":{"acp-autohand":{
        "provider":"","model":"","enabled":true,"label":"Selected Autohand","runtime":{"kind":"native"},"builtin":false
    }}}));
    let entry = &catalog["acp-autohand"];
    assert!(entry.enabled && entry.builtin);
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "autohand".into() });
}

#[test]
fn acp_onboarding_deepagents_builtin_preset_metadata() {
    let catalog = load_agent_configs(&json!({}));
    let entry = catalog.get("acp-deepagents").expect("official DeepAgents preset");
    assert!(entry.builtin);
    assert!(!entry.enabled);
    assert!(!entry.allow_as_subagent);
    assert_eq!(entry.label.as_deref(), Some("DeepAgents"));
    assert!(!entry.description.as_ref().unwrap().is_empty());
    assert!(!entry.recommended_prompts.is_empty());
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "deepagents".into() });
}

#[test]
fn acp_onboarding_deepagents_catalog_binding() {
    let catalog = load_agent_configs(&json!({"agents":{"acp-deepagents":{
        "provider":"","model":"","enabled":true,"label":"Selected DeepAgents","runtime":{"kind":"native"},"builtin":false
    }}}));
    let entry = &catalog["acp-deepagents"];
    assert!(entry.enabled && entry.builtin);
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "deepagents".into() });
}
#[test]
fn acp_onboarding_dimcode_builtin_preset_metadata() {
    let catalog = load_agent_configs(&json!({}));
    let entry = catalog.get("acp-dimcode").expect("official DimCode preset");
    assert!(entry.builtin);
    assert!(!entry.enabled);
    assert!(!entry.allow_as_subagent);
    assert_eq!(entry.label.as_deref(), Some("DimCode"));
    assert!(!entry.description.as_ref().unwrap().is_empty());
    assert!(!entry.recommended_prompts.is_empty());
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "dimcode".into() });
}

#[test]
fn acp_onboarding_dimcode_catalog_binding() {
    let catalog = load_agent_configs(&json!({"agents":{"acp-dimcode":{
        "provider":"","model":"","enabled":true,"label":"Selected DimCode","runtime":{"kind":"native"},"builtin":false
    }}}));
    let entry = &catalog["acp-dimcode"];
    assert!(entry.enabled && entry.builtin);
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "dimcode".into() });
}

#[test]
fn acp_onboarding_dirac_builtin_preset_metadata() {
    let catalog = load_agent_configs(&json!({}));
    let entry = catalog.get("acp-dirac").expect("official Dirac preset");
    assert!(entry.builtin);
    assert!(!entry.enabled);
    assert!(!entry.allow_as_subagent);
    assert_eq!(entry.label.as_deref(), Some("Dirac"));
    assert!(!entry.description.as_ref().unwrap().is_empty());
    assert!(!entry.recommended_prompts.is_empty());
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "dirac".into() });
}

#[test]
fn acp_onboarding_dirac_catalog_binding() {
    let catalog = load_agent_configs(&json!({"agents":{"acp-dirac":{
        "provider":"","model":"","enabled":true,"label":"Selected Dirac","runtime":{"kind":"native"},"builtin":false
    }}}));
    let entry = &catalog["acp-dirac"];
    assert!(entry.enabled && entry.builtin);
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "dirac".into() });
}

#[test]
fn acp_onboarding_glm_acp_agent_builtin_preset_metadata() {
    let catalog = load_agent_configs(&json!({}));
    let entry = catalog.get("acp-glm-acp-agent").expect("official GLM Agent preset");
    assert!(entry.builtin);
    assert!(!entry.enabled);
    assert!(!entry.allow_as_subagent);
    assert_eq!(entry.label.as_deref(), Some("GLM Agent"));
    assert!(!entry.description.as_ref().unwrap().is_empty());
    assert!(!entry.recommended_prompts.is_empty());
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "glm-acp-agent".into() });
}

#[test]
fn acp_onboarding_glm_acp_agent_catalog_binding() {
    let catalog = load_agent_configs(&json!({"agents":{"acp-glm-acp-agent":{
        "provider":"","model":"","enabled":true,"label":"Selected GLM Agent","runtime":{"kind":"native"},"builtin":false
    }}}));
    let entry = &catalog["acp-glm-acp-agent"];
    assert!(entry.enabled && entry.builtin);
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "glm-acp-agent".into() });
}

#[test]
fn acp_onboarding_grok_builtin_preset_metadata() {
    let catalog = load_agent_configs(&json!({}));
    let entry = catalog.get("acp-grok").expect("official Grok Build preset");
    assert!(entry.builtin);
    assert!(!entry.enabled);
    assert!(!entry.allow_as_subagent);
    assert_eq!(entry.label.as_deref(), Some("Grok Build"));
    assert!(!entry.description.as_ref().unwrap().is_empty());
    assert!(!entry.recommended_prompts.is_empty());
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "grok".into() });
    assert_eq!(
        serde_json::to_value(&entry.defaults).unwrap(),
        serde_json::to_value(config_agent::AgentDefinitionConfig::default().defaults)
            .unwrap()
    );
}

#[test]
fn acp_onboarding_grok_catalog_binding() {
    let catalog = load_agent_configs(&json!({"agents":{"acp-grok":{
        "provider":"","model":"","enabled":true,"label":"Selected Grok Build","runtime":{"kind":"native"},"builtin":false
    }}}));
    let entry = &catalog["acp-grok"];
    assert!(entry.enabled && entry.builtin);
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "grok".into() });
}

#[test]
fn acp_onboarding_nova_builtin_preset_metadata() {
    let catalog = load_agent_configs(&json!({}));
    let entry = catalog.get("acp-nova").expect("official Nova preset");
    assert!(entry.builtin);
    assert!(!entry.enabled);
    assert!(!entry.allow_as_subagent);
    assert_eq!(entry.label.as_deref(), Some("Nova"));
    assert!(!entry.description.as_ref().unwrap().is_empty());
    assert!(!entry.recommended_prompts.is_empty());
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "nova".into() });
    assert_eq!(
        serde_json::to_value(&entry.defaults).unwrap(),
        serde_json::to_value(config_agent::AgentDefinitionConfig::default().defaults)
            .unwrap()
    );
}

#[test]
fn acp_onboarding_nova_catalog_binding() {
    let catalog = load_agent_configs(&json!({"agents":{"acp-nova":{
        "provider":"","model":"","enabled":true,"label":"Selected Nova","runtime":{"kind":"native"},"builtin":false
    }}}));
    let entry = &catalog["acp-nova"];
    assert!(entry.enabled && entry.builtin);
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "nova".into() });
}

#[test]
fn acp_onboarding_sigit_builtin_preset_metadata() {
    let catalog = load_agent_configs(&json!({}));
    let entry = catalog.get("acp-sigit").expect("official siGit Code preset");
    assert!(entry.builtin);
    assert!(!entry.enabled);
    assert!(!entry.allow_as_subagent);
    assert_eq!(entry.label.as_deref(), Some("siGit Code"));
    assert!(!entry.description.as_ref().unwrap().is_empty());
    assert!(!entry.recommended_prompts.is_empty());
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "sigit".into() });
    assert_eq!(
        serde_json::to_value(&entry.defaults).unwrap(),
        serde_json::to_value(config_agent::AgentDefinitionConfig::default().defaults)
            .unwrap()
    );
}

#[test]
fn acp_onboarding_sigit_catalog_binding() {
    let catalog = load_agent_configs(&json!({"agents":{"acp-sigit":{
        "provider":"","model":"","enabled":true,"label":"Selected siGit Code","runtime":{"kind":"native"},"builtin":false
    }}}));
    let entry = &catalog["acp-sigit"];
    assert!(entry.enabled && entry.builtin);
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "sigit".into() });
}

#[test]
fn acp_onboarding_amp_acp_builtin_preset_metadata() {
    let catalog = load_agent_configs(&json!({}));
    let entry = catalog.get("acp-amp-acp").expect("official Amp preset");
    assert!(entry.builtin);
    assert!(!entry.enabled);
    assert!(!entry.allow_as_subagent);
    assert_eq!(entry.label.as_deref(), Some("Amp"));
    assert!(!entry.description.as_ref().unwrap().is_empty());
    assert!(!entry.recommended_prompts.is_empty());
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "amp-acp".into() });
    assert_eq!(
        serde_json::to_value(&entry.defaults).unwrap(),
        serde_json::to_value(config_agent::AgentDefinitionConfig::default().defaults)
            .unwrap()
    );
}

#[test]
fn acp_onboarding_amp_acp_catalog_binding() {
    let catalog = load_agent_configs(&json!({"agents":{"acp-amp-acp":{
        "provider":"","model":"","enabled":true,"label":"Selected Amp","runtime":{"kind":"native"},"builtin":false
    }}}));
    let entry = &catalog["acp-amp-acp"];
    assert!(entry.enabled && entry.builtin);
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "amp-acp".into() });
}

#[test]
fn acp_onboarding_devin_builtin_preset_metadata() {
    let catalog = load_agent_configs(&json!({}));
    let entry = catalog.get("acp-devin").expect("official Devin preset");
    assert!(entry.builtin);
    assert!(!entry.enabled);
    assert!(!entry.allow_as_subagent);
    assert_eq!(entry.label.as_deref(), Some("Devin"));
    assert!(!entry.description.as_ref().unwrap().is_empty());
    assert!(!entry.recommended_prompts.is_empty());
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "devin".into() });
    assert_eq!(
        serde_json::to_value(&entry.defaults).unwrap(),
        serde_json::to_value(config_agent::AgentDefinitionConfig::default().defaults)
            .unwrap()
    );
}

#[test]
fn acp_onboarding_devin_catalog_binding() {
    let catalog = load_agent_configs(&json!({"agents":{"acp-devin":{
        "provider":"","model":"","enabled":true,"label":"Selected Devin","runtime":{"kind":"native"},"builtin":false
    }}}));
    let entry = &catalog["acp-devin"];
    assert!(entry.enabled && entry.builtin);
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "devin".into() });
    let saved = serde_json::to_value(&catalog).unwrap();
    let reloaded = load_agent_configs(&json!({"agents": saved}));
    assert_eq!(
        serde_json::to_value(&reloaded["acp-devin"]).unwrap(),
        serde_json::to_value(entry).unwrap()
    );
}

#[test]
fn acp_onboarding_junie_builtin_preset_metadata() {
    let catalog = load_agent_configs(&json!({}));
    let entry = catalog.get("acp-junie").expect("official Junie preset");
    assert!(entry.builtin);
    assert!(!entry.enabled);
    assert!(!entry.allow_as_subagent);
    assert_eq!(entry.label.as_deref(), Some("Junie"));
    assert!(!entry.description.as_ref().unwrap().is_empty());
    assert!(!entry.recommended_prompts.is_empty());
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "junie".into() });
    assert_eq!(
        serde_json::to_value(&entry.defaults).unwrap(),
        serde_json::to_value(config_agent::AgentDefinitionConfig::default().defaults)
            .unwrap()
    );
}

#[test]
fn acp_onboarding_junie_catalog_binding() {
    let catalog = load_agent_configs(&json!({"agents":{"acp-junie":{
        "provider":"","model":"","enabled":true,"label":"Selected Junie","runtime":{"kind":"native"},"builtin":false
    }}}));
    let entry = &catalog["acp-junie"];
    assert!(entry.enabled && entry.builtin);
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "junie".into() });
    let saved = serde_json::to_value(&catalog).unwrap();
    let reloaded = load_agent_configs(&json!({"agents": saved}));
    assert_eq!(
        serde_json::to_value(&reloaded["acp-junie"]).unwrap(),
        serde_json::to_value(entry).unwrap()
    );
}

#[test]
fn acp_onboarding_cortex_code_builtin_preset_metadata() {
    let catalog = load_agent_configs(&json!({}));
    let entry = catalog.get("acp-cortex-code").expect("official Cortex Code preset");
    assert!(entry.builtin);
    assert!(!entry.enabled);
    assert!(!entry.allow_as_subagent);
    assert_eq!(entry.label.as_deref(), Some("Cortex Code"));
    assert!(!entry.description.as_ref().unwrap().is_empty());
    assert!(!entry.recommended_prompts.is_empty());
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "cortex-code".into() });
    assert_eq!(
        serde_json::to_value(&entry.defaults).unwrap(),
        serde_json::to_value(config_agent::AgentDefinitionConfig::default().defaults)
            .unwrap()
    );
}

#[test]
fn acp_onboarding_cortex_code_catalog_binding() {
    let catalog = load_agent_configs(&json!({"agents":{"acp-cortex-code":{
        "provider":"","model":"","enabled":true,"label":"Selected Cortex Code","runtime":{"kind":"native"},"builtin":false
    }}}));
    let entry = &catalog["acp-cortex-code"];
    assert!(entry.enabled && entry.builtin);
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "cortex-code".into() });
    let saved = serde_json::to_value(&catalog).unwrap();
    let reloaded = load_agent_configs(&json!({"agents": saved}));
    assert_eq!(
        serde_json::to_value(&reloaded["acp-cortex-code"]).unwrap(),
        serde_json::to_value(entry).unwrap()
    );
}

#[test]
fn acp_onboarding_stakpak_builtin_preset_metadata() {
    let catalog = load_agent_configs(&json!({}));
    let entry = catalog.get("acp-stakpak").expect("official Stakpak preset");
    assert!(entry.builtin);
    assert!(!entry.enabled);
    assert!(!entry.allow_as_subagent);
    assert_eq!(entry.label.as_deref(), Some("Stakpak"));
    assert!(!entry.description.as_ref().unwrap().is_empty());
    assert!(!entry.recommended_prompts.is_empty());
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "stakpak".into() });
    assert_eq!(
        serde_json::to_value(&entry.defaults).unwrap(),
        serde_json::to_value(config_agent::AgentDefinitionConfig::default().defaults)
            .unwrap()
    );
}

#[test]
fn acp_onboarding_stakpak_catalog_binding() {
    let catalog = load_agent_configs(&json!({"agents":{"acp-stakpak":{
        "provider":"","model":"","enabled":true,"label":"Selected Stakpak","runtime":{"kind":"native"},"builtin":false
    }}}));
    let entry = &catalog["acp-stakpak"];
    assert!(entry.enabled && entry.builtin);
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "stakpak".into() });
    let saved = serde_json::to_value(&catalog).unwrap();
    let reloaded = load_agent_configs(&json!({"agents": saved}));
    assert_eq!(
        serde_json::to_value(&reloaded["acp-stakpak"]).unwrap(),
        serde_json::to_value(entry).unwrap()
    );
}

#[test]
fn acp_onboarding_harn_builtin_preset_metadata() {
    let catalog = load_agent_configs(&json!({}));
    let entry = catalog.get("acp-harn").expect("official Harn preset");
    assert!(entry.builtin);
    assert!(!entry.enabled);
    assert!(!entry.allow_as_subagent);
    assert_eq!(entry.label.as_deref(), Some("Harn"));
    assert!(!entry.description.as_ref().unwrap().is_empty());
    assert!(!entry.recommended_prompts.is_empty());
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "harn".into() });
    assert_eq!(
        serde_json::to_value(&entry.defaults).unwrap(),
        serde_json::to_value(config_agent::AgentDefinitionConfig::default().defaults)
            .unwrap()
    );
}

#[test]
fn acp_onboarding_harn_catalog_binding() {
    let catalog = load_agent_configs(&json!({"agents":{"acp-harn":{
        "provider":"","model":"","enabled":true,"label":"Selected Harn","runtime":{"kind":"native"},"builtin":false
    }}}));
    let entry = &catalog["acp-harn"];
    assert!(entry.enabled && entry.builtin);
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "harn".into() });
    let saved = serde_json::to_value(&catalog).unwrap();
    let reloaded = load_agent_configs(&json!({"agents": saved}));
    assert_eq!(
        serde_json::to_value(&reloaded["acp-harn"]).unwrap(),
        serde_json::to_value(entry).unwrap()
    );
}

#[test]
fn acp_onboarding_mimo_code_builtin_preset_metadata() {
    let catalog = load_agent_configs(&json!({}));
    let entry = catalog.get("acp-mimo-code").expect("official MiMo Code preset");
    assert!(entry.builtin);
    assert!(!entry.enabled);
    assert!(!entry.allow_as_subagent);
    assert_eq!(entry.label.as_deref(), Some("MiMo Code"));
    assert!(!entry.description.as_ref().unwrap().is_empty());
    assert!(!entry.recommended_prompts.is_empty());
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "mimo-code".into() });
    assert_eq!(
        serde_json::to_value(&entry.defaults).unwrap(),
        serde_json::to_value(config_agent::AgentDefinitionConfig::default().defaults)
            .unwrap()
    );
}

#[test]
fn acp_onboarding_mimo_code_catalog_binding() {
    let catalog = load_agent_configs(&json!({"agents":{"acp-mimo-code":{
        "provider":"","model":"","enabled":true,"label":"Selected MiMo Code","runtime":{"kind":"native"},"builtin":false
    }}}));
    let entry = &catalog["acp-mimo-code"];
    assert!(entry.enabled && entry.builtin);
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "mimo-code".into() });
    let saved = serde_json::to_value(&catalog).unwrap();
    let reloaded = load_agent_configs(&json!({"agents": saved}));
    assert_eq!(
        serde_json::to_value(&reloaded["acp-mimo-code"]).unwrap(),
        serde_json::to_value(entry).unwrap()
    );
}

#[test]
fn acp_onboarding_omp_builtin_preset_metadata() {
    let catalog = load_agent_configs(&json!({}));
    let entry = catalog.get("acp-omp").expect("official omp preset");
    assert!(entry.builtin);
    assert!(!entry.enabled);
    assert!(!entry.allow_as_subagent);
    assert_eq!(entry.label.as_deref(), Some("omp"));
    assert!(!entry.description.as_ref().unwrap().is_empty());
    assert!(!entry.recommended_prompts.is_empty());
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "omp".into() });
    assert_eq!(
        serde_json::to_value(&entry.defaults).unwrap(),
        serde_json::to_value(config_agent::AgentDefinitionConfig::default().defaults)
            .unwrap()
    );
}

#[test]
fn acp_onboarding_omp_catalog_binding() {
    let catalog = load_agent_configs(&json!({"agents":{"acp-omp":{
        "provider":"","model":"","enabled":true,"label":"Selected omp","runtime":{"kind":"native"},"builtin":false
    }}}));
    let entry = &catalog["acp-omp"];
    assert!(entry.enabled && entry.builtin);
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "omp".into() });
    let saved = serde_json::to_value(&catalog).unwrap();
    let reloaded = load_agent_configs(&json!({"agents": saved}));
    assert_eq!(
        serde_json::to_value(&reloaded["acp-omp"]).unwrap(),
        serde_json::to_value(entry).unwrap()
    );
}

#[test]
fn acp_onboarding_vtcode_builtin_preset_metadata() {
    let catalog = load_agent_configs(&json!({}));
    let entry = catalog.get("acp-vtcode").expect("official VT Code preset");
    assert!(entry.builtin);
    assert!(!entry.enabled);
    assert!(!entry.allow_as_subagent);
    assert_eq!(entry.label.as_deref(), Some("VT Code"));
    assert!(!entry.description.as_ref().unwrap().is_empty());
    assert!(!entry.recommended_prompts.is_empty());
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "vtcode".into() });
    assert_eq!(
        serde_json::to_value(&entry.defaults).unwrap(),
        serde_json::to_value(config_agent::AgentDefinitionConfig::default().defaults)
            .unwrap()
    );
}

#[test]
fn acp_onboarding_vtcode_catalog_binding() {
    let catalog = load_agent_configs(&json!({"agents":{"acp-vtcode":{
        "provider":"","model":"","enabled":true,"label":"Selected VT Code","runtime":{"kind":"native"},"builtin":false
    }}}));
    let entry = &catalog["acp-vtcode"];
    assert!(entry.enabled && entry.builtin);
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "vtcode".into() });
    let saved = serde_json::to_value(&catalog).unwrap();
    let reloaded = load_agent_configs(&json!({"agents": saved}));
    assert_eq!(
        serde_json::to_value(&reloaded["acp-vtcode"]).unwrap(),
        serde_json::to_value(entry).unwrap()
    );
}

#[test]
fn acp_onboarding_aionrs_builtin_preset_metadata() {
    let catalog = load_agent_configs(&json!({}));
    let entry = catalog.get("acp-aionrs").expect("official aionrs preset");
    assert!(entry.builtin);
    assert!(!entry.enabled);
    assert!(!entry.allow_as_subagent);
    assert_eq!(entry.label.as_deref(), Some("aionrs"));
    assert!(!entry.description.as_ref().unwrap().is_empty());
    assert!(!entry.recommended_prompts.is_empty());
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "aionrs".into() });
    assert_eq!(
        serde_json::to_value(&entry.defaults).unwrap(),
        serde_json::to_value(config_agent::AgentDefinitionConfig::default().defaults)
            .unwrap()
    );
}

#[test]
fn acp_onboarding_aionrs_catalog_binding() {
    let catalog = load_agent_configs(&json!({"agents":{"acp-aionrs":{
        "provider":"","model":"","enabled":true,"label":"Selected aionrs","runtime":{"kind":"native"},"builtin":false
    }}}));
    let entry = &catalog["acp-aionrs"];
    assert!(entry.enabled && entry.builtin);
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "aionrs".into() });
    let saved = serde_json::to_value(&catalog).unwrap();
    let reloaded = load_agent_configs(&json!({"agents": saved}));
    assert_eq!(
        serde_json::to_value(&reloaded["acp-aionrs"]).unwrap(),
        serde_json::to_value(entry).unwrap()
    );
}

#[test]
fn acp_onboarding_poolside_builtin_preset_metadata() {
    let catalog = load_agent_configs(&json!({}));
    let entry = catalog.get("acp-poolside").expect("official Poolside preset");
    assert!(entry.builtin);
    assert!(!entry.enabled);
    assert!(!entry.allow_as_subagent);
    assert_eq!(entry.label.as_deref(), Some("Poolside"));
    assert!(!entry.description.as_ref().unwrap().is_empty());
    assert!(!entry.recommended_prompts.is_empty());
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "poolside".into() });
    assert_eq!(
        serde_json::to_value(&entry.defaults).unwrap(),
        serde_json::to_value(config_agent::AgentDefinitionConfig::default().defaults)
            .unwrap()
    );
}

#[test]
fn acp_onboarding_poolside_catalog_binding() {
    let catalog = load_agent_configs(&json!({"agents":{"acp-poolside":{
        "provider":"","model":"","enabled":true,"label":"Selected Poolside","runtime":{"kind":"native"},"builtin":false
    }}}));
    let entry = &catalog["acp-poolside"];
    assert!(entry.enabled && entry.builtin);
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "poolside".into() });
    let saved = serde_json::to_value(&catalog).unwrap();
    let reloaded = load_agent_configs(&json!({"agents": saved}));
    assert_eq!(
        serde_json::to_value(&reloaded["acp-poolside"]).unwrap(),
        serde_json::to_value(entry).unwrap()
    );
}

#[test]
fn acp_onboarding_corust_agent_builtin_preset_metadata() {
    let catalog = load_agent_configs(&json!({}));
    let entry = catalog.get("acp-corust-agent").expect("official Corust Agent preset");
    assert!(entry.builtin);
    assert!(!entry.enabled);
    assert!(!entry.allow_as_subagent);
    assert_eq!(entry.label.as_deref(), Some("Corust Agent"));
    assert!(!entry.description.as_ref().unwrap().is_empty());
    assert!(!entry.recommended_prompts.is_empty());
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "corust-agent".into() });
    assert_eq!(
        serde_json::to_value(&entry.defaults).unwrap(),
        serde_json::to_value(config_agent::AgentDefinitionConfig::default().defaults)
            .unwrap()
    );
}

#[test]
fn acp_onboarding_corust_agent_catalog_binding() {
    let catalog = load_agent_configs(&json!({"agents":{"acp-corust-agent":{
        "provider":"","model":"","enabled":true,"label":"Selected Corust Agent","runtime":{"kind":"native"},"builtin":false
    }}}));
    let entry = &catalog["acp-corust-agent"];
    assert!(entry.enabled && entry.builtin);
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "corust-agent".into() });
    let saved = serde_json::to_value(&catalog).unwrap();
    let reloaded = load_agent_configs(&json!({"agents": saved}));
    assert_eq!(
        serde_json::to_value(&reloaded["acp-corust-agent"]).unwrap(),
        serde_json::to_value(entry).unwrap()
    );
}
