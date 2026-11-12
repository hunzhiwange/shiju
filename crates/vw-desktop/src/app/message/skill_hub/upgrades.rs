//! 已安装识矩功能的批量升级，以及主技能的多目录升级。

use super::*;
use crate::app::state::ToastKind;

#[derive(Debug, Clone, Default)]
pub(crate) struct FunctionUpgradeReport {
    pub(crate) succeeded: usize,
    pub(crate) failed: Vec<u64>,
    pub(crate) cache_pending: Vec<u64>,
}

pub(super) fn start_functions(app: &mut App) -> Task<Message> {
    if app.expert_hub.skill_operations_busy() {
        return Task::none();
    }
    let Some((account_id, credential)) = active_shiju_account(app) else {
        return app.show_localized_toast(ToastKind::Warning, "登录识矩账户后管理技能", vec![]);
    };
    let LoadState::Loaded(skills) = &app.expert_hub.installed else {
        return Task::none();
    };
    let mut skill_ids = Vec::new();
    for skill in skills.iter().filter(|skill| skill.upgrade_available) {
        if !skill_ids.contains(&skill.skill_id) {
            skill_ids.push(skill.skill_id);
        }
    }
    let cache_retry = app.expert_hub.function_cache_retry.clone();
    if skill_ids.is_empty() && cache_retry.is_empty() {
        return Task::none();
    }
    app.expert_hub.action_generation = app.expert_hub.action_generation.wrapping_add(1);
    let generation = app.expert_hub.action_generation;
    app.expert_hub.function_upgrade = Some(generation);
    Task::perform(
        async move {
            let credential = credential
                .gateway_credential()
                .map_err(|_| SkillHubError::from("识矩账户凭据无效"))?;
            // 在更改服务端前确认主技能存在，避免无法同步的半完成操作。
            let installation = tokio::task::spawn_blocking(shiju_installation)
                .await
                .map_err(|_| SkillHubError::from("主技能安装状态检查异常终止"))?
                .map_err(SkillHubError::from)?
                .ok_or_else(|| SkillHubError::from("请先安装主技能后升级功能"))?;
            let mut report = FunctionUpgradeReport::default();
            for skill_id in cache_retry.iter().filter(|id| !skill_ids.contains(id)) {
                if sync_business_skill_cache(&installation.installed_dir, *skill_id).await.is_err()
                {
                    report.cache_pending.push(*skill_id);
                } else {
                    report.succeeded += 1;
                }
            }
            // 顺序执行，避免多个 CLI 同时重建同一索引。
            for skill_id in skill_ids {
                let remote = async {
                    let detail = gateway_shiju_skill_detail_async(&credential, skill_id).await?;
                    if detail.id != skill_id || detail.current_version_id == 0 {
                        return Err(SkillHubError::from("技能升级详情或版本无效"));
                    }
                    gateway_shiju_skill_remote_install_version_async(
                        &credential,
                        skill_id,
                        detail.current_version_id,
                    )
                    .await
                    .map_err(SkillHubError::from)
                }
                .await;
                if remote.is_err() {
                    report.failed.push(skill_id);
                    // 上次待同步的功能即使本次远端失败，仍保留重试入口。
                    if cache_retry.contains(&skill_id) {
                        report.cache_pending.push(skill_id);
                    }
                    continue;
                }
                if sync_business_skill_cache(&installation.installed_dir, skill_id).await.is_err() {
                    report.cache_pending.push(skill_id);
                } else {
                    report.succeeded += 1;
                }
            }
            Ok(report)
        },
        move |result| {
            Message::SkillHub(SkillHubMessage::AllFunctionsUpgraded {
                generation,
                account_id,
                result,
            })
        },
    )
}

pub(super) fn finish_functions(
    app: &mut App,
    generation: u64,
    account_id: &str,
    result: Result<FunctionUpgradeReport, SkillHubError>,
) -> Task<Message> {
    if app.expert_hub.function_upgrade != Some(generation)
        || app.expert_hub.account_id.as_deref() != Some(account_id)
    {
        return Task::none();
    }
    app.expert_hub.function_upgrade = None;
    match result {
        Ok(report) => {
            app.expert_hub.function_cache_retry = report.cache_pending;
            let pending = app.expert_hub.function_cache_retry.len();
            let kind = if report.failed.is_empty() && pending == 0 {
                ToastKind::Success
            } else {
                ToastKind::Warning
            };
            Task::batch([
                app.show_localized_toast(
                    kind,
                    "功能升级完成：成功 %s 项，失败 %s 项，缓存待同步 %s 项",
                    vec![
                        report.succeeded.to_string().into(),
                        report.failed.len().to_string().into(),
                        pending.to_string().into(),
                    ],
                ),
                load_installed_skills(app, true),
            ])
        }
        Err(error) => show_skill_error(app, ToastKind::Error, Some("功能批量升级失败：%s"), error),
    }
}

pub(super) fn main_selections(app: &App) -> Vec<SkillInstallSelection> {
    let native = SkillInstallSelection {
        target: SkillInstallTarget::VibeWindow,
        scope: SkillInstallScope::Global,
    };
    let mut selections = vec![native];
    for selection in app.expert_hub.install_guide.selected_install_selections() {
        if !selections.contains(&selection) {
            selections.push(selection);
        }
    }
    selections
}

