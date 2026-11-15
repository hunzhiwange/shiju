//! 从已加载配置构造本领域设置；不执行网络或宿主操作。

pub fn build_query_classification_settings(
    full_agent_cfg: &config_aggregate::Config,
) -> crate::state::QueryClassificationSettingsState {
    let query_classification_cfg = &full_agent_cfg.query_classification;

    crate::state::QueryClassificationSettingsState {
        enabled: query_classification_cfg.enabled,
        rules: query_classification_cfg
            .rules
            .iter()
            .map(|rule| crate::state::QueryClassificationRuleInput {
                pattern: rule
                    .patterns
                    .first()
                    .cloned()
                    .or_else(|| rule.keywords.first().cloned())
                    .unwrap_or_else(|| rule.hint.clone()),
                category: rule.hint.clone(),
                priority_input: rule.priority.to_string(),
            })
            .collect(),
        save_error: None,
        save_error_translation: None,
    }
}

pub fn build_hooks_settings(
    full_agent_cfg: &config_aggregate::Config,
) -> crate::state::HooksSettingsState {
    let hooks_cfg = &full_agent_cfg.hooks;

    crate::state::HooksSettingsState {
        enabled: hooks_cfg.enabled,
        command_logger: hooks_cfg.builtin.command_logger,
        plugins_enabled: hooks_cfg.plugins.enabled,
        catalog: Vec::new(),
        catalog_loading: false,
        catalog_error: None,
        audit: Vec::new(),
        audit_loading: false,
        audit_error: None,
        mutation_loading: false,
        mutation_error: None,
        save_error: None,
    }
}

pub fn build_runtime_settings(
    full_agent_cfg: &config_aggregate::Config,
) -> crate::state::RuntimeSettingsState {
    let runtime_cfg = &full_agent_cfg.runtime;

    crate::state::RuntimeSettingsState {
        kind: match runtime_cfg.kind.trim() {
            "native" | "docker" => runtime_cfg.kind.clone(),
            _ => "native".to_string(),
        },
        docker_image: {
            let value = runtime_cfg.docker.image.trim().to_string();
            if value.is_empty() { "alpine:3.20".to_string() } else { value }
        },
        docker_network: {
            let value = runtime_cfg.docker.network.trim().to_string();
            if value.is_empty() { "none".to_string() } else { value }
        },
        docker_memory_limit_mb_input: runtime_cfg
            .docker
            .memory_limit_mb
            .map(|value| value.to_string())
            .unwrap_or_default(),
        docker_cpu_limit_input: runtime_cfg
            .docker
            .cpu_limit
            .map(|value| value.to_string())
            .unwrap_or_default(),
        docker_read_only_rootfs: runtime_cfg.docker.read_only_rootfs,
        docker_mount_workspace: runtime_cfg.docker.mount_workspace,
        docker_allowed_workspace_roots_input: runtime_cfg.docker.allowed_workspace_roots.join(", "),
        reasoning_enabled_input: match runtime_cfg.reasoning_enabled {
            None => "auto".to_string(),
            Some(true) => "true".to_string(),
            Some(false) => "false".to_string(),
        },
        reasoning_level_input: runtime_cfg.reasoning_level.clone().unwrap_or_default(),
        save_error: None,
    }
}

pub fn build_reliability_settings(
    full_agent_cfg: &config_aggregate::Config,
) -> crate::state::ReliabilitySettingsState {
    let reliability_cfg = &full_agent_cfg.reliability;

    crate::state::ReliabilitySettingsState {
        provider_retries: reliability_cfg.provider_retries,
        provider_backoff_ms: reliability_cfg.provider_backoff_ms,
        channel_initial_backoff_secs: reliability_cfg.channel_initial_backoff_secs,
        channel_max_backoff_secs: reliability_cfg.channel_max_backoff_secs,
        scheduler_poll_secs: reliability_cfg.scheduler_poll_secs,
        scheduler_retries: reliability_cfg.scheduler_retries,
        show_help_modal: false,
        save_error: None,
    }
}

pub fn build_multimodal_settings(
    full_agent_cfg: &config_aggregate::Config,
) -> crate::state::MultimodalSettingsState {
    let multimodal_cfg = &full_agent_cfg.multimodal;

    crate::state::MultimodalSettingsState {
        max_images: multimodal_cfg.max_images.clamp(1, 16) as u32,
        max_image_size_mb: multimodal_cfg.max_image_size_mb.clamp(1, 20) as u32,
        allow_remote_fetch: multimodal_cfg.allow_remote_fetch,
        ocr_enabled: multimodal_cfg.ocr.enabled,
        model_supports_images: multimodal_cfg.ocr.model_supports_images,
        ocr_model: multimodal_cfg
            .ocr
            .model
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .to_string(),
        save_error: None,
    }
}

pub fn build_observability_settings(
    full_agent_cfg: &config_aggregate::Config,
) -> crate::state::ObservabilitySettingsState {
    let observability_cfg = &full_agent_cfg.observability;

    crate::state::ObservabilitySettingsState {
        backend: match observability_cfg.backend.trim() {
            "none" | "log" | "prometheus" | "otel" => observability_cfg.backend.clone(),
            _ => "none".to_string(),
        },
        otel_endpoint_input: observability_cfg.otel_endpoint.clone().unwrap_or_default(),
        otel_service_name_input: observability_cfg.otel_service_name.clone().unwrap_or_default(),
        runtime_trace_mode: match observability_cfg.runtime_trace_mode.trim() {
            "none" | "rolling" | "full" => observability_cfg.runtime_trace_mode.clone(),
            _ => "none".to_string(),
        },
        runtime_trace_path_input: {
            let value = observability_cfg.runtime_trace_path.trim().to_string();
            if value.is_empty() { "state/runtime-trace.jsonl".to_string() } else { value }
        },
        runtime_trace_max_entries: observability_cfg.runtime_trace_max_entries.clamp(1, 100_000)
            as u32,
        show_help_modal: false,
        save_error: None,
    }
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod settings_tests;
