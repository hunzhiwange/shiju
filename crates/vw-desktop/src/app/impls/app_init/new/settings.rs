//! 组织桌面应用初始化阶段的 settings.rs 逻辑。
//! 本模块把启动输入、配置加载和初始状态装配拆开，便于定位启动失败路径。

use super::*;

/// 模块内可见函数，执行 build_embedding_routes_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_embedding_routes_settings(
    full_agent_cfg: &vw_config_types::config::Config,
    gateway_cfg_result: &Result<vw_config_types::gateway::GatewayConfig, String>,
) -> crate::app::state::EmbeddingRoutesSettingsState {
    crate::app::state::EmbeddingRoutesSettingsState {
        routes: full_agent_cfg
            .embedding_routes
            .iter()
            .map(|route| crate::app::state::EmbeddingRouteDraft {
                pattern: route.hint.clone(),
                provider: route.provider.clone(),
                model: route.model.clone(),
                dimensions: route.dimensions.map(|value| value.to_string()).unwrap_or_default(),
                api_key_input: route.api_key.clone().unwrap_or_default(),
            })
            .collect(),
        save_error: gateway_cfg_result
            .as_ref()
            .err()
            .map(|err| config::server_config_unreachable_error(err.clone())),
        save_success: false,
    }
}

/// 模块内可见函数，执行 build_model_routes_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_model_routes_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> crate::app::state::ModelRoutesSettingsState {
    let query_classification_cfg = &full_agent_cfg.query_classification;
    let model_routes_cfg = &full_agent_cfg.model_routes;

    crate::app::state::ModelRoutesSettingsState {
        routes: model_routes_cfg
            .iter()
            .map(|route| crate::app::state::ModelRoute {
                pattern: route.hint.clone(),
                provider: route.provider.clone(),
                model: route.model.clone(),
                priority_input: query_classification_cfg
                    .rules
                    .iter()
                    .find(|rule| rule.hint == route.hint)
                    .map(|rule| rule.priority.to_string())
                    .unwrap_or_else(|| "0".to_string()),
            })
            .collect(),
        save_error: if model_routes_cfg.is_empty() && !query_classification_cfg.rules.is_empty() {
            Some("已检测到 query_classification 规则，但桌面模型路由列表为空".to_string())
        } else {
            None
        },
    }
}

pub(super) fn build_query_classification_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> crate::app::state::QueryClassificationSettingsState {
    let query_classification_cfg = &full_agent_cfg.query_classification;

    crate::app::state::QueryClassificationSettingsState {
        enabled: query_classification_cfg.enabled,
        rules: query_classification_cfg
            .rules
            .iter()
            .map(|rule| crate::app::state::QueryClassificationRuleInput {
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
    }
}

/// 模块内可见函数，执行 build_heartbeat_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_heartbeat_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> crate::app::state::HeartbeatSettingsState {
    let heartbeat_cfg = &full_agent_cfg.heartbeat;

    crate::app::state::HeartbeatSettingsState {
        enabled: heartbeat_cfg.enabled,
        interval_minutes: heartbeat_cfg.interval_minutes.clamp(1, 1440),
        message_input: heartbeat_cfg.message.clone().unwrap_or_default(),
        target_input: heartbeat_cfg.target.clone().unwrap_or_default(),
        to_input: heartbeat_cfg.to.clone().unwrap_or_default(),
        show_help_modal: false,
        save_error: None,
    }
}

/// 模块内可见函数，执行 build_cron_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_cron_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> crate::app::state::CronSettingsState {
    let cron_cfg = &full_agent_cfg.cron;

    crate::app::state::CronSettingsState {
        enabled: cron_cfg.enabled,
        max_run_history: cron_cfg.max_run_history.clamp(1, 10_000),
        active_tab: crate::app::state::CronSettingsTab::default(),
        jobs_loading: false,
        jobs: Vec::new(),
        selected_job_ids: Vec::new(),
        expanded_job_ids: Vec::new(),
        editing_job_id: None,
        edit_draft: crate::app::state::CronJobDraft::default(),
        add_draft: crate::app::state::CronJobDraft::default(),
        runs_modal_job_id: None,
        runs_modal_loading: false,
        runs_modal_error: None,
        runs_modal: Vec::new(),
        runs_modal_editor: iced::widget::text_editor::Content::new(),
        show_help_modal: false,
        save_error: None,
        action_status: None,
    }
}

