use super::*;

pub(super) fn open_skill_detail(app: &mut SkillUpdate<'_>, skill_id: u64) -> Task<Message> {
    let gateway = app.gateway;
    let Some(account) = app.active_account else {
        return app.show_localized_toast(
            ToastKind::Error,
            "登录识矩账户后查看技能详情",
            vec![],
        );
    };
    let account_id = account.id().to_string();
    let credential = account.skill_hub_credential();
    if app.expert_hub.account_id.as_deref() != Some(account_id.as_str()) {
        app.clear_expert_hub_account(Some(account_id.clone()));
    }
    app.expert_hub.cancel_version_action();
    let generation = app.expert_hub.next_detail_generation();
    app.expert_hub.detail = Some(SkillDetailState {
        skill_id,
        generation,
        detail: LoadState::Loading { generation },
        readme: LoadState::Loading { generation },
        readme_mode: PublicDetailMode::Visual,
        readme_items: Vec::new(),
        versions: LoadState::Loading { generation },
    });
    Task::perform(
        async move {
            let credential = credential
                .gateway_credential()
                .map_err(|error| SkillHubError::from(error.to_string()))?;
            let dto = gateway_shiju_skill_detail_async(gateway, &credential, skill_id)
                .await
                .map_err(SkillHubError::from)?;
            SkillDetail::try_from(dto)
        },
        move |result| {
            Message::SkillHub(SkillHubMessage::SkillDetailLoaded {
                generation,
                account_id,
                skill_id,
                result,
            })
        },
    )
}

pub(super) fn load_skill_readme(
    gateway: crate::GatewayFactory,
    credential: desktop_account::SecretSkey,
    generation: u64,
    account_id: String,
    requested_skill_id: u64,
    canonical_skill_id: u64,
) -> Task<Message> {
    Task::perform(
        async move {
            let credential = credential
                .gateway_credential()
                .map_err(|error| SkillHubError::from(error.to_string()))?;
            gateway_shiju_skill_readme_async(gateway, &credential, canonical_skill_id)
                .await
                .map(|readme| readme.markdown.unwrap_or_default())
                .map_err(SkillHubError::from)
        },
        move |result| {
            Message::SkillHub(SkillHubMessage::SkillReadmeLoaded {
                generation,
                account_id,
                skill_id: requested_skill_id,
                result,
            })
        },
    )
}

pub(super) fn load_skill_versions(
    gateway: crate::GatewayFactory,
    credential: desktop_account::SecretSkey,
    generation: u64,
    account_id: String,
    requested_skill_id: u64,
    canonical_skill_id: u64,
) -> Task<Message> {
    Task::perform(
        async move {
            let credential = credential
                .gateway_credential()
                .map_err(|error| SkillHubError::from(error.to_string()))?;
            gateway_shiju_skill_versions_async(gateway, &credential, canonical_skill_id)
                .await
                .map(|versions| versions.into_iter().map(SkillVersion::from).collect())
                .map_err(SkillHubError::from)
        },
        move |result| {
            Message::SkillHub(SkillHubMessage::SkillVersionsLoaded {
                generation,
                account_id,
                skill_id: requested_skill_id,
                result,
            })
        },
    )
}

pub(super) fn detail_response_is_current(app: &SkillUpdate<'_>, generation: u64, account_id: &str, skill_id: u64) -> bool {
    app.expert_hub.account_id.as_deref() == Some(account_id)
        && app
            .expert_hub
            .detail
            .as_ref()
            .is_some_and(|detail| detail.generation == generation && detail.skill_id == skill_id)
}

pub(super) fn start_skill_action(app: &mut SkillUpdate<'_>, skill_id: u64, action: SkillActionKind) -> Task<Message> {
    let gateway = app.gateway;
    let Some(account) = app.active_account else {
        return app.show_localized_toast(
            ToastKind::Error,
            "登录识矩账户后管理技能",
            vec![],
        );
    };
    let account_id = account.id().to_string();
    if app.expert_hub.account_id.as_deref() != Some(account_id.as_str()) {
        app.clear_expert_hub_account(Some(account_id.clone()));
    }
    let credential = account.skill_hub_credential();
    let Some(generation) = app.expert_hub.start_action(skill_id, action.clone()) else {
        return Task::none();
    };
    let request_action = action.clone();
    Task::perform(
        async move {
            let credential = credential.gateway_credential().map_err(|error| error.to_string())?;
            match request_action {
                SkillActionKind::Install => {
                    gateway_shiju_skill_remote_install_async(gateway, &credential, skill_id)
                        .await
                        .map_err(SkillHubError::from)
                }
                SkillActionKind::Uninstall => {
                    gateway_shiju_skill_remote_uninstall_async(gateway, &credential, skill_id)
                        .await
                        .map_err(SkillHubError::from)
                }
                SkillActionKind::ResolveUpgrade { .. } | SkillActionKind::Version(_) => {
                    Err(SkillHubError::localized("无效的技能操作", vec![]))
                }
            }
        },
        move |result| {
            Message::SkillHub(SkillHubMessage::SkillActionFinished {
                generation,
                account_id,
                skill_id,
                action,
                result: result.map_err(SkillHubError::from),
            })
        },
    )
}

