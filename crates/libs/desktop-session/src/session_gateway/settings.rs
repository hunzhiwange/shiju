//! 技能、插件和外部应用网关操作。
use super::*;

pub async fn gateway_external_apps_async(endpoint: gateway_endpoint::GatewayEndpoint) -> Result<ExternalAppsStateDto, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).desktop_external_apps_get().await
}

pub async fn gateway_external_open_async(endpoint: gateway_endpoint::GatewayEndpoint, path: &str, target: &str) -> Result<(), String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).desktop_external_app_open(path, target).await
}

pub async fn gateway_external_reveal_async(endpoint: gateway_endpoint::GatewayEndpoint, path: &str) -> Result<(), String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).desktop_external_path_reveal(path).await
}

pub async fn gateway_skills_async(endpoint: gateway_endpoint::GatewayEndpoint,
    project_path: Option<&str>,
) -> Result<Vec<DesktopSkillCatalogEntryDto>, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).skills_get(project_path).await
}

pub async fn gateway_skill_detail_async(endpoint: gateway_endpoint::GatewayEndpoint,
    project_path: Option<&str>,
    skill_id: &str,
) -> Result<DesktopSkillDetailDto, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).skill_detail_get(project_path, skill_id).await
}

pub async fn gateway_skill_create_async(endpoint: gateway_endpoint::GatewayEndpoint, project_path: &str) -> Result<String, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).skill_create(project_path).await
}

pub async fn gateway_skill_install_builtin_async(endpoint: gateway_endpoint::GatewayEndpoint,
    project_path: &str,
    skill_id: &str,
) -> Result<String, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).skill_install_builtin(project_path, skill_id).await
}

pub async fn gateway_skill_set_enabled_async(endpoint: gateway_endpoint::GatewayEndpoint,
    project_path: Option<&str>,
    skill_id: &str,
    enabled: bool,
) -> Result<String, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).skill_set_enabled(project_path, skill_id, enabled).await
}

pub async fn gateway_skill_delete_async(endpoint: gateway_endpoint::GatewayEndpoint,
    project_path: Option<&str>,
    skill_id: &str,
) -> Result<String, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).skill_delete(project_path, skill_id).await
}

pub async fn gateway_skill_market_sources_async(endpoint: gateway_endpoint::GatewayEndpoint) -> Result<Vec<SkillMarketSourceDto>, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(vw_gateway_client::GatewayClient::new(endpoint.clone())?.transport().clone());
    client.skill_market_sources().await
}

pub async fn gateway_skill_market_recommended_async(endpoint: gateway_endpoint::GatewayEndpoint,
    source: &str,
) -> Result<Vec<SkillMarketSkillDto>, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(vw_gateway_client::GatewayClient::new(endpoint.clone())?.transport().clone());
    client.skill_market_recommended(source).await
}

pub async fn gateway_skill_market_skills_async(endpoint: gateway_endpoint::GatewayEndpoint,
    source: &str,
    query: &SkillMarketSkillQuery,
) -> Result<SkillMarketSkillPageDto, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(vw_gateway_client::GatewayClient::new(endpoint.clone())?.transport().clone());
    client.skill_market_skills(source, query).await
}

pub async fn gateway_skill_market_categories_async(endpoint: gateway_endpoint::GatewayEndpoint,
    source: &str,
) -> Result<Vec<SkillMarketCategoryDto>, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(vw_gateway_client::GatewayClient::new(endpoint.clone())?.transport().clone());
    client.skill_market_categories(source).await
}

pub async fn gateway_skill_market_skill_detail_async(endpoint: gateway_endpoint::GatewayEndpoint,
    locator: &SkillMarketSkillLocatorDto,
) -> Result<SkillMarketSkillDetailDto, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(vw_gateway_client::GatewayClient::new(endpoint.clone())?.transport().clone());
    client.skill_market_skill_detail(locator).await
}