/// 模块内可见函数，执行 build_scheduler_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_scheduler_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> crate::app::state::SchedulerSettingsState {
    let scheduler_cfg = &full_agent_cfg.scheduler;

    crate::app::state::SchedulerSettingsState {
        enabled: scheduler_cfg.enabled,
        max_tasks: scheduler_cfg.max_tasks.clamp(1, 10_000) as u32,
        max_concurrent: scheduler_cfg.max_concurrent.clamp(1, 100) as u32,
        show_help_modal: false,
        save_error: None,
    }
}

/// 模块内可见函数，执行 build_hooks_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_hooks_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> crate::app::state::HooksSettingsState {
    let hooks_cfg = &full_agent_cfg.hooks;

    crate::app::state::HooksSettingsState {
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

/// 模块内可见函数，执行 build_runtime_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_runtime_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> crate::app::state::RuntimeSettingsState {
    let runtime_cfg = &full_agent_cfg.runtime;

    crate::app::state::RuntimeSettingsState {
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

/// 模块内可见函数，执行 build_skills_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_skills_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> crate::app::state::SkillsSettingsState {
    let skills_cfg = &full_agent_cfg.skills;

    crate::app::state::SkillsSettingsState {
        auto_load_project_skills: skills_cfg.auto_load_project_skills,
        directory_provider: skills_cfg.directory_provider,
        prompt_injection_mode: skills_cfg.prompt_injection_mode,
        active_tab: crate::app::state::SkillsSettingsTab::Skills,
        query: String::new(),
        directory_scope: crate::app::state::SkillsDirectoryScope::Project,
        loading: false,
        catalog: Vec::new(),
        plugin_catalog: Vec::new(),
        plugin_loading: false,
        selected_plugin_id: None,
        plugin_status_message: None,
        plugin_status_is_error: false,
        selected_skill_id: None,
        selected_skill_detail: None,
        detail_loading: false,
        detail_error: None,
        status_message: None,
        status_is_error: false,
        show_help_modal: false,
        save_error: None,
    }
}

/// 模块内可见函数，执行 build_web_search_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_web_search_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> crate::app::state::WebSearchSettingsState {
    let web_search_cfg = &full_agent_cfg.web_search;

    crate::app::state::WebSearchSettingsState {
        enabled: web_search_cfg.enabled,
        provider: match web_search_cfg.provider.trim().to_ascii_lowercase().as_str() {
            "ddg" | "duckduckgo" => "duckduckgo".to_string(),
            "brave" => "brave".to_string(),
            "serper" => "serper".to_string(),
            "google" => "google".to_string(),
            "bing" => "bing".to_string(),
            _ => "duckduckgo".to_string(),
        },
        api_key_input: web_search_cfg.api_key.clone().unwrap_or_default(),
        api_url_input: web_search_cfg.api_url.clone().unwrap_or_default(),
        brave_api_key_input: web_search_cfg.brave_api_key.clone().unwrap_or_default(),
        max_results_input: web_search_cfg.max_results.clamp(1, 10).to_string(),
        timeout_secs_input: web_search_cfg.timeout_secs.max(1).to_string(),
        user_agent: if web_search_cfg.user_agent.trim().is_empty() {
            vw_config_types::tools::DEFAULT_USER_AGENT.to_string()
        } else {
            web_search_cfg.user_agent.clone()
        },
        show_help_modal: false,
        save_error: None,
    }
}