pub(super) fn start_quick_upgrade(app: &mut SkillUpdate<'_>, skill_id: u64) -> Task<Message> {
    let gateway = app.gateway;
    let target_version = match &app.expert_hub.installed {
        LoadState::Loaded(items) => items
            .iter()
            .find(|item| item.skill_id == skill_id && item.upgrade_available)
            .map(|item| item.current_version.clone()),
        _ => None,
    };
    let Some(target_version) = target_version else {
        return Task::none();
    };
    let Some(account) = app.active_account else {
        return app.show_localized_toast(
            ToastKind::Error,
            "登录识矩账户后管理技能",
            vec![],
        );
    };
    let account_id = account.id().to_string();
    if app.expert_hub.account_id.as_deref() != Some(account_id.as_str()) {
        app.clear_expert_hub_account(Some(account_id.clone()));
        return Task::none();
    }
    let credential = account.skill_hub_credential();
    let Some(generation) =
        app.expert_hub.start_action(skill_id, SkillActionKind::ResolveUpgrade { target_version })
    else {
        return Task::none();
    };
    Task::perform(
        async move {
            let credential = credential.gateway_credential().map_err(|error| error.to_string())?;
            let dto = gateway_shiju_skill_detail_async(gateway, &credential, skill_id).await?;
            SkillDetail::try_from(dto)
        },
        move |result| {
            Message::SkillHub(SkillHubMessage::SkillUpgradeResolved {
                generation,
                account_id,
                skill_id,
                result: result.map_err(SkillHubError::from),
            })
        },
    )
}

pub(super) fn finish_quick_upgrade_resolution(
    app: &mut SkillUpdate<'_>,
    generation: u64,
    account_id: &str,
    skill_id: u64,
    result: Result<SkillDetail, SkillHubError>,
) -> Task<Message> {
    let target_version = app.expert_hub.action.as_ref().and_then(|active| {
        if active.generation != generation || active.skill_id != skill_id {
            return None;
        }
        match &active.kind {
            SkillActionKind::ResolveUpgrade { target_version } => Some(target_version.clone()),
            _ => None,
        }
    });
    if app.expert_hub.account_id.as_deref() != Some(account_id) || target_version.is_none() {
        return Task::none();
    }
    let detail = match result {
        Ok(detail) if detail.id == skill_id => detail,
        Ok(_) => {
            app.expert_hub.action = None;
            return app.show_localized_toast(
                ToastKind::Error,
                "技能升级失败：详情响应与目标技能不匹配",
                vec![],
            );
        }
        Err(error) => {
            app.expert_hub.action = None;
            return show_skill_error(
                app,
                ToastKind::Error,
                Some("技能升级失败：%s"),
                error.into(),
            );
        }
    };
    let action = SkillVersionAction {
        skill_id,
        version_id: detail.current_version_id,
        target_version: target_version.expect("checked above"),
        direction: SkillVersionDirection::Upgrade,
    };
    let Some(active) = app.expert_hub.action.as_mut() else {
        return Task::none();
    };
    active.kind = SkillActionKind::Version(action.clone());
    let Some(account) = app.active_account else {
        app.expert_hub.action = None;
        return app.show_localized_toast(
            ToastKind::Error,
            "登录识矩账户后管理技能",
            vec![],
        );
    };
    dispatch_version_action(app.gateway,
        account.skill_hub_credential(),
        generation,
        account_id.to_string(),
        action,
    )
}