pub async fn gateway_skill_market_skill_file_async(endpoint: gateway_endpoint::GatewayEndpoint,
    locator: &SkillMarketSkillLocatorDto,
) -> Result<SkillMarketSkillFileDto, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(vw_gateway_client::GatewayClient::new(endpoint.clone())?.transport().clone());
    client.skill_market_skill_file(locator).await
}

pub async fn gateway_skill_market_install_async(endpoint: gateway_endpoint::GatewayEndpoint,
    project_path: &str,
    coordinate: &SkillMarketCoordinateDto,
) -> Result<String, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(vw_gateway_client::GatewayClient::new(endpoint.clone())?.transport().clone());
    client.skill_market_install(project_path, coordinate).await
}

pub async fn gateway_skill_market_install_to_targets_async(endpoint: gateway_endpoint::GatewayEndpoint,
    project_path: Option<&str>,
    coordinate: &SkillMarketCoordinateDto,
    installations: &[SkillMarketInstallSelectionDto],
) -> Result<Vec<String>, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(vw_gateway_client::GatewayClient::new(endpoint.clone())?.transport().clone());
    client.skill_market_install_to_targets(project_path, coordinate, installations).await
}

pub async fn gateway_skill_market_skill_sets_async(endpoint: gateway_endpoint::GatewayEndpoint,
    source: &str,
    query: &SkillMarketSkillQuery,
) -> Result<SkillMarketSkillSetPageDto, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(vw_gateway_client::GatewayClient::new(endpoint.clone())?.transport().clone());
    client.skill_market_skill_sets(source, query).await
}

pub async fn gateway_skill_market_skill_set_detail_async(endpoint: gateway_endpoint::GatewayEndpoint,
    source: &str,
    slug: &str,
) -> Result<SkillMarketSkillSetDetailDto, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(vw_gateway_client::GatewayClient::new(endpoint.clone())?.transport().clone());
    client.skill_market_skill_set_detail(source, slug).await
}

pub async fn gateway_skill_market_skill_set_install_async(endpoint: gateway_endpoint::GatewayEndpoint,
    project_path: &str,
    source: &str,
    slug: &str,
) -> Result<Vec<String>, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(vw_gateway_client::GatewayClient::new(endpoint.clone())?.transport().clone());
    client.skill_market_skill_set_install(project_path, source, slug).await
}

pub async fn gateway_plugins_async(endpoint: gateway_endpoint::GatewayEndpoint) -> Result<Vec<DesktopPluginCatalogEntryDto>, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).plugins_status_get().await
}

pub async fn gateway_hooks_async(endpoint: gateway_endpoint::GatewayEndpoint) -> Result<Vec<HookCatalogEntryDto>, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).hooks_get().await
}

pub async fn gateway_hook_trust_async(endpoint: gateway_endpoint::GatewayEndpoint,
    plugin_id: &str,
    hook_hash: &str,
) -> Result<HookMutationResponseDto, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).hook_trust(plugin_id, hook_hash).await
}

pub async fn gateway_hook_revoke_async(endpoint: gateway_endpoint::GatewayEndpoint,
    plugin_id: &str,
    hook_hash: &str,
) -> Result<HookMutationResponseDto, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).hook_revoke(plugin_id, hook_hash).await
}

pub async fn gateway_hook_audit_async(endpoint: gateway_endpoint::GatewayEndpoint) -> Result<Vec<HookAuditEntryDto>, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).hook_audit_get().await
}

pub async fn gateway_plugin_set_enabled_async(endpoint: gateway_endpoint::GatewayEndpoint,
    plugin_id: &str,
    enabled: bool,
) -> Result<String, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).plugin_set_enabled(plugin_id, enabled).await
}

pub async fn gateway_plugin_delete_async(endpoint: gateway_endpoint::GatewayEndpoint, plugin_id: &str) -> Result<String, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).plugin_delete(plugin_id).await
}