/// 模块内可见函数，执行 build_browser_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_browser_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> crate::app::state::BrowserSettingsState {
    let browser_cfg = &full_agent_cfg.browser;
    let allowed_domains = browser_cfg.allowed_domains.join("\n");

    crate::app::state::BrowserSettingsState {
        enabled: browser_cfg.enabled,
        allowed_domains_input: allowed_domains.clone(),
        allowed_domains_editor: iced::widget::text_editor::Content::with_text(&allowed_domains),
        browser_open: match browser_cfg.browser_open.trim().to_ascii_lowercase().as_str() {
            "default" | "new_window" | "new_tab" => {
                browser_cfg.browser_open.trim().to_ascii_lowercase()
            }
            _ => "default".to_string(),
        },
        session_name_input: browser_cfg.session_name.clone().unwrap_or_default(),
        backend: match browser_cfg.backend.trim().to_ascii_lowercase().replace('-', "_").as_str() {
            "agent_browser" => "agent_browser".to_string(),
            "rust_native" | "native" => "native".to_string(),
            "computer_use" => "computer_use".to_string(),
            "auto" => "auto".to_string(),
            _ => "agent_browser".to_string(),
        },
        native_headless: browser_cfg.native_headless,
        native_webdriver_url: browser_cfg.native_webdriver_url.clone(),
        native_chrome_path_input: browser_cfg.native_chrome_path.clone().unwrap_or_default(),
        computer_use_endpoint: browser_cfg.computer_use.endpoint.clone(),
        computer_use_api_key_input: browser_cfg.computer_use.api_key.clone().unwrap_or_default(),
        computer_use_timeout_ms_input: browser_cfg.computer_use.timeout_ms.to_string(),
        computer_use_allow_remote_endpoint: browser_cfg.computer_use.allow_remote_endpoint,
        computer_use_window_allowlist_input: browser_cfg.computer_use.window_allowlist.join(", "),
        computer_use_max_coordinate_x_input: browser_cfg
            .computer_use
            .max_coordinate_x
            .map(|value| value.to_string())
            .unwrap_or_default(),
        computer_use_max_coordinate_y_input: browser_cfg
            .computer_use
            .max_coordinate_y
            .map(|value| value.to_string())
            .unwrap_or_default(),
        save_error: None,
    }
}

/// 模块内可见函数，执行 build_http_request_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_http_request_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> crate::app::state::HttpRequestSettingsState {
    let http_request_cfg = &full_agent_cfg.http_request;

    crate::app::state::HttpRequestSettingsState {
        enabled: http_request_cfg.enabled,
        allowed_domains: http_request_cfg.allowed_domains.clone(),
        new_allowed_domain_input: String::new(),
        max_response_size: http_request_cfg.max_response_size.min(u32::MAX as usize) as u32,
        timeout_secs: http_request_cfg.timeout_secs.min(u32::MAX as u64) as u32,
        user_agent: http_request_cfg.user_agent.clone(),
        save_error: None,
    }
}

