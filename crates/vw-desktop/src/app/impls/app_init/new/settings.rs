//! 组织桌面应用初始化阶段的 settings.rs 逻辑。
//! 本模块把启动输入、配置加载和初始状态装配拆开，便于定位启动失败路径。

use std::collections::BTreeSet;

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

/// 模块内可见函数，执行 build_goal_loop_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_goal_loop_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> crate::app::state::GoalLoopSettingsState {
    let goal_loop_cfg = &full_agent_cfg.goal_loop;

    crate::app::state::GoalLoopSettingsState {
        enabled: goal_loop_cfg.enabled,
        interval_minutes_input: goal_loop_cfg.interval_minutes.to_string(),
        step_timeout_secs_input: goal_loop_cfg.step_timeout_secs.to_string(),
        max_steps_per_cycle_input: goal_loop_cfg.max_steps_per_cycle.to_string(),
        channel_input: goal_loop_cfg.channel.clone().unwrap_or_default(),
        target_input: goal_loop_cfg.target.clone().unwrap_or_default(),
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

/// 模块内可见函数，执行 build_sop_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_sop_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> crate::app::state::SopSettingsState {
    let sop_cfg = &full_agent_cfg.sop;

    crate::app::state::SopSettingsState {
        sops_dir_input: sop_cfg.sops_dir.clone().unwrap_or_default(),
        default_execution_mode: match sop_cfg.default_execution_mode {
            vw_config_types::automation::SopExecutionMode::Auto => "autonomous".to_string(),
            _ => "supervised".to_string(),
        },
        max_finished_runs: sop_cfg.max_finished_runs.min(100_000),
        max_concurrent_total: sop_cfg.max_concurrent_total.clamp(1, 1_000),
        approval_timeout_secs: sop_cfg.approval_timeout_secs.min(86_400),
        save_error: None,
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
            "native" | "docker" | "wasm" => runtime_cfg.kind.clone(),
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
        wasm_tools_dir: {
            let value = runtime_cfg.wasm.tools_dir.trim().to_string();
            if value.is_empty() { "tools/wasm".to_string() } else { value }
        },
        wasm_fuel_limit_input: runtime_cfg.wasm.fuel_limit.clamp(1, 100_000_000).to_string(),
        wasm_memory_limit_mb_input: runtime_cfg.wasm.memory_limit_mb.clamp(1, 4096).to_string(),
        wasm_max_module_size_mb_input: runtime_cfg
            .wasm
            .max_module_size_mb
            .clamp(1, 4096)
            .to_string(),
        wasm_allow_workspace_read: runtime_cfg.wasm.allow_workspace_read,
        wasm_allow_workspace_write: runtime_cfg.wasm.allow_workspace_write,
        wasm_allowed_hosts_input: runtime_cfg.wasm.allowed_hosts.join(", "),
        wasm_require_workspace_relative_tools_dir: runtime_cfg
            .wasm
            .security
            .require_workspace_relative_tools_dir,
        wasm_reject_symlink_modules: runtime_cfg.wasm.security.reject_symlink_modules,
        wasm_reject_symlink_tools_dir: runtime_cfg.wasm.security.reject_symlink_tools_dir,
        wasm_strict_host_validation: runtime_cfg.wasm.security.strict_host_validation,
        wasm_capability_escalation_mode: match runtime_cfg.wasm.security.capability_escalation_mode
        {
            vw_config_types::runtime::WasmCapabilityEscalationMode::Deny => "deny".to_string(),
            vw_config_types::runtime::WasmCapabilityEscalationMode::Clamp => "clamp".to_string(),
        },
        wasm_module_hash_policy: match runtime_cfg.wasm.security.module_hash_policy {
            vw_config_types::runtime::WasmModuleHashPolicy::Disabled => "disabled".to_string(),
            vw_config_types::runtime::WasmModuleHashPolicy::Warn => "warn".to_string(),
            vw_config_types::runtime::WasmModuleHashPolicy::Enforce => "enforce".to_string(),
        },
        wasm_module_sha256_input: runtime_cfg
            .wasm
            .security
            .module_sha256
            .iter()
            .map(|(module, hash)| format!("{module}:{hash}"))
            .collect::<Vec<_>>()
            .join("\n"),
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
        open_skills_enabled: skills_cfg.open_skills_enabled,
        directory_provider: skills_cfg.directory_provider,
        open_skills_dir_input: skills_cfg.open_skills_dir.clone().unwrap_or_default(),
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

/// 模块内可见函数，执行 build_research_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_research_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> crate::app::state::ResearchSettingsState {
    let research_cfg = &full_agent_cfg.research;

    crate::app::state::ResearchSettingsState {
        enabled: research_cfg.enabled,
        trigger: research_cfg.trigger,
        keywords_input: research_cfg.keywords.join(", "),
        min_message_length: research_cfg.min_message_length.clamp(1, 10_000) as u32,
        max_iterations: research_cfg.max_iterations.clamp(1, 100) as u32,
        show_progress: research_cfg.show_progress,
        system_prompt_prefix: research_cfg.system_prompt_prefix.clone(),
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
            "VibeWindow/1.0".to_string()
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
        show_help_modal: false,
        save_error: None,
    }
}

/// 模块内可见函数，执行 build_gateway_client_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_gateway_client_settings(
    gateway_client_cfg: &vw_config_types::ui::GatewayClientSystemSettingsConfig,
) -> crate::app::state::GatewayClientSettingsState {
    let servers = gateway_client_cfg
        .normalized_servers()
        .iter()
        .map(crate::app::state::GatewayClientServerDraft::from_config)
        .collect::<Vec<_>>();
    let active = gateway_client_cfg.active_server();

    crate::app::state::GatewayClientSettingsState {
        selected_server_id: active.id.clone(),
        name_input: active.name.clone(),
        servers,
        health: std::collections::HashMap::new(),
        host_input: {
            let value = active.host.trim().to_string();
            if value.is_empty() { "127.0.0.1".to_string() } else { value }
        },
        port: active.port.clamp(1, u16::MAX),
        platform_id_input: active.platform_id.clone(),
        company_id_input: active.company_id.clone(),
        skey_input: if active.skey.trim().is_empty() {
            active.bearer_token.clone()
        } else {
            active.skey.clone()
        },
        pending_remove_server_id: None,
        show_help_modal: false,
        save_error: None,
    }
}