pub async fn gateway_plugin_prepare_async(endpoint: gateway_endpoint::GatewayEndpoint,
    source: String,
    update_id: Option<String>,
) -> Result<gateway_desktop_settings::DesktopPluginPreviewDto, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    match update_id {
        Some(id) => {
            gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).plugin_prepare_update(&id, (!source.trim().is_empty()).then_some(source.trim()))
                .await
        }
        None => gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).plugin_prepare_install(source.trim()).await,
    }
}

pub async fn gateway_plugin_activate_async(endpoint: gateway_endpoint::GatewayEndpoint, token: String, update: bool) -> Result<String, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    if update {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).plugin_update(&token, true).await
    } else {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).plugin_install(&token, true).await
    }
}

pub async fn gateway_plugin_discard_async(endpoint: gateway_endpoint::GatewayEndpoint, token: String) -> Result<(), String> {
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(vw_gateway_client::GatewayClient::new(endpoint.clone())?.transport().clone()).plugin_cancel(&token).await
}

pub async fn gateway_skill_repos_scope(endpoint: gateway_endpoint::GatewayEndpoint, directory: &str, host: String) -> Result<vw_api_app_skills::repos_api::SkillReposQuery, String> {
    let project = gateway_project::GatewayProjectClient::new(vw_gateway_client::GatewayClient::new(endpoint.clone())?.transport().clone()).project_resolve(&endpoint.tenant_scope().clone(),
        &vw_api_project::ResolveProjectRequest {
            directory: directory.to_owned(), create_if_missing: false,
        }).await?.project;
    Ok(vw_api_app_skills::repos_api::SkillReposQuery {
        project_id: project.id, target_host_id: host,
    })
}

pub async fn gateway_skill_repos_request(endpoint: gateway_endpoint::GatewayEndpoint,
    scope: vw_api_app_skills::repos_api::SkillReposQuery,
    version: u64,
    change: Option<(vw_api_app_skills::repos_api::SkillRepoEntry, bool)>,
) -> Result<vw_api_app_skills::repos_api::SkillReposResponse, String> {
    use vw_api_app_skills::repos_api::*;
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    let skills = gateway_app_skills::GatewayAppSkillsClient::new(client.transport().clone());
    match change {
        None => skills.skill_repos_list(&scope).await,
        Some((repo, true)) => skills.skill_repo_remove(&repo.owner, &repo.name,
            &RemoveSkillRepoRequest { scope, expected_version: version }).await,
        Some((repo, false)) => skills.skill_repo_save(&SaveSkillRepoRequest {
            scope, expected_version: version, repo,
        }).await,
    }
}

pub async fn gateway_claude_config_request(endpoint: gateway_endpoint::GatewayEndpoint,
    scope: vw_api_app_skills::claude_api::ClaudeConfigQuery,
    token: String,
    change: Option<(bool, vw_api_app_skills::claude_api::ClaudeConfigAction)>,
) -> Result<(vw_api_app_skills::claude_api::ClaudeConfigResponse, Option<String>), String> {
    use vw_api_app_skills::claude_api::*;
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    let skills = gateway_app_skills::GatewayAppSkillsClient::new(client.transport().clone());
    let notice = if let Some((onboarding, action)) = change {
        let request = ClaudeConfigRequest { scope: scope.clone(), action };
        let response = if onboarding { skills.claude_onboarding(&request, &token).await? }
            else { skills.claude_config_marker(&request, &token).await? };
        if response.scope.project_id != scope.project_id || response.scope.target_host_id != scope.target_host_id {
            return Err("Claude response scope mismatch".into());
        }
        Some(if response.changed { "操作已保存" } else { "配置已符合所选操作" }.to_owned())
    } else { None };
    Ok((skills.claude_config_status(&scope, &token).await?, notice))
}