/// 模块内可见函数，执行 build_gateway_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_gateway_settings(
    gateway_cfg_result: &Result<vw_config_types::gateway::GatewayConfig, String>,
) -> crate::app::state::GatewaySettingsState {
    let gateway_cfg = gateway_cfg_result.clone().unwrap_or_default();

    crate::app::state::GatewaySettingsState {
        active_tab: crate::app::state::GatewaySettingsTab::Config,
        port: gateway_cfg.port.clamp(1, u16::MAX),
        host_input: {
            let value = gateway_cfg.host.trim().to_string();
            if value.is_empty() { "127.0.0.1".to_string() } else { value }
        },
        auth_enabled: gateway_cfg.auth_enabled,
        skeys: gateway_cfg.skeys,
        new_skey_name_input: String::new(),
        new_skey_expires_at_input: String::new(),
        new_skey_calendar_month: String::new(),
        new_skey_calendar_open: false,
        last_created_skey: None,
        last_created_skey_copied: false,
        allow_public_bind: gateway_cfg.allow_public_bind,
        webhook_rate_limit_per_minute: gateway_cfg.webhook_rate_limit_per_minute.clamp(1, 100_000),
        trust_forwarded_headers: gateway_cfg.trust_forwarded_headers,
        rate_limit_max_keys: gateway_cfg.rate_limit_max_keys.min(u32::MAX as usize) as u32,
        idempotency_ttl_secs: gateway_cfg.idempotency_ttl_secs.min(u32::MAX as u64) as u32,
        idempotency_max_keys: gateway_cfg.idempotency_max_keys.min(u32::MAX as usize) as u32,
        node_control_enabled: gateway_cfg.node_control.enabled,
        node_control_auth_token_input: gateway_cfg.node_control.auth_token.unwrap_or_default(),
        node_control_allowed_node_ids_input: gateway_cfg.node_control.allowed_node_ids.join("\n"),
        service_action_running: None,
        service_action_output: None,
        copied_service_command: None,
        pending_service_command_confirmation: None,
        show_help_modal: false,
        save_error: None,
    }
}

/// 模块内可见函数，执行 build_gateway_client_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
/// 模块内可见函数，执行 build_agents_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。

/// 模块内可见函数，执行 build_memory_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_memory_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> crate::app::state::MemorySettingsState {
    let memory_cfg = &full_agent_cfg.memory;

    crate::app::state::MemorySettingsState {
        active_tab: crate::app::state::MemorySettingsTab::General,
        backend: match memory_cfg.backend.trim().to_ascii_lowercase().as_str() {
            "sqlite" | "postgres" | "qdrant" | "markdown" | "none" => {
                memory_cfg.backend.trim().to_ascii_lowercase()
            }
            "null" => "none".to_string(),
            _ => "sqlite".to_string(),
        },
        auto_save: memory_cfg.auto_save,
        hygiene_enabled: memory_cfg.hygiene_enabled,
        archive_after_days: memory_cfg.archive_after_days,
        purge_after_days: memory_cfg.purge_after_days,
        conversation_retention_days: memory_cfg.conversation_retention_days,
        embedding_provider: memory_cfg.embedding_provider.clone(),
        embedding_model: memory_cfg.embedding_model.clone(),
        embedding_dimensions: memory_cfg.embedding_dimensions.min(u32::MAX as usize) as u32,
        vector_weight: memory_cfg.vector_weight.clamp(0.0, 1.0) as f32,
        keyword_weight: memory_cfg.keyword_weight.clamp(0.0, 1.0) as f32,
        min_relevance_score: memory_cfg.min_relevance_score.clamp(0.0, 1.0) as f32,
        embedding_cache_size: memory_cfg.embedding_cache_size.min(u32::MAX as usize) as u32,
        chunk_max_tokens: memory_cfg.chunk_max_tokens.min(u32::MAX as usize) as u32,
        response_cache_enabled: memory_cfg.response_cache_enabled,
        response_cache_ttl_minutes: memory_cfg.response_cache_ttl_minutes,
        response_cache_max_entries: memory_cfg.response_cache_max_entries.min(u32::MAX as usize)
            as u32,
        snapshot_enabled: memory_cfg.snapshot_enabled,
        snapshot_on_hygiene: memory_cfg.snapshot_on_hygiene,
        auto_hydrate: memory_cfg.auto_hydrate,
        sqlite_open_timeout_secs: memory_cfg
            .sqlite_open_timeout_secs
            .unwrap_or_default()
            .min(u32::MAX as u64) as u32,
        qdrant_url_input: memory_cfg.qdrant.url.clone().unwrap_or_default(),
        qdrant_collection: {
            let value = memory_cfg.qdrant.collection.trim().to_string();
            if value.is_empty() { "vibewindow_memories".to_string() } else { value }
        },
        qdrant_api_key_input: memory_cfg.qdrant.api_key.clone().unwrap_or_default(),
        save_error: None,
    }
}

