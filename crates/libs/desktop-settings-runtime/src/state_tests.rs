use crate::state::*;

#[test]
fn hooks_and_runtime_defaults_match_secure_native_runtime() {
    let hooks = HooksSettingsState::default();
    assert!(hooks.enabled);
    assert!(!hooks.command_logger);
    assert!(hooks.plugins_enabled);
    assert!(hooks.catalog.is_empty());
    assert!(!hooks.catalog_loading);
    assert!(hooks.catalog_error.is_none());
    assert!(hooks.audit.is_empty());
    assert!(!hooks.audit_loading);
    assert!(hooks.audit_error.is_none());
    assert!(!hooks.mutation_loading);
    assert!(hooks.mutation_error.is_none());
    assert!(hooks.save_error.is_none());

    let runtime = RuntimeSettingsState::default();
    assert_eq!(runtime.kind, "native");
    assert_eq!(runtime.docker_image, "alpine:3.20");
    assert_eq!(runtime.docker_network, "none");
    assert_eq!(runtime.docker_memory_limit_mb_input, "512");
    assert_eq!(runtime.docker_cpu_limit_input, "1");
    assert!(runtime.docker_read_only_rootfs);
    assert!(runtime.docker_mount_workspace);
    assert!(runtime.docker_allowed_workspace_roots_input.is_empty());
    assert_eq!(runtime.reasoning_enabled_input, "auto");
    assert!(runtime.reasoning_level_input.is_empty());
    assert!(runtime.save_error.is_none());
}

#[test]
fn reliability_defaults_match_runtime_config() {
    let reliability = ReliabilitySettingsState::default();
    assert_eq!(reliability.provider_retries, 2);
    assert_eq!(reliability.provider_backoff_ms, 500);
    assert_eq!(reliability.channel_initial_backoff_secs, 2);
    assert_eq!(reliability.channel_max_backoff_secs, 60);
    assert_eq!(reliability.scheduler_poll_secs, 15);
    assert_eq!(reliability.scheduler_retries, 2);
    assert!(!reliability.show_help_modal);
    assert!(reliability.save_error.is_none());
}

#[test]
fn multimodal_defaults_match_runtime_config() {
    let multimodal = MultimodalSettingsState::default();
    assert_eq!(multimodal.max_images, 10);
    assert_eq!(multimodal.max_image_size_mb, 5);
    assert!(multimodal.allow_remote_fetch);
    assert!(!multimodal.ocr_enabled);
    assert!(multimodal.model_supports_images);
    assert!(multimodal.ocr_model.is_empty());
    assert!(multimodal.save_error.is_none());
}

#[test]
fn observability_defaults_match_runtime_config() {
    let observability = ObservabilitySettingsState::default();
    assert_eq!(observability.backend, "none");
    assert!(observability.otel_endpoint_input.is_empty());
    assert!(observability.otel_service_name_input.is_empty());
    assert_eq!(observability.runtime_trace_mode, "none");
    assert_eq!(observability.runtime_trace_path_input, "state/runtime-trace.jsonl");
    assert_eq!(observability.runtime_trace_max_entries, 200);
    assert!(!observability.show_help_modal);
    assert!(observability.save_error.is_none());
}

#[test]
fn runtime_state_clones_are_independent() {
    let mut original = RuntimeSettingsState {
        kind: "docker".to_string(),
        docker_image: "custom/image:latest".to_string(),
        save_error: Some("old error".to_string()),
        ..RuntimeSettingsState::default()
    };
    let cloned = original.clone();

    original.kind = "native".to_string();
    original.docker_image = "changed".to_string();
    original.save_error = None;

    assert_eq!(cloned.kind, "docker");
    assert_eq!(cloned.docker_image, "custom/image:latest");
    assert_eq!(cloned.save_error.as_deref(), Some("old error"));
}
