use super::*;

pub fn acp_onboarding_autohand_builtin_profile_persists_and_runs<H: ProfileAcceptanceHost>() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(H::env_lock().lock());
    let _fixture =
        runtime.block_on(H::recovery(
            "https://example.test/v1",
            None,
        ));
    struct RealTransport(Option<std::ffi::OsString>);
    impl Drop for RealTransport {
        fn drop(&mut self) {
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("VW_RECOVERY_WINDOW_HTTP", value),
                    None => std::env::remove_var("VW_RECOVERY_WINDOW_HTTP"),
                }
            }
        }
    }
    let _transport = RealTransport(std::env::var_os("VW_RECOVERY_WINDOW_HTTP"));
    unsafe {
        std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
    }
    let workspace = tempfile::tempdir().unwrap();
    assert_acp_profile::<H>(&runtime, workspace.path(), "autohand");
    runtime
        .block_on(agent_config::runtime::update(json!({"agents":{"acp-autohand":{"enabled":false}}})))
        .unwrap();
    let disabled = runtime.block_on(H::chat(
        Query(InstanceQuery { directory: Some(workspace.path().to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(GatewayChatStreamRequest {
            session_id: Some(format!("ses_disabled_{}", uuid::Uuid::new_v4()).into()),
            agent: Some("acp-autohand".into()),
            messages: vec![json!({"role":"user","content":"must reject"})],
            ..Default::default()
        }),
    ));
    assert!(disabled.is_err(), "disabled preset must reject before execution");
}

pub fn acp_onboarding_deepagents_builtin_profile_persists_and_runs<H: ProfileAcceptanceHost>() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(H::env_lock().lock());
    let _fixture =
        runtime.block_on(H::recovery(
            "https://example.test/v1",
            None,
        ));
    let entry = std::env::var("SHIJU_DEEPAGENTS_TEST_ENTRY").unwrap();
    acp_agent_registry::verify_reviewed_agent_executable("deepagents", &entry).unwrap();
    let endpoint = std::env::var("SHIJU_DEEPAGENTS_TEST_ENDPOINT").unwrap();
    assert!(endpoint.starts_with("http://127.0.0.1:"));
    let previous = std::env::var_os("VW_RECOVERY_WINDOW_HTTP");
    struct Restore(Option<std::ffi::OsString>);
    impl Drop for Restore {
        fn drop(&mut self) {
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("VW_RECOVERY_WINDOW_HTTP", value),
                    None => std::env::remove_var("VW_RECOVERY_WINDOW_HTTP"),
                }
            }
        }
    }
    let _restore = Restore(previous);
    unsafe {
        std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
    }
    let workspace = tempfile::tempdir().unwrap();
    runtime.block_on(agent_config::runtime::update(json!({
        "acp":{"deepagents":{
            "command":entry,
            "args":["--model","openai:gpt-4o","--workspace",workspace.path()],
            "env":{"OPENAI_API_KEY":"local-fixture","OPENAI_BASE_URL":endpoint,"LANGSMITH_TRACING":"false","HOME":workspace.path()}
        }},
        "agents":{"acp-deepagents":{
            "enabled":true,"label":"DeepAgents acceptance",
            "defaults":{
                "permission":{"mode":"fixed","value":{"*":"deny"}},
                "skills":{"mode":"fixed","value":[]},
                "mcp":{"mode":"fixed","value":[]}
            }
        }}
    }))).unwrap();
    let directory = workspace.path().to_string_lossy().into_owned();
    runtime
        .block_on(H::with_tenant_instance(
            H::current_tenant_scope(),
            directory,
            || {
                Box::pin(async {
                    let config = H::config().await.unwrap();
                    let catalog = agent_config::catalog::merged_agent_catalog(&config.agents);
                    assert!(catalog["acp-deepagents"].builtin && catalog["acp-deepagents"].enabled);
                    assert_eq!(
                        catalog["acp-deepagents"].label.as_deref(),
                        Some("DeepAgents acceptance")
                    );
                    assert_eq!(
                        catalog["acp-deepagents"].runtime,
                        config_agent_profile::AgentRuntime::Acp { backend: "deepagents".into() }
                    );
                    Ok(())
                })
            },
        ))
        .unwrap();
    let session = format!("ses_deepagents_acceptance_{}", uuid::Uuid::new_v4());
    for _ in 0..2 {
        let events = runtime.block_on(send::<H>(workspace.path(), &session, "acp-deepagents"));
        assert!(events.contains("chat.done"), "{events}");
        assert!(!events.contains("chat.error"), "{events}");
        assert!(events.contains("验收回复"), "{events}");
    }
    runtime
        .block_on(agent_config::runtime::update(json!({"agents":{"acp-deepagents":{"enabled":false}}})))
        .unwrap();
    let disabled = runtime.block_on(H::chat(
        Query(InstanceQuery { directory: Some(workspace.path().to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(GatewayChatStreamRequest {
            agent: Some("acp-deepagents".into()),
            messages: vec![json!({"role":"user","content":"must reject"})],
            ..Default::default()
        }),
    ));
    assert!(disabled.is_err());
}
pub fn acp_onboarding_dimcode_builtin_profile_persists_and_runs<H: ProfileAcceptanceHost>() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(H::env_lock().lock());
    let _fixture =
        runtime.block_on(H::recovery(
            "https://example.test/v1",
            None,
        ));
    struct RealTransport(Option<std::ffi::OsString>);
    impl Drop for RealTransport {
        fn drop(&mut self) {
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("VW_RECOVERY_WINDOW_HTTP", value),
                    None => std::env::remove_var("VW_RECOVERY_WINDOW_HTTP"),
                }
            }
        }
    }
    let _transport = RealTransport(std::env::var_os("VW_RECOVERY_WINDOW_HTTP"));
    unsafe {
        std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
    }
    let workspace = tempfile::tempdir().unwrap();
    assert_acp_profile::<H>(&runtime, workspace.path(), "dimcode");
    runtime
        .block_on(agent_config::runtime::update(json!({"agents":{"acp-dimcode":{"enabled":false}}})))
        .unwrap();
    let disabled = runtime.block_on(H::chat(
        Query(InstanceQuery { directory: Some(workspace.path().to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(GatewayChatStreamRequest {
            session_id: Some(format!("ses_disabled_{}", uuid::Uuid::new_v4()).into()),
            agent: Some("acp-dimcode".into()),
            messages: vec![json!({"role":"user","content":"must reject"})],
            ..Default::default()
        }),
    ));
    assert!(disabled.is_err(), "disabled preset must reject before execution");
}

pub fn acp_onboarding_dirac_builtin_profile_persists_and_runs<H: ProfileAcceptanceHost>() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(H::env_lock().lock());
    let _fixture =
        runtime.block_on(H::recovery(
            "https://example.test/v1",
            None,
        ));
    struct RealTransport(Option<std::ffi::OsString>);
    impl Drop for RealTransport {
        fn drop(&mut self) {
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("VW_RECOVERY_WINDOW_HTTP", value),
                    None => std::env::remove_var("VW_RECOVERY_WINDOW_HTTP"),
                }
            }
        }
    }
    let _transport = RealTransport(std::env::var_os("VW_RECOVERY_WINDOW_HTTP"));
    unsafe {
        std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
    }
    let workspace = tempfile::tempdir().unwrap();
    assert_acp_profile::<H>(&runtime, workspace.path(), "dirac");
    runtime
        .block_on(agent_config::runtime::update(json!({"agents":{"acp-dirac":{"enabled":false}}})))
        .unwrap();
    let disabled = runtime.block_on(H::chat(
        Query(InstanceQuery { directory: Some(workspace.path().to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(GatewayChatStreamRequest {
            session_id: Some(format!("ses_disabled_{}", uuid::Uuid::new_v4()).into()),
            agent: Some("acp-dirac".into()),
            messages: vec![json!({"role":"user","content":"must reject"})],
            ..Default::default()
        }),
    ));
    assert!(disabled.is_err(), "disabled preset must reject before execution");
}

pub fn acp_onboarding_glm_acp_agent_builtin_profile_persists_and_runs<H: ProfileAcceptanceHost>() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(H::env_lock().lock());
    let _fixture =
        runtime.block_on(H::recovery(
            "https://example.test/v1",
            None,
        ));
    struct RealTransport(Option<std::ffi::OsString>);
    impl Drop for RealTransport {
        fn drop(&mut self) {
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("VW_RECOVERY_WINDOW_HTTP", value),
                    None => std::env::remove_var("VW_RECOVERY_WINDOW_HTTP"),
                }
            }
        }
    }
    let _transport = RealTransport(std::env::var_os("VW_RECOVERY_WINDOW_HTTP"));
    unsafe {
        std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
    }
    let workspace = tempfile::tempdir().unwrap();
    assert_acp_profile::<H>(&runtime, workspace.path(), "glm-acp-agent");
    runtime
        .block_on(agent_config::runtime::update(json!({"agents":{"acp-glm-acp-agent":{"enabled":false}}})))
        .unwrap();
    let disabled = runtime.block_on(H::chat(
        Query(InstanceQuery { directory: Some(workspace.path().to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(GatewayChatStreamRequest {
            session_id: Some(format!("ses_disabled_{}", uuid::Uuid::new_v4()).into()),
            agent: Some("acp-glm-acp-agent".into()),
            messages: vec![json!({"role":"user","content":"must reject"})],
            ..Default::default()
        }),
    ));
    assert!(disabled.is_err(), "disabled preset must reject before execution");
}

pub fn acp_onboarding_grok_builtin_profile_persists_and_runs<H: ProfileAcceptanceHost>() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(H::env_lock().lock());
    let _fixture =
        runtime.block_on(H::recovery(
            "https://example.test/v1",
            None,
        ));
    struct RealTransport(Option<std::ffi::OsString>);
    impl Drop for RealTransport {
        fn drop(&mut self) {
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("VW_RECOVERY_WINDOW_HTTP", value),
                    None => std::env::remove_var("VW_RECOVERY_WINDOW_HTTP"),
                }
            }
        }
    }
    let _transport = RealTransport(std::env::var_os("VW_RECOVERY_WINDOW_HTTP"));
    unsafe {
        std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
    }
    let workspace = tempfile::tempdir().unwrap();
    assert_acp_profile::<H>(&runtime, workspace.path(), "grok");
    runtime
        .block_on(agent_config::runtime::update(json!({"agents":{"acp-grok":{"enabled":false}}})))
        .unwrap();
    let disabled = runtime.block_on(H::chat(
        Query(InstanceQuery { directory: Some(workspace.path().to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(GatewayChatStreamRequest {
            session_id: Some(format!("ses_disabled_{}", uuid::Uuid::new_v4()).into()),
            agent: Some("acp-grok".into()),
            messages: vec![json!({"role":"user","content":"must reject"})],
            ..Default::default()
        }),
    ));
    assert!(disabled.is_err(), "disabled preset must reject before execution");
}

pub fn acp_onboarding_nova_builtin_profile_persists_and_runs<H: ProfileAcceptanceHost>() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(H::env_lock().lock());
    let _fixture =
        runtime.block_on(H::recovery(
            "https://example.test/v1",
            None,
        ));
    struct RealTransport(Option<std::ffi::OsString>);
    impl Drop for RealTransport {
        fn drop(&mut self) {
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("VW_RECOVERY_WINDOW_HTTP", value),
                    None => std::env::remove_var("VW_RECOVERY_WINDOW_HTTP"),
                }
            }
        }
    }
    let _transport = RealTransport(std::env::var_os("VW_RECOVERY_WINDOW_HTTP"));
    unsafe {
        std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
    }
    let workspace = tempfile::tempdir().unwrap();
    assert_acp_profile::<H>(&runtime, workspace.path(), "nova");
    runtime
        .block_on(agent_config::runtime::update(json!({"agents":{"acp-nova":{"enabled":false}}})))
        .unwrap();
    let disabled = runtime.block_on(H::chat(
        Query(InstanceQuery { directory: Some(workspace.path().to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(GatewayChatStreamRequest {
            session_id: Some(format!("ses_disabled_{}", uuid::Uuid::new_v4()).into()),
            agent: Some("acp-nova".into()),
            messages: vec![json!({"role":"user","content":"must reject"})],
            ..Default::default()
        }),
    ));
    assert!(disabled.is_err(), "disabled preset must reject before execution");
}

pub fn acp_onboarding_nova_real_builtin_profile_persists_and_runs<H: ProfileAcceptanceHost>() {
    assert!(std::env::var_os("SHIJU_NOVA_REAL_PROFILE").is_some());
    acp_onboarding_nova_builtin_profile_persists_and_runs::<H>();
}

pub fn acp_onboarding_sigit_builtin_profile_persists_and_runs<H: ProfileAcceptanceHost>() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(H::env_lock().lock());
    let _fixture =
        runtime.block_on(H::recovery(
            "https://example.test/v1",
            None,
        ));
    struct RealTransport(Option<std::ffi::OsString>);
    impl Drop for RealTransport {
        fn drop(&mut self) {
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("VW_RECOVERY_WINDOW_HTTP", value),
                    None => std::env::remove_var("VW_RECOVERY_WINDOW_HTTP"),
                }
            }
        }
    }
    let _transport = RealTransport(std::env::var_os("VW_RECOVERY_WINDOW_HTTP"));
    unsafe {
        std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
    }
    let workspace = tempfile::tempdir().unwrap();
    assert_acp_profile::<H>(&runtime, workspace.path(), "sigit");
    runtime
        .block_on(agent_config::runtime::update(json!({"agents":{"acp-sigit":{"enabled":false}}})))
        .unwrap();
    let disabled = runtime.block_on(H::chat(
        Query(InstanceQuery { directory: Some(workspace.path().to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(GatewayChatStreamRequest {
            session_id: Some(format!("ses_disabled_{}", uuid::Uuid::new_v4()).into()),
            agent: Some("acp-sigit".into()),
            messages: vec![json!({"role":"user","content":"must reject"})],
            ..Default::default()
        }),
    ));
    assert!(disabled.is_err(), "disabled preset must reject before execution");
}

pub fn acp_onboarding_sigit_real_builtin_profile_persists_and_runs<H: ProfileAcceptanceHost>() {
    assert!(std::env::var_os("SHIJU_SIGIT_REAL_ENTRY").is_some());
    acp_onboarding_sigit_builtin_profile_persists_and_runs::<H>();
}

pub fn acp_onboarding_amp_acp_builtin_profile_persists_and_runs<H: ProfileAcceptanceHost>() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(H::env_lock().lock());
    let _fixture =
        runtime.block_on(H::recovery(
            &format!(
                "{}v1/chat/completions",
                std::env::var("SHIJU_AMP_REAL_ENDPOINT").expect("owned Amp model fixture")
            ),
            Some("fixture-model-token"),
        ));
    struct RealTransport(Option<std::ffi::OsString>);
    impl Drop for RealTransport {
        fn drop(&mut self) {
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("VW_RECOVERY_WINDOW_HTTP", value),
                    None => std::env::remove_var("VW_RECOVERY_WINDOW_HTTP"),
                }
            }
        }
    }
    let _transport = RealTransport(std::env::var_os("VW_RECOVERY_WINDOW_HTTP"));
    unsafe {
        std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
    }
    runtime
        .block_on(agent_config::runtime::update(
            json!({"providers":{"model-recovery":{"adapter":"openai-compatible","models":{"original":{"tool_call":true}}}}}),
        ))
        .unwrap();
    runtime.block_on(H::invalidate_cache());
    let entry = std::path::PathBuf::from(
        std::env::var("SHIJU_AMP_REAL_ENTRY").expect("reviewed Amp runtime"),
    );
    let installation =
        entry.parent().and_then(std::path::Path::parent).and_then(std::path::Path::parent).unwrap();
    let directory = std::path::PathBuf::from(std::env::var_os("VIBEWINDOW_CONFIG_DIR").unwrap())
        .join("acp-runtime");
    std::fs::create_dir_all(&directory).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(installation, directory.join("amp-acp-0.12.0-model-source-v1"))
        .unwrap();
    let workspace = tempfile::tempdir().unwrap();
    assert_acp_profile::<H>(&runtime, workspace.path(), "amp-acp");
    runtime
        .block_on(agent_config::runtime::update(json!({"agents":{"acp-amp-acp":{"enabled":false}}})))
        .unwrap();
    let disabled = runtime.block_on(H::chat(
        Query(InstanceQuery { directory: Some(workspace.path().to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(GatewayChatStreamRequest {
            session_id: Some(format!("ses_disabled_{}", uuid::Uuid::new_v4()).into()),
            agent: Some("acp-amp-acp".into()),
            messages: vec![json!({"role":"user","content":"must reject"})],
            ..Default::default()
        }),
    ));
    assert!(disabled.is_err(), "disabled preset must reject before execution");
}

pub fn acp_onboarding_devin_builtin_profile_persists_and_runs<H: ProfileAcceptanceHost>() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(H::env_lock().lock());
    let _fixture =
        runtime.block_on(H::recovery(
            "https://example.test/v1",
            None,
        ));
    struct RealTransport(Option<std::ffi::OsString>);
    impl Drop for RealTransport {
        fn drop(&mut self) {
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("VW_RECOVERY_WINDOW_HTTP", value),
                    None => std::env::remove_var("VW_RECOVERY_WINDOW_HTTP"),
                }
            }
        }
    }
    let _transport = RealTransport(std::env::var_os("VW_RECOVERY_WINDOW_HTTP"));
    unsafe {
        std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
    }
    let workspace = tempfile::tempdir().unwrap();
    assert_acp_profile::<H>(&runtime, workspace.path(), "devin");
    runtime
        .block_on(agent_config::runtime::update(json!({"agents":{"acp-devin":{"enabled":false}}})))
        .unwrap();
    let disabled = runtime.block_on(H::chat(
        Query(InstanceQuery { directory: Some(workspace.path().to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(GatewayChatStreamRequest {
            session_id: Some(format!("ses_disabled_{}", uuid::Uuid::new_v4()).into()),
            agent: Some("acp-devin".into()),
            messages: vec![json!({"role":"user","content":"must reject"})],
            ..Default::default()
        }),
    ));
    assert!(disabled.is_err(), "disabled preset must reject before execution");
}

pub fn acp_onboarding_junie_builtin_profile_persists_and_runs<H: ProfileAcceptanceHost>() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(H::env_lock().lock());
    let _fixture =
        runtime.block_on(H::recovery(
            "https://example.test/v1",
            None,
        ));
    struct RealTransport(Option<std::ffi::OsString>);
    impl Drop for RealTransport {
        fn drop(&mut self) {
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("VW_RECOVERY_WINDOW_HTTP", value),
                    None => std::env::remove_var("VW_RECOVERY_WINDOW_HTTP"),
                }
            }
        }
    }
    let _transport = RealTransport(std::env::var_os("VW_RECOVERY_WINDOW_HTTP"));
    unsafe {
        std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
    }
    let workspace = tempfile::tempdir().unwrap();
    assert_acp_profile::<H>(&runtime, workspace.path(), "junie");
    runtime
        .block_on(agent_config::runtime::update(json!({"agents":{"acp-junie":{"enabled":false}}})))
        .unwrap();
    let disabled = runtime.block_on(H::chat(
        Query(InstanceQuery { directory: Some(workspace.path().to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(GatewayChatStreamRequest {
            session_id: Some(format!("ses_disabled_{}", uuid::Uuid::new_v4()).into()),
            agent: Some("acp-junie".into()),
            messages: vec![json!({"role":"user","content":"must reject"})],
            ..Default::default()
        }),
    ));
    assert!(disabled.is_err(), "disabled preset must reject before execution");
}

pub fn acp_onboarding_cortex_code_builtin_profile_persists_and_runs<H: ProfileAcceptanceHost>() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(H::env_lock().lock());
    let _fixture =
        runtime.block_on(H::recovery(
            "https://example.test/v1",
            None,
        ));
    struct RealTransport(Option<std::ffi::OsString>);
    impl Drop for RealTransport {
        fn drop(&mut self) {
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("VW_RECOVERY_WINDOW_HTTP", value),
                    None => std::env::remove_var("VW_RECOVERY_WINDOW_HTTP"),
                }
            }
        }
    }
    let _transport = RealTransport(std::env::var_os("VW_RECOVERY_WINDOW_HTTP"));
    unsafe {
        std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
    }
    let workspace = tempfile::tempdir().unwrap();
    assert_acp_profile::<H>(&runtime, workspace.path(), "cortex-code");
    runtime
        .block_on(agent_config::runtime::update(json!({"agents":{"acp-cortex-code":{"enabled":false}}})))
        .unwrap();
    let disabled = runtime.block_on(H::chat(
        Query(InstanceQuery { directory: Some(workspace.path().to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(GatewayChatStreamRequest {
            session_id: Some(format!("ses_disabled_{}", uuid::Uuid::new_v4()).into()),
            agent: Some("acp-cortex-code".into()),
            messages: vec![json!({"role":"user","content":"must reject"})],
            ..Default::default()
        }),
    ));
    assert!(disabled.is_err(), "disabled preset must reject before execution");
}

pub fn acp_onboarding_stakpak_builtin_profile_persists_and_runs<H: ProfileAcceptanceHost>() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(H::env_lock().lock());
    let _fixture =
        runtime.block_on(H::recovery(
            "https://example.test/v1",
            None,
        ));
    struct RealTransport(Option<std::ffi::OsString>);
    impl Drop for RealTransport {
        fn drop(&mut self) {
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("VW_RECOVERY_WINDOW_HTTP", value),
                    None => std::env::remove_var("VW_RECOVERY_WINDOW_HTTP"),
                }
            }
        }
    }
    let _transport = RealTransport(std::env::var_os("VW_RECOVERY_WINDOW_HTTP"));
    unsafe {
        std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
    }
    let workspace = tempfile::tempdir().unwrap();
    assert_acp_profile::<H>(&runtime, workspace.path(), "stakpak");
    runtime
        .block_on(agent_config::runtime::update(json!({"agents":{"acp-stakpak":{"enabled":false}}})))
        .unwrap();
    let disabled = runtime.block_on(H::chat(
        Query(InstanceQuery { directory: Some(workspace.path().to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(GatewayChatStreamRequest {
            session_id: Some(format!("ses_disabled_{}", uuid::Uuid::new_v4()).into()),
            agent: Some("acp-stakpak".into()),
            messages: vec![json!({"role":"user","content":"must reject"})],
            ..Default::default()
        }),
    ));
    assert!(disabled.is_err(), "disabled preset must reject before execution");
}

pub fn acp_onboarding_harn_builtin_profile_persists_and_runs<H: ProfileAcceptanceHost>() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(H::env_lock().lock());
    let _fixture =
        runtime.block_on(H::recovery(
            "https://example.test/v1",
            None,
        ));
    struct RealTransport(Option<std::ffi::OsString>);
    impl Drop for RealTransport {
        fn drop(&mut self) {
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("VW_RECOVERY_WINDOW_HTTP", value),
                    None => std::env::remove_var("VW_RECOVERY_WINDOW_HTTP"),
                }
            }
        }
    }
    let _transport = RealTransport(std::env::var_os("VW_RECOVERY_WINDOW_HTTP"));
    unsafe {
        std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
    }
    let workspace = tempfile::tempdir().unwrap();
    assert_acp_profile::<H>(&runtime, workspace.path(), "harn");
    runtime
        .block_on(agent_config::runtime::update(json!({"agents":{"acp-harn":{"enabled":false}}})))
        .unwrap();
    let disabled = runtime.block_on(H::chat(
        Query(InstanceQuery { directory: Some(workspace.path().to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(GatewayChatStreamRequest {
            session_id: Some(format!("ses_disabled_{}", uuid::Uuid::new_v4()).into()),
            agent: Some("acp-harn".into()),
            messages: vec![json!({"role":"user","content":"must reject"})],
            ..Default::default()
        }),
    ));
    assert!(disabled.is_err(), "disabled preset must reject before execution");
}

pub fn acp_onboarding_mimo_code_builtin_profile_persists_and_runs<H: ProfileAcceptanceHost>() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(H::env_lock().lock());
    let _fixture =
        runtime.block_on(H::recovery(
            "https://example.test/v1",
            None,
        ));
    struct RealTransport(Option<std::ffi::OsString>);
    impl Drop for RealTransport {
        fn drop(&mut self) {
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("VW_RECOVERY_WINDOW_HTTP", value),
                    None => std::env::remove_var("VW_RECOVERY_WINDOW_HTTP"),
                }
            }
        }
    }
    let _transport = RealTransport(std::env::var_os("VW_RECOVERY_WINDOW_HTTP"));
    unsafe {
        std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
    }
    let workspace = tempfile::tempdir().unwrap();
    assert_acp_profile::<H>(&runtime, workspace.path(), "mimo-code");
    runtime
        .block_on(agent_config::runtime::update(json!({"agents":{"acp-mimo-code":{"enabled":false}}})))
        .unwrap();
    let disabled = runtime.block_on(H::chat(
        Query(InstanceQuery { directory: Some(workspace.path().to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(GatewayChatStreamRequest {
            session_id: Some(format!("ses_disabled_{}", uuid::Uuid::new_v4()).into()),
            agent: Some("acp-mimo-code".into()),
            messages: vec![json!({"role":"user","content":"must reject"})],
            ..Default::default()
        }),
    ));
    assert!(disabled.is_err(), "disabled preset must reject before execution");
}

pub fn acp_onboarding_omp_builtin_profile_persists_and_runs<H: ProfileAcceptanceHost>() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(H::env_lock().lock());
    let _fixture =
        runtime.block_on(H::recovery(
            "https://example.test/v1",
            None,
        ));
    struct RealTransport(Option<std::ffi::OsString>);
    impl Drop for RealTransport {
        fn drop(&mut self) {
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("VW_RECOVERY_WINDOW_HTTP", value),
                    None => std::env::remove_var("VW_RECOVERY_WINDOW_HTTP"),
                }
            }
        }
    }
    let _transport = RealTransport(std::env::var_os("VW_RECOVERY_WINDOW_HTTP"));
    unsafe {
        std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
    }
    let workspace = tempfile::tempdir().unwrap();
    assert_acp_profile::<H>(&runtime, workspace.path(), "omp");
    runtime
        .block_on(agent_config::runtime::update(json!({"agents":{"acp-omp":{"enabled":false}}})))
        .unwrap();
    let disabled = runtime.block_on(H::chat(
        Query(InstanceQuery { directory: Some(workspace.path().to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(GatewayChatStreamRequest {
            session_id: Some(format!("ses_disabled_{}", uuid::Uuid::new_v4()).into()),
            agent: Some("acp-omp".into()),
            messages: vec![json!({"role":"user","content":"must reject"})],
            ..Default::default()
        }),
    ));
    assert!(disabled.is_err(), "disabled preset must reject before execution");
}