/// 模块内可见函数，执行 build_reliability_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_reliability_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> crate::app::state::ReliabilitySettingsState {
    let reliability_cfg = &full_agent_cfg.reliability;

    crate::app::state::ReliabilitySettingsState {
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

/// 模块内可见函数，执行 build_multimodal_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_multimodal_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> crate::app::state::MultimodalSettingsState {
    let multimodal_cfg = &full_agent_cfg.multimodal;

    crate::app::state::MultimodalSettingsState {
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

/// 模块内可见函数，执行 build_security_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_security_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> crate::app::state::SecuritySettingsState {
    let security_cfg = &full_agent_cfg.security;

    crate::app::state::SecuritySettingsState {
        active_tab: crate::app::state::SecuritySettingsTab::Bash,
        bash: security_cfg.bash.clone(),
        sandbox_enabled_input: match security_cfg.sandbox.enabled {
            None => "auto".to_string(),
            Some(true) => "true".to_string(),
            Some(false) => "false".to_string(),
        },
        sandbox_backend_input: match security_cfg.sandbox.backend {
            vw_config_types::security::SandboxBackend::Auto => "auto".to_string(),
            vw_config_types::security::SandboxBackend::Landlock => "landlock".to_string(),
            vw_config_types::security::SandboxBackend::Firejail => "firejail".to_string(),
            vw_config_types::security::SandboxBackend::Bubblewrap => "bubblewrap".to_string(),
            vw_config_types::security::SandboxBackend::Docker => "docker".to_string(),
            vw_config_types::security::SandboxBackend::None => "none".to_string(),
        },
        sandbox_firejail_args: security_cfg.sandbox.firejail_args.clone(),
        resources_max_memory_mb: security_cfg.resources.max_memory_mb.clamp(32, 65_536),
        resources_max_cpu_time_seconds: security_cfg
            .resources
            .max_cpu_time_seconds
            .clamp(1, 86_400),
        resources_max_subprocesses: security_cfg.resources.max_subprocesses.clamp(1, 10_000),
        resources_memory_monitoring: security_cfg.resources.memory_monitoring,
        audit_enabled: security_cfg.audit.enabled,
        audit_log_path: {
            let value = security_cfg.audit.log_path.trim().to_string();
            if value.is_empty() { "audit.log".to_string() } else { value }
        },
        audit_max_size_mb: security_cfg.audit.max_size_mb.clamp(1, 10_000),
        audit_sign_events: security_cfg.audit.sign_events,
        otp_enabled: security_cfg.otp.enabled,
        otp_method_input: match security_cfg.otp.method {
            vw_config_types::security::OtpMethod::Totp => "totp".to_string(),
            vw_config_types::security::OtpMethod::Pairing => "pairing".to_string(),
            vw_config_types::security::OtpMethod::CliPrompt => "cli-prompt".to_string(),
        },
        otp_token_ttl_secs: security_cfg.otp.token_ttl_secs.clamp(1, 600),
        otp_cache_valid_secs: security_cfg.otp.cache_valid_secs.clamp(1, 86_400),
        otp_gated_actions: security_cfg.otp.gated_actions.clone(),
        otp_gated_domains: security_cfg.otp.gated_domains.clone(),
        otp_gated_domain_categories: security_cfg.otp.gated_domain_categories.clone(),
        estop_enabled: security_cfg.estop.enabled,
        estop_state_file: {
            let value = security_cfg.estop.state_file.trim().to_string();
            if value.is_empty() { vw_config_types::paths::estop_state_file_path() } else { value }
        },
        estop_require_otp_to_resume: security_cfg.estop.require_otp_to_resume,
        syscall_anomaly_enabled: security_cfg.syscall_anomaly.enabled,
        syscall_anomaly_strict_mode: security_cfg.syscall_anomaly.strict_mode,
        syscall_anomaly_alert_on_unknown_syscall: security_cfg
            .syscall_anomaly
            .alert_on_unknown_syscall,
        syscall_anomaly_max_denied_events_per_minute: security_cfg
            .syscall_anomaly
            .max_denied_events_per_minute
            .clamp(1, 10_000),
        syscall_anomaly_max_total_events_per_minute: security_cfg
            .syscall_anomaly
            .max_total_events_per_minute
            .clamp(1, 100_000),
        syscall_anomaly_max_alerts_per_minute: security_cfg
            .syscall_anomaly
            .max_alerts_per_minute
            .clamp(1, 10_000),
        syscall_anomaly_alert_cooldown_secs: security_cfg
            .syscall_anomaly
            .alert_cooldown_secs
            .clamp(1, 3600),
        syscall_anomaly_log_path: {
            let value = security_cfg.syscall_anomaly.log_path.trim().to_string();
            if value.is_empty() { "syscall-anomalies.log".to_string() } else { value }
        },
        syscall_anomaly_baseline_syscalls: security_cfg.syscall_anomaly.baseline_syscalls.clone(),
        list_editor: crate::app::state::SecurityListEditorState::default(),
        canary_tokens: security_cfg.canary_tokens.clone(),
        semantic_guard: security_cfg.semantic_guard,
        semantic_guard_collection: {
            let value = security_cfg.semantic_guard_collection.trim().to_string();
            if value.is_empty() { "semantic_guard".to_string() } else { value }
        },
        semantic_guard_threshold: security_cfg.semantic_guard_threshold.clamp(0.0, 1.0),
        show_help_modal: false,
        save_error: None,
    }
}

/// 模块内可见函数，执行 build_autonomy_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_autonomy_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> crate::app::state::AutonomySettingsState {
    crate::app::state::AutonomySettingsState::from_config(&full_agent_cfg.autonomy)
}

