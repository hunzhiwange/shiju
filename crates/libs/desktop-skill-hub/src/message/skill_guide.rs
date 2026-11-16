use super::*;

pub fn open_expert_hub_tab(app: &mut SkillUpdate<'_>) {
    app.effects.push(Effect::OpenExpertHub);
}
pub(super) fn open_shiju_skills_tab(app: &mut SkillUpdate<'_>) {
    app.effects.push(Effect::OpenSkills);
}

pub(super) fn open_install_guide(app: &mut SkillUpdate<'_>) -> Task<Message> {
    app.effects.push(Effect::CloseAccountMenu);
    open_shiju_skills_tab(app);
    app.expert_hub.skill_tab = SkillTab::Install;
    app.expert_hub.detail = None;
    app.expert_hub.install_guide.open = true;
    app.expert_hub.install_guide.current_package = ShijuPackageId::current();
    app.expert_hub.install_guide.selected_project_path = app
        .project_path
        .iter()
        .chain(app.recent_projects.iter())
        .map(|path| path.trim())
        .find(|path| !path.is_empty())
        .map(str::to_string);
    Task::batch([load_agent_package(app, false), start_installation_load(app)])
}

pub(super) fn start_installation_load(app: &mut SkillUpdate<'_>) -> Task<Message> {
    let generation = app.expert_hub.install_guide.start_installation_load();
    let project_root =
        app.expert_hub.install_guide.selected_project_path.as_deref().map(PathBuf::from);
    let selections = all_install_selections(project_root.is_some());
    let result_context = selections.clone();
    Task::perform(
        async move {
            let home = home_directory()?;
            tokio::task::spawn_blocking(move || {
                shiju_installations_for_targets(&home, project_root.as_deref(), &selections)
                    .map_err(SkillHubError::from)
            })
            .await
            .map_err(|_| SkillHubError::localized("识矩技能安装状态检查任务异常终止", vec![]))?
        },
        move |result| {
            Message::SkillHub(SkillHubMessage::ShijuInstallationsLoaded {
                generation,
                selections: result_context,
                result,
            })
        },
    )
}

pub(super) fn finish_installation_load(
    app: &mut SkillUpdate<'_>,
    generation: u64,
    _selections: &[SkillInstallSelection],
    result: Result<ShijuInstallationsReport, SkillHubError>,
) -> Task<Message> {
    if !app.expert_hub.install_guide.open
        || app.expert_hub.install_guide.installation_generation != generation
    {
        return Task::none();
    }
    match result {
        Ok(report) => {
            app.expert_hub.install_guide.set_installations(report);
            Task::none()
        }
        Err(error) => {
            app.expert_hub
                .install_guide
                .set_installations(ShijuInstallationsReport { targets: Vec::new() });
            show_skill_error(
                app,
                ToastKind::Error,
                Some("无法检查识矩技能安装状态：%s"),
                error.into(),
            )
        }
    }
}

pub fn selected_project_root(
    state: &crate::skill_hub::ExpertHubState,
    selections: &[SkillInstallSelection],
) -> Result<Option<PathBuf>, SkillHubError> {
    if selections.iter().any(|selection| selection.scope == SkillInstallScope::Project) {
        let project = state
            .install_guide
            .selected_project_path
            .as_deref()
            .filter(|path| !path.trim().is_empty())
            .ok_or_else(|| SkillHubError::localized("项目级技能安装需要选择项目", vec![]))?;
        return Ok(Some(PathBuf::from(project)));
    }
    Ok(None)
}

pub(super) fn select_shiju_install_scope(
    app: &mut SkillUpdate<'_>,
    scope: SkillInstallScope,
) -> Task<Message> {
    if scope == SkillInstallScope::Project
        && app.expert_hub.install_guide.selected_project_path.as_deref().is_none_or(str::is_empty)
    {
        app.expert_hub.install_guide.selected_project_path = app
            .project_path
            .iter()
            .chain(app.recent_projects.iter())
            .map(|path| path.trim())
            .find(|path| !path.is_empty())
            .map(str::to_string);
        if app.expert_hub.install_guide.selected_project_path.as_deref().is_none_or(str::is_empty) {
            return app.show_localized_toast(
                ToastKind::Error,
                "项目级技能安装需要选择项目",
                vec![],
            );
        }
    }
    if app.expert_hub.install_guide.install_scope == scope {
        return Task::none();
    }
    if app.expert_hub.install_guide.is_busy() {
        return app.show_localized_toast(
            ToastKind::Warning,
            "请等待当前安装、卸载或同步操作完成",
            vec![],
        );
    }
    app.expert_hub.install_guide.select_scope(scope);
    Task::none()
}

