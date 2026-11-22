use crate::acp::{config, controls, model_source::drivers, request, skill_mounts};
use serde_json::json;

#[path = "../../aisdk/convert_test_support.rs"]
mod models;
use vw_api_acp_model::acp_model_source::{
    AcpModelSource as Source, AcpModelSourceCapability as Capability,
};

#[test]
fn acp_platform_dispatch_contract_preserves_native_options_and_rejects_mounts() {
    for agent in ["corust-agent", "omp", "stakpak", "vtcode"] {
        let spec = acp_agent_registry::resolve_agent_spec(agent).unwrap();
        assert!(!spec.command.is_empty(), "{agent}");
        let mut options = Default::default();
        let host = controls::Host {
            tenant: || unreachable!(),
            open: |_, _| unreachable!(),
            session_name: |_| unreachable!(),
            verified_release: drivers::verified_release,
            capabilities: drivers::capabilities,
        };
        controls::annotate_model_source(&host, &mut options, agent);
        assert!(
            options
                .model_source_capabilities
                .contains(&Capability::Supported { source: Source::CliLocal }),
            "{agent}"
        );
        assert!(
            options.model_source_capabilities.iter().any(|capability| matches!(
                capability,
                Capability::Unsupported { source: Source::SessionModel, .. }
            )),
            "{agent}"
        );
        assert!(skill_mounts::validate(agent, true).is_err(), "{agent}");
        assert!(acp_client::AcpClient::ensure_desktop_native_access(agent, false).is_err());
        assert!(acp_client::AcpClient::ensure_desktop_native_access(agent, true).is_ok());
        assert!(request::validate_linux_cli_mcp(agent, false).is_ok(), "{agent}");
        assert!(request::validate_linux_cli_mcp(agent, true).is_err(), "{agent}");
    }
}

#[test]
fn acp_platform_dispatch_contract_resolves_native_config_and_session_options() {
    let model = models::test_model(
        "fixture",
        "openai-compatible",
        "http://127.0.0.1:34567/v1",
        "selected",
        "Selected",
    );
    let host_config = agent_config::schema::Config::default();
    let registry = acp_agent_registry::built_in_agent_specs();
    assert_eq!(registry.len(), 41);

    for (agent, spec) in registry {
        let options = json!({
            "acp_agent": agent,
            "acp_session_model": "native-option",
            "acp_session_mode": "default",
            "acp_session_config": {"fixture_option": "chosen"},
        });
        let (resolved, cfg) = config::lookup_acp_command(&host_config, &model, &options).unwrap();
        assert_eq!(resolved, agent);
        assert_eq!(cfg.command, spec.command, "{agent}");
        let parsed = config::parse_acp_options::<()>(&options, &agent, &cfg).unwrap();
        assert_eq!(parsed.session_mode.as_deref(), Some("default"), "{agent}");
        assert_eq!(
            parsed.session_options.unwrap().model.as_deref(),
            Some("native-option"),
            "{agent}",
        );
        assert_eq!(
            parsed.session_config_options,
            [("fixture_option".to_owned(), "chosen".to_owned())]
        );
    }

    assert!(
        config::lookup_acp_command(
            &host_config,
            &model,
            &json!({"acp_agent": "unknown-acp-platform-test"}),
        )
        .is_none()
    );
}