pub async fn gateway_skills_market_request(endpoint: gateway_endpoint::GatewayEndpoint,
    project: String, host: String, query: String, token: String,
) -> Result<desktop_settings_skills::market_view::Results, String> {
    use vw_api_app_skills::{repos_api::SkillReposQuery,
        discovery_api::DiscoverSkillsRequest, search_api::MarketSearchQuery};
    use desktop_settings_skills::market_view::Results;
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    let skills = gateway_app_skills::GatewayAppSkillsClient::new(client.transport().clone());
    if query.is_empty() {
        skills.agent_apps_skills_discover(&DiscoverSkillsRequest {
            scope: SkillReposQuery { project_id: project.into(), target_host_id: host },
            repo_id: None, force_refresh: false,
        }, &token).await.map(Results::Discovery).map_err(|e| e.to_string())
    } else {
        skills.agent_apps_market_search(&MarketSearchQuery {
            project_id: project.into(), target_host_id: host, query, limit: 100, offset: 0,
        }, &token).await.map(Results::Search).map_err(|e| e.to_string())
    }
}

pub async fn gateway_skills_install_view(endpoint: gateway_endpoint::GatewayEndpoint,
    scope: vw_api_app_skills::repos_api::SkillReposQuery,
    request: Option<vw_api_app_skills::install_api::InstallSkillRequest>,
    token: String,
) -> Result<Option<vw_api_app_skills::inventory_api::SkillInventoryResponse>, String> {
    use vw_api_app_skills::jobs_api::SkillJobPhase;
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    let skills = gateway_app_skills::GatewayAppSkillsClient::new(client.transport().clone());
    let Some(request) = request else {
        return skills.agent_apps_skills_inventory(&scope, &token).await.map(Some).map_err(|e| e.to_string());
    };
    let response = skills.agent_apps_skills_install(&request, &token).await.map_err(|e| e.to_string())?;
    if response.scope != scope { return Err("安装响应目标不匹配".into()); }
    loop {
        let job = skills.agent_apps_skills_job(&scope, &response.task_id, &token).await.map_err(|e| e.to_string())?;
        if job.scope != scope || job.task_id != response.task_id { return Err("安装任务目标不匹配".into()); }
        match job.phase {
            SkillJobPhase::Completed if !job.requires_reconciliation => return Ok(None),
            SkillJobPhase::Completed | SkillJobPhase::Failed | SkillJobPhase::Interrupted =>
                return Err("安装未完成或需要核对目标目录，请刷新后检查".into()),
            _ => tokio::time::sleep(std::time::Duration::from_secs(1)).await,
        }
    }
}

pub async fn gateway_skills_import_request(endpoint: gateway_endpoint::GatewayEndpoint,
    project: String, host: String,
    apps: Vec<vw_api_app_identity::AppId>,
    selected: Vec<vw_api_app_skills::import_api::UnmanagedSkillItem>,
    token: String,
) -> (Vec<(String, Result<(), String>)>, Result<vw_api_app_skills::import_api::UnmanagedSkillsResponse, String>) {
    use vw_api_app_skills::{import_api::*, repos_api::SkillReposQuery};
    let mut outcomes = Vec::new();
    let result = async {
        let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    let skills = gateway_app_skills::GatewayAppSkillsClient::new(client.transport().clone());
        let app_names = apps.iter().map(|id| {
            serde_json::to_value(id).map_err(|_| "应用标识无效".to_owned())?
                .as_str().map(str::to_owned).ok_or_else(|| "应用标识无效".to_owned())
        }).collect::<Result<Vec<_>, String>>()?;
        let query = UnmanagedSkillsQuery { project_id: project.into(), target_host_id: host.clone(),
            apps: app_names.join(",") };
        let mut report = skills.agent_apps_skills_unmanaged(&query, &token).await.map_err(|e| e.to_string())?;
        for row in selected {
            if report.scope.project_id != query.project_id || report.scope.target_host_id != host {
                return Err("扫描响应作用域不匹配".to_owned());
            }
            let result = async {
                let current = report.skills.iter().find(|s| s.directory == row.directory
                    && s.content_hash == row.content_hash && !s.managed && !s.conflict && !s.external_link)
                    .ok_or_else(|| "技能来源已变化，请重新扫描".to_owned())?;
                let selection_id = current.selection_id.clone().ok_or_else(|| "技能不可导入".to_owned())?;
                let targets: Vec<_> = apps.iter().copied().filter(|id| current.apps.contains(id)).collect();
                if targets.is_empty() { return Err("未选择来源应用".to_owned()); }
                let imported = skills.agent_apps_skills_import(&ImportSkillRequest {
                    scope: SkillReposQuery { project_id: query.project_id.clone(), target_host_id: host.clone() },
                    expected_version: report.version, selection_id, apps: targets,
                }, &token).await.map_err(|e| e.to_string())?;
                if imported.scope.project_id != query.project_id || imported.scope.target_host_id != host
                    || imported.directory != row.directory {
                    return Err("导入响应目标不匹配，请重新扫描确认实际状态".to_owned());
                }
                Ok(())
            }.await;
            outcomes.push((row.directory, result));
            report = skills.agent_apps_skills_unmanaged(&query, &token).await.map_err(|e| e.to_string())?;
        }
        Ok(report)
    }.await;
    (outcomes, result)
}