/// 模块内可见函数，执行 build_agents_ipc_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_agents_ipc_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> crate::app::state::AgentsIpcSettingsState {
    let agents_ipc_cfg = &full_agent_cfg.agents_ipc;

    crate::app::state::AgentsIpcSettingsState {
        enabled: agents_ipc_cfg.enabled,
        db_path_input: agents_ipc_cfg.db_path.clone(),
        staleness_secs: agents_ipc_cfg.staleness_secs.clamp(1, 86_400),
        show_help_modal: false,
        save_error: None,
    }
}

/// 模块内可见函数，执行 build_agents_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_agents_settings(
    cfg: &serde_json::Value,
    full_agent_cfg: &vw_config_types::config::Config,
) -> crate::app::state::AgentsSettingsState {
    let delegate_agents_cfg = &full_agent_cfg.agents;
    let agent_cfg = &full_agent_cfg.agent;
    let default_provider_cfg = full_agent_cfg.default_provider.clone();
    let default_model_cfg = full_agent_cfg.default_model.clone();
    let default_temperature_cfg = full_agent_cfg.default_temperature;
    let ordered_keys = crate::app::state::ordered_agent_keys(delegate_agents_cfg);

    let entries = ordered_keys
        .into_iter()
        .map(|key| {
            let config = delegate_agents_cfg.get(&key).cloned();
            if key == "main" {
                let config = config.unwrap_or_else(|| {
                    vw_config_types::agent::builtin_agent_config("main").unwrap_or_default()
                });
                let entry = vw_config_types::agent::DelegateAgentConfig {
                    label: config.label,
                    description: config.description,
                    builtin: config.builtin,
                    mode: "primary".to_string(),
                    enabled: true,
                    provider: if config.provider.trim().is_empty() {
                        default_provider_cfg.clone().unwrap_or_default()
                    } else {
                        config.provider
                    },
                    model: if config.model.trim().is_empty() {
                        default_model_cfg
                            .as_deref()
                            .and_then(|value| value.split('/').next_back())
                            .unwrap_or_default()
                            .to_string()
                    } else {
                        config.model
                    },
                    system_prompt: config.system_prompt,
                    api_key: config.api_key,
                    temperature: Some(config.temperature.unwrap_or(default_temperature_cfg)),
                    top_p: config.top_p,
                    identity_format: Some("openclaw".to_string()),
                    hidden: config.hidden,
                    max_depth: config.max_depth,
                    agentic: config.agentic,
                    allowed_tools: config.allowed_tools,
                    allowed_skills: config.allowed_skills,
                    options: config.options,
                    permission: config.permission,
                    max_iterations: config.max_iterations,
                    steps: config.steps,
                };
                let mut ui_entry =
                    crate::app::state::DelegateAgentSettingsEntry::from_config(&key, Some(entry));
                ui_entry.compact_context = agent_cfg.compact_context;
                ui_entry.max_tool_iterations = agent_cfg.max_tool_iterations.clamp(1, 200) as u32;
                ui_entry.max_history_messages =
                    agent_cfg.max_history_messages.clamp(1, 1000) as u32;
                ui_entry.parallel_tools = agent_cfg.parallel_tools;
                ui_entry.tool_dispatcher = {
                    let value = agent_cfg.tool_dispatcher.trim().to_string();
                    if value.is_empty() { "auto".to_string() } else { value }
                };
                ui_entry
            } else {
                crate::app::state::DelegateAgentSettingsEntry::from_config(&key, config)
            }
        })
        .collect::<Vec<_>>();

    crate::app::state::AgentsSettingsState {
        loading: false,
        providers: Vec::new(),
        provider_models: Vec::new(),
        entries,
        new_agent_key_input: String::new(),
        selected_agent: crate::app::state::MAIN_AGENT_KEY.to_string(),
        active_detail_tab: crate::app::state::AGENT_DETAIL_BASIC_TAB.to_string(),
        active_prompt_tab: crate::app::state::AGENT_PROMPT_SYSTEM_TAB.to_string(),
        workspace_identity_files: crate::app::state::WORKSPACE_IDENTITY_FILES
            .iter()
            .map(|(file_name, label)| crate::app::state::WorkspaceIdentityFileState {
                file_name: (*file_name).to_string(),
                label: (*label).to_string(),
                editor: iced::widget::text_editor::Content::with_text(""),
                size_bytes: None,
                modified_at_ms: None,
            })
            .collect(),
        workspace_identity_root_path: None,
        available_tools: config::load_tools_list_via_gateway(),
        save_error: None,
    }
}