/// 模块内可见函数，执行 build_observability_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_observability_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> crate::app::state::ObservabilitySettingsState {
    let observability_cfg = &full_agent_cfg.observability;

    crate::app::state::ObservabilitySettingsState {
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

/// 模块内可见函数，执行 build_storage_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_storage_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> crate::app::state::StorageSettingsState {
    let storage_cfg = &full_agent_cfg.storage;

    crate::app::state::StorageSettingsState {
        provider: storage_cfg.provider.config.provider.clone(),
        db_url_input: storage_cfg.provider.config.db_url.clone().unwrap_or_default(),
        schema: {
            let value = storage_cfg.provider.config.schema.trim().to_string();
            if value.is_empty() { "public".to_string() } else { value }
        },
        table: {
            let value = storage_cfg.provider.config.table.trim().to_string();
            if value.is_empty() { "memories".to_string() } else { value }
        },
        connect_timeout_secs_input: storage_cfg
            .provider
            .config
            .connect_timeout_secs
            .map(|value| value.to_string())
            .unwrap_or_default(),
        tls: storage_cfg.provider.config.tls,
        save_error: None,
    }
}

/// 模块内可见函数，执行 build_proxy_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_proxy_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> crate::app::state::ProxySettingsState {
    let proxy_cfg = &full_agent_cfg.proxy;

    crate::app::state::ProxySettingsState {
        enabled: proxy_cfg.enabled,
        http_proxy: proxy_cfg.http_proxy.clone().unwrap_or_default(),
        https_proxy: proxy_cfg.https_proxy.clone().unwrap_or_default(),
        all_proxy: proxy_cfg.all_proxy.clone().unwrap_or_default(),
        no_proxy_input: proxy_cfg.no_proxy.join(", "),
        scope: proxy_cfg.scope,
        services_input: proxy_cfg.services.join(", "),
        show_help_modal: false,
        save_error: None,
    }
}