pub async fn gateway_skills_job(endpoint: gateway_endpoint::GatewayEndpoint,
    scope: vw_api_app_skills::repos_api::SkillReposQuery,
    id: vw_api_transport::TaskId,
    token: String,
    cancel: bool,
) -> Result<vw_api_app_skills::jobs_api::SkillJobResponse, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    let skills = gateway_app_skills::GatewayAppSkillsClient::new(client.transport().clone());
    if cancel { skills.agent_apps_skills_cancel_job(&scope, &id, &token).await }
    else { skills.agent_apps_skills_job(&scope, &id, &token).await }.map_err(|e| e.to_string())
}

pub async fn gateway_skills_submit(endpoint: gateway_endpoint::GatewayEndpoint,
    request: vw_api_app_skills::install_api::InstallSkillRequest,
    token: String,
) -> Result<vw_api_app_skills::install_api::InstallSkillResponse, String> {
    vw_gateway_client::GatewayClient::new(endpoint.clone())?.agent_apps_skills_install(&request, &token).await.map_err(|e| e.to_string())
}

pub async fn gateway_skill_bindings_request(endpoint: gateway_endpoint::GatewayEndpoint,
    scope: vw_api_app_skills::repos_api::SkillReposQuery,
    token: String,
    change: Option<desktop_settings_skills::update::bindings_view::Change>,
) -> Result<desktop_settings_skills::update::bindings_view::Readback, String> {
    use desktop_settings_skills::update::bindings_view::{Change, Readback, complete_success};
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    let skills = gateway_app_skills::GatewayAppSkillsClient::new(client.transport().clone());
    let changed = change.is_some();
    let mut resync_targets = None;
    let mut results = Vec::new();
    let mut error = None;
    match change {
        Some(Change::Save(request)) => {
            if skills.agent_apps_skill_binding_targets(&scope, Some(&request), &token).await.is_err() {
                error = Some("保存目标偏好失败，请刷新后重试".into());
            }
        }
        Some(Change::Bind(id, request)) => {
            match skills.agent_apps_skills_bindings(&id, &request, &token).await {
                Ok(response) if response.scope == scope => {
                    if !complete_success(&response.results, &request.targets, &id, request.enabled) {
                        error = Some("绑定未全部成功，请检查逐项结果".into());
                    }
                    results = response.results;
                }
                _ => error = Some("绑定失败，请重读确认实际状态".into()),
            }
        }
        Some(Change::Resync(request)) => {
            resync_targets = Some(request.targets.clone());
            match skills.agent_apps_skills_resync(&request, &token).await {
                Ok(response) if response.scope == scope => {
                    // Resync can legitimately have no pairs when no skills are installed.
                    if response.results.iter().any(|r| r.status != vw_api_app_skills::bindings_api::SkillBindingStatus::Enabled) {
                        error = Some("重新同步未全部成功，请检查逐项结果".into());
                    }
                    results = response.results;
                }
                _ => error = Some("重新同步失败，请刷新确认".into()),
            }
        }
        None => {}
    }
    let snapshot = match skills.agent_apps_skill_binding_targets(&scope, None, &token).await {
        Ok(snapshot) => Some(snapshot),
        Err(cause) => {
            error.get_or_insert_with(|| format!("读取绑定目标失败：{cause}；请刷新确认实际状态"));
            None
        }
    };
    let inventory = match skills.agent_apps_skills_inventory(&scope, &token).await {
        Ok(inventory) => Some(inventory),
        Err(cause) => {
            error.get_or_insert_with(|| format!("读取技能库存失败：{cause}；请刷新确认实际状态"));
            None
        }
    };
    if let (Some(targets), Some(inventory)) = (resync_targets, inventory.as_ref()) {
        let skills: Vec<_> = inventory.skills.iter().filter(|s| s.present).collect();
        if results.len() != skills.len() * targets.len() || skills.iter().any(|skill|
            !complete_success(&results.iter().filter(|r| r.directory == skill.id).cloned().collect::<Vec<_>>(), &targets, &skill.id, true)) {
            error = Some("重新同步未全部成功，请检查逐项结果".into());
        }
    }
    if snapshot.is_none() || inventory.is_none() {
        error.get_or_insert_with(|| "重读失败，已保留逐项写入结果，请刷新确认实际状态".into());
    }
    Ok(Readback { snapshot, inventory, results, error, changed })
}