pub(super) fn select_shiju_install_project(
    app: &mut SkillUpdate<'_>,
    project_path: String,
) -> Task<Message> {
    if app.expert_hub.install_guide.is_busy() {
        return app.show_localized_toast(
            ToastKind::Warning,
            "请等待当前安装、卸载或同步操作完成",
            vec![],
        );
    }
    let project_path = project_path.trim();
    if project_path.is_empty()
        || !app
            .project_path
            .iter()
            .chain(app.recent_projects.iter())
            .any(|candidate| candidate.trim() == project_path)
    {
        return app.show_localized_toast(ToastKind::Error, "请选择已打开或最近使用的项目", vec![]);
    }
    if app.expert_hub.install_guide.selected_project_path.as_deref() == Some(project_path) {
        return Task::none();
    }
    app.expert_hub.install_guide.selected_project_path = Some(project_path.to_string());
    app.expert_hub
        .install_guide
        .set_installations(ShijuInstallationsReport { targets: Vec::new() });
    start_installation_load(app)
}

pub(super) fn toggle_shiju_install_target(
    app: &mut SkillUpdate<'_>,
    target: SkillInstallTarget,
) -> Task<Message> {
    if app.expert_hub.install_guide.is_busy() {
        return app.show_localized_toast(
            ToastKind::Warning,
            "请等待当前安装、卸载或同步操作完成",
            vec![],
        );
    }
    let is_selected = app.expert_hub.install_guide.selected_targets.contains(&target);
    if is_selected && app.expert_hub.install_guide.selected_targets.len() == 1 {
        return app.show_localized_toast(ToastKind::Warning, "至少选择一个技能安装目标", vec![]);
    }
    app.expert_hub.install_guide.toggle_target(target);
    Task::none()
}

pub(super) fn open_selected_shiju_install_dir(app: &mut SkillUpdate<'_>) -> Task<Message> {
    let target = app.expert_hub.install_guide.selected_targets.iter().copied().find(|target| {
        app.expert_hub
            .install_guide
            .installation(app.expert_hub.install_guide.install_scope, *target)
            .is_some()
    });
    let Some(target) = target else {
        return app.show_localized_toast(ToastKind::Warning, "所选目标尚未安装识矩技能", vec![]);
    };
    open_shiju_install_dir_for(app, target)
}

pub(super) fn open_shiju_install_dir_for(
    app: &mut SkillUpdate<'_>,
    target: SkillInstallTarget,
) -> Task<Message> {
    let Some(directory) = app
        .expert_hub
        .install_guide
        .installation(app.expert_hub.install_guide.install_scope, target)
        .map(|installation| installation.installed_dir.clone())
    else {
        return app.show_localized_toast(ToastKind::Warning, "指定目标尚未安装识矩技能", vec![]);
    };
    match open_shiju_install_dir_with(&directory, |path| {
        open::that(path).map_err(|_| SkillHubError::localized("无法打开识矩技能目录", vec![]))
    }) {
        Ok(()) => app.show_localized_toast(ToastKind::Info, "已打开识矩技能目录", vec![]),
        Err(error) => show_skill_error(app, ToastKind::Error, None, error.into()),
    }
}

pub fn active_shiju_account(
    app: &mut SkillUpdate<'_>,
) -> Option<(String, desktop_account::SecretSkey)> {
    let active = app
        .active_account
        .map(|account| (account.id().to_string(), account.skill_hub_credential()));
    let Some((account_id, credential)) = active else {
        app.expert_hub.install_guide.agent_package = LoadState::NeedsLogin;
        return None;
    };
    if app.expert_hub.account_id.as_deref() != Some(account_id.as_str()) {
        app.clear_expert_hub_account(Some(account_id.clone()));
    }
    Some((account_id, credential))
}