pub(super) fn start_version_action(app: &mut SkillUpdate<'_>, action: SkillVersionAction) -> Task<Message> {
    let Some(account) = app.active_account else {
        return app.show_localized_toast(
            ToastKind::Error,
            "登录识矩账户后管理技能",
            vec![],
        );
    };
    let account_id = account.id().to_string();
    if app.expert_hub.account_id.as_deref() != Some(account_id.as_str()) {
        app.clear_expert_hub_account(Some(account_id.clone()));
    }
    let credential = account.skill_hub_credential();
    let direction = action.direction;
    let Some(generation) = app.expert_hub.request_version_action(action.clone()) else {
        return Task::none();
    };
    if direction == SkillVersionDirection::Rollback {
        return Task::none();
    }
    dispatch_version_action(app.gateway, credential, generation, account_id, action)
}

pub(super) fn confirm_version_action(app: &mut SkillUpdate<'_>) -> Task<Message> {
    let Some(action) = app.expert_hub.version_confirmation.clone() else {
        return Task::none();
    };
    let Some(account) = app.active_account else {
        return app.show_localized_toast(
            ToastKind::Error,
            "登录识矩账户后管理技能",
            vec![],
        );
    };
    let account_id = account.id().to_string();
    if app.expert_hub.account_id.as_deref() != Some(account_id.as_str()) {
        app.clear_expert_hub_account(Some(account_id));
        return Task::none();
    }
    let credential = account.skill_hub_credential();
    let Some(generation) = app.expert_hub.confirm_version_action() else {
        return Task::none();
    };
    dispatch_version_action(app.gateway, credential, generation, account_id, action)
}

pub(super) fn dispatch_version_action(
    gateway: crate::GatewayFactory,
    credential: desktop_account::SecretSkey,
    generation: u64,
    account_id: String,
    action: SkillVersionAction,
) -> Task<Message> {
    let skill_id = action.skill_id;
    let request_action = action.clone();
    Task::perform(
        async move {
            let credential = credential.gateway_credential().map_err(|error| error.to_string())?;
            gateway_shiju_skill_remote_install_version_async(gateway,
                &credential,
                request_action.skill_id,
                request_action.version_id,
            )
            .await
        },
        move |result| {
            Message::SkillHub(SkillHubMessage::SkillActionFinished {
                generation,
                account_id,
                skill_id,
                action: SkillActionKind::Version(action),
                result: result.map_err(SkillHubError::from),
            })
        },
    )
}

pub(super) fn finish_skill_action(
    app: &mut SkillUpdate<'_>,
    generation: u64,
    account_id: &str,
    skill_id: u64,
    action: SkillActionKind,
    result: Result<(), SkillHubError>,
) -> Task<Message> {
    let is_current = app.expert_hub.account_id.as_deref() == Some(account_id)
        && app.expert_hub.action.as_ref().is_some_and(|active| {
            active.generation == generation && active.skill_id == skill_id && active.kind == action
        });
    if !is_current {
        return Task::none();
    }
    app.expert_hub.action = None;
    match result {
        Ok(()) if matches!(action, SkillActionKind::Install | SkillActionKind::Version(_)) => {
            Task::batch([
                app.show_localized_toast(
                    ToastKind::Info,
                    "服务端已更新，正在同步本地缓存",
                    vec![],
                ),
                start_skill_cache_sync(
                    app,
                    skill_id,
                    matches!(action, SkillActionKind::Version(_)),
                ),
            ])
        }
        Ok(()) => Task::batch([
            app.show_localized_toast(ToastKind::Success, "技能卸载成功", vec![]),
            load_current_tab(app),
        ]),
        Err(error) => {
            let label = match &action {
                SkillActionKind::Install => "技能安装失败：%s",
                SkillActionKind::Uninstall => "技能卸载失败：%s",
                SkillActionKind::ResolveUpgrade { .. } => "技能升级失败：%s",
                SkillActionKind::Version(version) => match version.direction {
                    SkillVersionDirection::Upgrade => "技能升级失败：%s",
                    SkillVersionDirection::Rollback => "技能回退失败：%s",
                    SkillVersionDirection::Switch => "技能版本切换失败：%s",
                },
            };
            show_skill_error(app, ToastKind::Error, Some(label), error)
        }
    }
}