pub(super) fn start_main(app: &mut App) -> Task<Message> {
    if app.expert_hub.skill_operations_busy() {
        return Task::none();
    }
    let Some((account_id, credential)) = active_shiju_account(app) else {
        return app.show_localized_toast(ToastKind::Warning, "登录识矩账户后管理技能", vec![]);
    };
    let Some(package_id) = ShijuPackageId::current() else {
        return app.show_localized_toast(
            ToastKind::Error,
            "当前系统或架构暂不支持直接安装",
            vec![],
        );
    };
    let selections = main_selections(app);
    let project_root = match selected_project_root(app, &selections) {
        Ok(project) => project,
        Err(error) => return show_skill_error(app, ToastKind::Error, None, error),
    };
    app.expert_hub.action_generation = app.expert_hub.action_generation.wrapping_add(1);
    let generation = app.expert_hub.action_generation;
    app.expert_hub.main_upgrade = Some(generation);
    let request_selections = selections.clone();
    let request_project = project_root.clone();
    Task::perform(
        async move {
            let home = home_directory()?;
            resolve_skill_destinations(&home, request_project.as_deref(), &request_selections)
                .map_err(SkillHubError::from)?;
            SkillHubClient::new(credential)?.latest_shiju_package(package_id).await
        },
        move |result| {
            Message::SkillHub(SkillHubMessage::MainUpgradePackageLoaded {
                generation,
                account_id,
                selections,
                project_root,
                result,
            })
        },
    )
}

fn main_response_is_current(app: &App, generation: u64, account_id: &str) -> bool {
    app.expert_hub.main_upgrade == Some(generation)
        && app.expert_hub.account_id.as_deref() == Some(account_id)
        && app.account.active_account().is_some_and(|account| account.id() == account_id)
}

pub(super) fn finish_main_package(
    app: &mut App,
    generation: u64,
    account_id: &str,
    selections: Vec<SkillInstallSelection>,
    project_root: Option<PathBuf>,
    result: Result<ShijuPackageRelease, SkillHubError>,
) -> Task<Message> {
    if !main_response_is_current(app, generation, account_id) {
        return Task::none();
    }
    let release = match result {
        Ok(release) => release,
        Err(error) => return finish_main(app, generation, account_id, None, Err(error)),
    };
    let account_id = account_id.to_string();
    let result_project = project_root.clone();
    Task::perform(
        async move {
            let report =
                install_shiju_package_for_targets(release, project_root, selections.clone())
                    .await?;
            if !install_report_matches_selections(&report, &selections) {
                return Err(SkillHubError::from("识矩技能安装结果不完整"));
            }
            Ok(report)
        },
        move |result| {
            Message::SkillHub(SkillHubMessage::MainSkillUpgraded {
                generation,
                account_id,
                project_root: result_project,
                result,
            })
        },
    )
}

pub(super) fn finish_main(
    app: &mut App,
    generation: u64,
    account_id: &str,
    project_root: Option<PathBuf>,
    result: Result<ShijuMultiInstallReport, SkillHubError>,
) -> Task<Message> {
    if !main_response_is_current(app, generation, account_id) {
        return Task::none();
    }
    let report = match result {
        Ok(report) => report,
        Err(error) => {
            app.expert_hub.main_upgrade = None;
            return show_skill_error(app, ToastKind::Error, Some("主技能升级失败：%s"), error);
        }
    };
    let selections = report
        .installs
        .iter()
        .map(|install| SkillInstallSelection {
            target: install.destination.target,
            scope: install.destination.scope,
        })
        .collect::<Vec<_>>();
    app.expert_hub.install_guide.apply_install_report(report);
    reload_local_shiju_package_version(app);
    let credential =
        app.account.active_account().expect("current account checked").skill_hub_credential();
    let account_id = account_id.to_string();
    Task::perform(
        async move {
            let home = home_directory()?;
            tokio::task::spawn_blocking(move || {
                sync_shiju_key_for_targets(&home, project_root.as_deref(), &selections, &credential)
                    .map_err(SkillHubError::from)
            })
            .await
            .map_err(|_| SkillHubError::from("识矩登录信息同步任务异常终止"))?
        },
        move |result| {
            Message::SkillHub(SkillHubMessage::MainUpgradeKeySynced {
                generation,
                account_id,
                result,
            })
        },
    )
}

pub(super) fn finish_main_key_sync(
    app: &mut App,
    generation: u64,
    account_id: &str,
    result: Result<ShijuKeySyncReport, SkillHubError>,
) -> Task<Message> {
    if !main_response_is_current(app, generation, account_id) {
        return Task::none();
    }
    app.expert_hub.main_upgrade = None;
    match result {
        Ok(report)
            if report
                .targets
                .iter()
                .all(|target| target.outcome == ShijuKeySyncOutcome::Updated) =>
        {
            app.show_localized_toast(
                ToastKind::Success,
                "主技能已升级到 %s 个目录",
                vec![report.targets.len().to_string().into()],
            )
        }
        _ => app.show_localized_toast(
            ToastKind::Warning,
            "主技能已升级，登录信息同步失败，请在安装识矩技能页重试同步",
            vec![],
        ),
    }
}