pub(super) fn load_agent_package(
    app: &mut SkillUpdate<'_>,
    copy_after_load: bool,
) -> Task<Message> {
    let Some(package_id) = app.expert_hub.install_guide.current_package else {
        app.expert_hub.install_guide.agent_package =
            LoadState::Error(SkillHubError::localized("当前系统或架构暂不支持自动安装", vec![]));
        return Task::none();
    };
    let Some((account_id, credential)) = active_shiju_account(app) else {
        return Task::none();
    };
    let generation = app.expert_hub.install_guide.start_agent_request();
    Task::perform(
        async move {
            let client = SkillHubClient::new(credential)?;
            client.latest_shiju_package(package_id).await
        },
        move |result| {
            Message::SkillHub(SkillHubMessage::AgentPackageLoaded {
                generation,
                account_id,
                copy_after_load,
                result,
            })
        },
    )
}

pub(super) fn finish_agent_package_load(
    app: &mut SkillUpdate<'_>,
    generation: u64,
    account_id: &str,
    copy_after_load: bool,
    result: Result<ShijuPackageRelease, SkillHubError>,
) -> Task<Message> {
    if !app.expert_hub.install_guide.open
        || app.expert_hub.account_id.as_deref() != Some(account_id)
        || app.expert_hub.install_guide.agent_generation != generation
    {
        return Task::none();
    }
    match result {
        Ok(release) => {
            app.expert_hub.install_guide.agent_package = LoadState::Loaded(release);
            let text = agent_install_text(
                desktop_i18n::I18n::new(app.desktop_locale),
                &app.expert_hub.install_guide,
            );
            if copy_after_load {
                Task::batch([
                    Task::done(Message::CopyCode(text)),
                    app.show_localized_toast(ToastKind::Success, "安装文字已复制", vec![]),
                ])
            } else {
                Task::none()
            }
        }
        Err(error) => {
            app.expert_hub.install_guide.agent_package = LoadState::Error(error);
            if copy_after_load {
                app.show_localized_toast(ToastKind::Error, "安装文字准备失败，请稍后重试", vec![])
            } else {
                Task::none()
            }
        }
    }
}

/// 返回适合复制给代理的安装命令；尚未加载包信息时返回空文本。

pub(super) fn start_manual_download(
    app: &mut SkillUpdate<'_>,
    package_id: ShijuPackageId,
) -> Task<Message> {
    let Some((account_id, credential)) = active_shiju_account(app) else {
        return app.show_localized_toast(ToastKind::Warning, "登录识矩账户后下载安装包", vec![]);
    };
    let Some(generation) = app.expert_hub.install_guide.start_manual_download(package_id) else {
        return Task::none();
    };
    Task::perform(
        async move {
            let client = SkillHubClient::new(credential)?;
            client.latest_shiju_package(package_id).await
        },
        move |result| {
            Message::SkillHub(SkillHubMessage::ManualPackageLoaded {
                generation,
                account_id,
                package_id,
                result,
            })
        },
    )
}

pub(super) fn finish_manual_download(
    app: &mut SkillUpdate<'_>,
    generation: u64,
    account_id: &str,
    package_id: ShijuPackageId,
    result: Result<ShijuPackageRelease, SkillHubError>,
) -> Task<Message> {
    let is_current = app.expert_hub.install_guide.open
        && app.expert_hub.account_id.as_deref() == Some(account_id)
        && app.expert_hub.install_guide.manual_download.is_some_and(|active| {
            active.generation == generation && active.package_id == package_id
        });
    if !is_current {
        return Task::none();
    }
    app.expert_hub.install_guide.manual_download = None;
    match result {
        Ok(release) => match dispatch_shiju_download_with(&release, |url| {
            open::that(url)
                .map_err(|_| SkillHubError::localized("无法打开识矩技能下载地址", vec![]))
        }) {
            Ok(()) => {
                app.show_localized_toast(ToastKind::Info, "已在系统浏览器中打开安装包", vec![])
            }
            Err(error) => show_skill_error(app, ToastKind::Error, None, error.into()),
        },
        Err(error) => {
            show_skill_error(app, ToastKind::Error, Some("安装包地址生成失败：%s"), error.into())
        }
    }
}