pub(super) fn start_skill_cache_sync(app: &mut SkillUpdate<'_>, skill_id: u64, refresh_detail: bool) -> Task<Message> {
    let Some(account_id) = app.active_account.map(|account| account.id().to_string())
    else {
        return app.show_localized_toast(
            ToastKind::Warning,
            "登录识矩账户后重试本地缓存同步",
            vec![],
        );
    };
    if app.expert_hub.account_id.as_deref() != Some(account_id.as_str()) {
        app.clear_expert_hub_account(Some(account_id));
        return Task::none();
    }
    let Some(generation) = app.expert_hub.start_cache_sync(skill_id, refresh_detail) else {
        return Task::none();
    };
    Task::perform(
        async move {
            let installation = tokio::task::spawn_blocking(shiju_installation)
                .await
                .map_err(|_| {
                    desktop_skill_hub::skill_hub::SkillCacheSyncError::new(
                        "主识矩技能安装状态检查异常终止",
                    )
                })?
                .map_err(|_| {
                    desktop_skill_hub::skill_hub::SkillCacheSyncError::new("无法验证主识矩技能安装状态")
                })?
                .ok_or_else(|| {
                    desktop_skill_hub::skill_hub::SkillCacheSyncError::new(
                        "未安装主识矩技能，请前往识矩技能页安装后重试",
                    )
                })?;
            sync_business_skill_cache(&installation.installed_dir, skill_id).await
        },
        move |result| {
            Message::SkillHub(SkillHubMessage::SkillCacheSyncFinished {
                generation,
                account_id,
                skill_id,
                result,
            })
        },
    )
}

pub(super) fn retry_skill_cache_sync(app: &mut SkillUpdate<'_>) -> Task<Message> {
    let Some((skill_id, refresh_detail)) = app.expert_hub.cache_sync.pending_retry_target() else {
        return Task::none();
    };
    Task::batch([
        app.show_localized_toast(
            ToastKind::Info,
            "正在重试本地缓存同步",
            vec![],
        ),
        start_skill_cache_sync(app, skill_id, refresh_detail),
    ])
}

pub(super) fn finish_skill_cache_sync(
    app: &mut SkillUpdate<'_>,
    generation: u64,
    account_id: &str,
    skill_id: u64,
    result: Result<SkillCacheSyncReport, desktop_skill_hub::skill_hub::SkillCacheSyncError>,
) -> Task<Message> {
    let refresh_detail = app.expert_hub.cache_sync.syncing_refreshes_detail(generation, skill_id);
    if app.expert_hub.account_id.as_deref() != Some(account_id)
        || refresh_detail.is_none()
        || !app.expert_hub.finish_cache_sync(generation, skill_id, result.is_ok())
    {
        return Task::none();
    }
    match result {
        Ok(_) => Task::batch([
            app.show_localized_toast(
                ToastKind::Success,
                "技能与本地缓存同步成功",
                vec![],
            ),
            refresh_skill_cache_views(app, skill_id, refresh_detail.unwrap_or(false)),
        ]),
        Err(error) => {
            let message = desktop_i18n::format::Message::nested(
                "服务端已更新，本地缓存待同步：%s，请重试".into(),
                vec![error.message()],
            );
            app.feedback(ToastKind::Warning, message)
        }
    }
}

pub(super) fn refresh_skill_cache_views(app: &mut SkillUpdate<'_>, skill_id: u64, refresh_detail: bool) -> Task<Message> {
    if refresh_detail {
        return refresh_version_action_views(app, skill_id);
    }
    Task::batch([load_marketplace_skills(app), load_installed_skills(app, true)])
}

pub(super) fn refresh_version_action_views(app: &mut SkillUpdate<'_>, skill_id: u64) -> Task<Message> {
    let detail_task =
        if app.expert_hub.detail.as_ref().is_some_and(|detail| detail.skill_id == skill_id) {
            open_skill_detail(app, skill_id)
        } else {
            Task::none()
        };
    Task::batch([load_marketplace_skills(app), load_installed_skills(app, true), detail_task])
}

/// 下载技能版本到临时文件，并将完成结果映射回技能消息。
pub fn dispatch_download_with(
    state: &desktop_skill_hub::skill_hub::ExpertHubState,
    version_id: u64,
    opener: impl FnOnce(&str) -> Result<(), SkillHubError>,
) -> Result<(), SkillHubError> {
    let version = state
        .detail
        .as_ref()
        .and_then(|detail| match &detail.versions {
            LoadState::Loaded(versions) => versions.iter().find(|version| version.id == version_id),
            _ => None,
        })
        .ok_or_else(|| SkillHubError::localized("技能版本下载地址不可用", vec![]))?;
    let download_url = version
        .download_url
        .as_ref()
        .ok_or_else(|| SkillHubError::localized("技能版本下载地址不可用", vec![]))?;
    opener(download_url.as_str())
        .map_err(|_| SkillHubError::localized("无法打开技能下载地址", vec![]))
}