pub async fn gateway_skills_mutation(endpoint: gateway_endpoint::GatewayEndpoint,
    scope: vw_api_app_skills::repos_api::SkillReposQuery,
    operation: (desktop_settings_skills::updates_view::Operation, u64, Option<String>),
    token: String,
) -> Result<desktop_settings_skills::updates_view::Outcome, String> {
    use desktop_settings_skills::updates_view::{Operation, Outcome};
    use vw_api_app_skills::{updates_api::*, uninstall_api::*};
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    let skills = gateway_app_skills::GatewayAppSkillsClient::new(client.transport().clone());
    let (operation, expected_version, hash) = operation;
    match operation {
        Operation::Uninstall(id) => {
            let response = skills.agent_apps_skills_uninstall(&id, &UninstallSkillRequest { scope: scope.clone(), expected_version }, &token).await.map_err(|e| e.to_string())?;
            if response.scope != scope { return Err("卸载响应目标不匹配".into()); }
            Ok(Outcome::Uninstalled(id, response))
        }
        Operation::Update(id) => {
            let expected_hash = hash.ok_or("缺少已检查的当前哈希")?;
            let mut task = skills.agent_apps_skills_update(&id, &UpdateSkillRequest { scope: scope.clone(), expected_version, expected_hash }, &token).await.map_err(|e| e.to_string())?;
            let task_id = task.task_id.clone();
            for _ in 0..300 {
                if task.scope != scope || task.task_id != task_id || task.directory.as_deref().is_some_and(|d| d != id) {
                    return Err("更新任务目标不匹配".into());
                }
                match task.phase {
                    SkillUpdatePhase::Completed | SkillUpdatePhase::Failed | SkillUpdatePhase::Interrupted => return Ok(Outcome::Updated(task)),
                    _ => {}
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                task = skills.agent_apps_skills_update_task(task_id.as_ref(), &scope, &token).await.map_err(|e| e.to_string())?;
            }
            Err("更新仍在执行，请稍后刷新核对实际状态".into())
        }
    }
}

pub async fn gateway_skill_storage_request(endpoint: gateway_endpoint::GatewayEndpoint,
    scope: vw_api_app_skills::repos_api::SkillReposQuery,
    token: String,
    change: desktop_settings_skills::update::storage_modal::Change,
) -> Result<desktop_settings_skills::update::storage_modal::Readback, String> {
    use desktop_settings_skills::update::storage_modal::{Change, Readback};
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    let skills = gateway_app_skills::GatewayAppSkillsClient::new(client.transport().clone());
    match change {
        Change::OpenDirectory(path) => {
            tokio::task::spawn_blocking(move || {
                let home = desktop_settings_apps::system_settings_apps::local::launch_directory(&client, &scope.target_host_id, &token, None)
                    .ok_or_else(|| "目标主机与本机授权不匹配".to_string())?;
                desktop_settings_skills::update::storage_modal::open_local(&home, &path)
                    .map_err(|_| "无法打开主副本目录".to_string())?;
                Ok(Readback::Opened)
            }).await.map_err(|_| "打开目录任务失败".to_string())?
        }
        Change::Read => skills.agent_apps_skills_storage(&scope, &token).await.map(Readback::Storage).map_err(|e| e.to_string()),
        Change::Mode(request) => skills.agent_apps_skills_storage_patch(&request, &token).await.map(Readback::Storage).map_err(|e| e.to_string()),
        Change::Move(mut request) => {
            let targets = skills.agent_apps_skill_binding_targets(&scope, None, &token).await.map_err(|e| e.to_string())?;
            if targets.scope != scope { return Err("目标作用域不匹配".into()); }
            request.targets = targets.targets;
            skills.agent_apps_skills_storage_move(&request, &token).await.map(Readback::Move).map_err(|e| e.to_string())
        }
        Change::Resync(mut request) => {
            let targets = skills.agent_apps_skill_binding_targets(&scope, None, &token).await.map_err(|e| e.to_string())?;
            if targets.scope != scope || targets.targets.is_empty() { return Err("请先配置应用绑定目标".into()); }
            request.targets = targets.targets;
            skills.agent_apps_skills_resync(&request, &token).await.map(Readback::Sync).map_err(|e| e.to_string())
        }
    }
}

pub async fn gateway_skill_backups_request(endpoint: gateway_endpoint::GatewayEndpoint,
    scope: vw_api_app_skills::repos_api::SkillReposQuery,
    token: String,
    change: desktop_settings_skills::update::backup_modal::Change,
) -> Result<desktop_settings_skills::update::backup_modal::Readback, String> {
    use desktop_settings_skills::update::backup_modal::{Change, Readback};
    use vw_api_app_skills::backups_api::DeleteSkillBackupRequest;
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone()).map_err(|_| "无法连接备份服务".to_string())?;
    let skills = gateway_app_skills::GatewayAppSkillsClient::new(client.transport().clone());
    match change {
        Change::Read => {
            let list = skills.agent_apps_skills_backups(&scope, &token).await.map_err(|e| e.to_string())?;
            let inventory = skills.agent_apps_skills_inventory(&scope, &token).await.map_err(|e| e.to_string())?;
            let bindings = skills.agent_apps_skill_binding_targets(&scope, None, &token).await.map_err(|e| e.to_string())?;
            if list.scope != scope || inventory.scope != scope || bindings.scope != scope {
                return Err("备份读取作用域不匹配".into());
            }
            Ok(Readback::List(list, inventory.version, bindings.targets))
        }
        Change::Delete(id) => skills.agent_apps_skills_backup_delete(&id, &DeleteSkillBackupRequest { scope }, &token)
            .await.map(Readback::Deleted).map_err(|e| e.to_string()),
        Change::Restore(id, request) => skills.agent_apps_skills_backup_restore(&id, &request, &token)
            .await.map(Readback::Restored).map_err(|e| e.to_string()),
    }
}