/// 模块内可见函数，执行 build_tunnel_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_tunnel_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> crate::app::state::TunnelSettingsState {
    let tunnel_cfg = &full_agent_cfg.tunnel;

    crate::app::state::TunnelSettingsState {
        provider: match tunnel_cfg.provider.trim().to_ascii_lowercase().as_str() {
            "cloudflare" => "cloudflare".to_string(),
            "tailscale" => "tailscale".to_string(),
            "ngrok" => "ngrok".to_string(),
            "custom" => "custom".to_string(),
            _ => "none".to_string(),
        },
        cloudflare_token: tunnel_cfg
            .cloudflare
            .as_ref()
            .map(|config| config.token.clone())
            .unwrap_or_default(),
        tailscale_funnel: tunnel_cfg
            .tailscale
            .as_ref()
            .map(|config| config.funnel)
            .unwrap_or(false),
        tailscale_hostname: tunnel_cfg
            .tailscale
            .as_ref()
            .and_then(|config| config.hostname.clone())
            .unwrap_or_default(),
        ngrok_auth_token: tunnel_cfg
            .ngrok
            .as_ref()
            .map(|config| config.auth_token.clone())
            .unwrap_or_default(),
        ngrok_domain: tunnel_cfg
            .ngrok
            .as_ref()
            .and_then(|config| config.domain.clone())
            .unwrap_or_default(),
        custom_start_command: tunnel_cfg
            .custom
            .as_ref()
            .map(|config| config.start_command.clone())
            .unwrap_or_default(),
        custom_health_url: tunnel_cfg
            .custom
            .as_ref()
            .and_then(|config| config.health_url.clone())
            .unwrap_or_default(),
        custom_url_pattern: tunnel_cfg
            .custom
            .as_ref()
            .and_then(|config| config.url_pattern.clone())
            .unwrap_or_default(),
        save_error: None,
    }
}

pub(super) fn apply_bootstrap_agent_config(
    app: &mut App,
    full_agent_cfg: &vw_config_types::config::Config,
) {
    let gateway_cfg_result = Ok(full_agent_cfg.gateway.clone());

    app.embedding_routes_settings =
        build_embedding_routes_settings(full_agent_cfg, &gateway_cfg_result);
    app.model_routes_settings = build_model_routes_settings(full_agent_cfg);
    app.query_classification_settings = build_query_classification_settings(full_agent_cfg);
    app.heartbeat_settings = build_heartbeat_settings(full_agent_cfg);
    app.cron_settings = build_cron_settings(full_agent_cfg);
    app.scheduler_settings = build_scheduler_settings(full_agent_cfg);
    app.hooks_settings = build_hooks_settings(full_agent_cfg);
    app.runtime_settings = build_runtime_settings(full_agent_cfg);
    app.skills_settings = build_skills_settings(full_agent_cfg);
    app.web_search_settings = build_web_search_settings(full_agent_cfg);
    app.browser_settings = build_browser_settings(full_agent_cfg);
    app.http_request_settings = build_http_request_settings(full_agent_cfg);
    app.gateway_settings = build_gateway_settings(&gateway_cfg_result);
    app.memory_settings = build_memory_settings(full_agent_cfg);
    app.channels_settings =
        super::settings_builders::build_channels_settings(&full_agent_cfg.channels_config);
    app.reliability_settings = build_reliability_settings(full_agent_cfg);
    app.multimodal_settings = build_multimodal_settings(full_agent_cfg);
    app.security_settings = build_security_settings(full_agent_cfg);
    app.autonomy_settings = build_autonomy_settings(full_agent_cfg);
    app.observability_settings = build_observability_settings(full_agent_cfg);
    app.storage_settings = build_storage_settings(full_agent_cfg);
    app.proxy_settings = build_proxy_settings(full_agent_cfg);
    app.tunnel_settings = build_tunnel_settings(full_agent_cfg);
    app.acp_agents = super::sort_acp_agents(&full_agent_cfg.acp);
}