/// 模块内可见函数，执行 build_staff_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_staff_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> crate::app::state::StaffSettingsState {
    crate::app::state::StaffSettingsState::from_config(full_agent_cfg.staff.clone())
}

/// 模块内可见函数，执行 build_coordination_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_coordination_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> crate::app::state::CoordinationSettingsState {
    let coordination_cfg = &full_agent_cfg.coordination;

    crate::app::state::CoordinationSettingsState {
        enabled: coordination_cfg.enabled,
        lead_agent_input: coordination_cfg.lead_agent.clone(),
        max_inbox_messages_per_agent: coordination_cfg.max_inbox_messages_per_agent.clamp(1, 10_000)
            as u32,
        max_dead_letters: coordination_cfg.max_dead_letters.clamp(1, 10_000) as u32,
        max_context_entries: coordination_cfg.max_context_entries.clamp(1, 20_000) as u32,
        max_seen_message_ids: coordination_cfg.max_seen_message_ids.clamp(1, 100_000) as u32,
        show_help_modal: false,
        save_error: None,
    }
}

/// 模块内可见函数，执行 build_memory_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_memory_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> crate::app::state::MemorySettingsState {
    let memory_cfg = &full_agent_cfg.memory;

    crate::app::state::MemorySettingsState {
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
        sandbox_firejail_args_input: security_cfg.sandbox.firejail_args.join(", "),
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
        otp_gated_actions_input: security_cfg.otp.gated_actions.join(", "),
        otp_gated_domains_input: security_cfg.otp.gated_domains.join(", "),
        otp_gated_domain_categories_input: security_cfg.otp.gated_domain_categories.join(", "),
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
        syscall_anomaly_baseline_syscalls_input: security_cfg
            .syscall_anomaly
            .baseline_syscalls
            .join(", "),
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
    let autonomy_cfg = &full_agent_cfg.autonomy;

    crate::app::state::AutonomySettingsState {
        level: autonomy_cfg.level,
        workspace_only: autonomy_cfg.workspace_only,
        allowed_commands_input: autonomy_cfg.allowed_commands.join(", "),
        forbidden_paths_input: autonomy_cfg.forbidden_paths.join(", "),
        max_actions_per_hour: autonomy_cfg.max_actions_per_hour,
        max_cost_per_day_cents: autonomy_cfg.max_cost_per_day_cents,
        require_approval_for_medium_risk: autonomy_cfg.require_approval_for_medium_risk,
        block_high_risk_commands: autonomy_cfg.block_high_risk_commands,
        shell_redirect_policy: autonomy_cfg.shell_redirect_policy,
        shell_env_passthrough_input: autonomy_cfg.shell_env_passthrough.join(", "),
        auto_approve_input: autonomy_cfg.auto_approve.join(", "),
        always_ask_input: autonomy_cfg.always_ask.join(", "),
        allowed_roots_input: autonomy_cfg.allowed_roots.join(", "),
        non_cli_excluded_tools_input: autonomy_cfg.non_cli_excluded_tools.join(", "),
        non_cli_approval_approvers_input: autonomy_cfg.non_cli_approval_approvers.join(", "),
        non_cli_natural_language_approval_mode: autonomy_cfg.non_cli_natural_language_approval_mode,
        non_cli_natural_language_approval_mode_by_channel_input: autonomy_cfg
            .non_cli_natural_language_approval_mode_by_channel
            .iter()
            .map(|(channel, mode)| {
                let mode = match mode {
                    vw_config_types::security::NonCliNaturalLanguageApprovalMode::Disabled => "disabled",
                    vw_config_types::security::NonCliNaturalLanguageApprovalMode::RequestConfirm => "request_confirm",
                    vw_config_types::security::NonCliNaturalLanguageApprovalMode::Direct => "direct",
                };
                format!("{channel}:{mode}")
            })
            .collect::<Vec<_>>()
            .join(", "),
        show_help_modal: false,
        save_error: None,
    }
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

/// 模块内可见函数，执行 build_composio_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_composio_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> crate::app::state::ComposioSettingsState {
    let composio_cfg = &full_agent_cfg.composio;

    crate::app::state::ComposioSettingsState {
        enabled: composio_cfg.enabled,
        api_key_input: composio_cfg.api_key.clone().unwrap_or_default(),
        entity_id_input: {
            let value = composio_cfg.entity_id.trim().to_string();
            if value.is_empty() { "default".to_string() } else { value }
        },
        save_error: None,
    }
}

/// 模块内可见函数，执行 build_transcription_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_transcription_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> crate::app::state::TranscriptionSettingsState {
    let transcription_cfg = &full_agent_cfg.transcription;

    crate::app::state::TranscriptionSettingsState {
        enabled: transcription_cfg.enabled,
        api_url: transcription_cfg.api_url.clone(),
        model: transcription_cfg.model.clone(),
        language: transcription_cfg.language.clone().unwrap_or_default(),
        max_duration_secs: transcription_cfg.max_duration_secs.clamp(1, 3600),
        show_help_modal: false,
        save_error: None,
    }
}

/// 模块内可见函数，执行 build_task_board_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_task_board_settings(
    _cfg: &serde_json::Value,
) -> crate::app::task::TaskBoardSettings {
    crate::app::task::TaskBoardSettings::new()
}
#[cfg(test)]
#[path = "settings_tests.rs"]
mod settings_tests;
