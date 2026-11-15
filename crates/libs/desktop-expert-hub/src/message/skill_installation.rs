use super::*;

pub(super) fn start_direct_install(app: &mut SkillUpdate<'_>) -> Task<Message> {
    let Some(package_id) = app.expert_hub.install_guide.current_package else {
        return app.show_localized_toast(
            ToastKind::Error,
            "当前系统或架构暂不支持直接安装",
            vec![],
        );
    };
    let Some((account_id, credential)) = active_shiju_account(app) else {
        return app.show_localized_toast(
            ToastKind::Warning,
            "登录识矩账户后直接安装",
            vec![],
        );
    };
    let selections = app.expert_hub.install_guide.selected_install_selections();
    let project_root = match selected_project_root(app.expert_hub, &selections) {
        Ok(project_root) => project_root,
        Err(error) => {
            return show_skill_error(app, ToastKind::Error, None, error.into());
        }
    };
    let home = match home_directory() {
        Ok(home) => home,
        Err(error) => {
            return show_skill_error(app, ToastKind::Error, None, error.into());
        }
    };
    if let Err(error) = resolve_skill_destinations(&home, project_root.as_deref(), &selections) {
        return show_skill_error(app, ToastKind::Error, None, error.into());
    }
    let Some(generation) = app.expert_hub.install_guide.start_direct_install(package_id) else {
        return Task::none();
    };
    let result_selections = selections.clone();
    Task::perform(
        async move {
            let client = SkillHubClient::new(credential)?;
            client.latest_shiju_package(package_id).await
        },
        move |result| {
            Message::SkillHub(SkillHubMessage::DirectPackageLoaded {
                generation,
                account_id,
                package_id,
                selections: result_selections,
                project_root,
                result,
            })
        },
    )
}

pub(super) fn finish_direct_package_load(
    app: &mut SkillUpdate<'_>,
    generation: u64,
    account_id: &str,
    package_id: ShijuPackageId,
    selections: Vec<SkillInstallSelection>,
    project_root: Option<PathBuf>,
    result: Result<ShijuPackageRelease, SkillHubError>,
) -> Task<Message> {
    let is_current = app.expert_hub.install_guide.open
        && app.expert_hub.account_id.as_deref() == Some(account_id)
        && selected_project_root(app.expert_hub, &selections).is_ok_and(|current| current == project_root)
        && app.expert_hub.install_guide.direct_selections == selections
        && matches!(
            app.expert_hub.install_guide.direct_install,
            ShijuDirectInstallState::Fetching {
                generation: active_generation,
                package_id: active_package,
            } if active_generation == generation && active_package == package_id
        );
    if !is_current {
        return Task::none();
    }
    let release = match result {
        Ok(release) => release,
        Err(error) => {
            app.expert_hub.install_guide.direct_install = ShijuDirectInstallState::Idle;
            return show_skill_error(
                app,
                ToastKind::Error,
                Some("直接安装准备失败：%s"),
                error.into(),
            );
        }
    };
    app.expert_hub.install_guide.direct_install =
        ShijuDirectInstallState::Installing { generation, package_id };
    let account_id = account_id.to_string();
    let result_selections = selections.clone();
    Task::perform(
        install_shiju_package_for_targets(release, project_root, selections),
        move |result| {
            Message::SkillHub(SkillHubMessage::ShijuInstallFinished {
                generation,
                account_id,
                package_id,
                selections: result_selections,
                result,
            })
        },
    )
}

pub(super) fn finish_direct_install(
    app: &mut SkillUpdate<'_>,
    generation: u64,
    account_id: &str,
    package_id: ShijuPackageId,
    selections: &[SkillInstallSelection],
    result: Result<ShijuMultiInstallReport, SkillHubError>,
) -> Task<Message> {
    let is_current = app.expert_hub.install_guide.open
        && app.expert_hub.account_id.as_deref() == Some(account_id)
        && app.expert_hub.install_guide.direct_selections == selections
        && matches!(
            app.expert_hub.install_guide.direct_install,
            ShijuDirectInstallState::Installing {
                generation: active_generation,
                package_id: active_package,
            } if active_generation == generation && active_package == package_id
        );
    if !is_current {
        return Task::none();
    }
    match result {
        Ok(report) if install_report_matches_selections(&report, selections) => {
            let installed_count = report.installs.len();
            app.expert_hub.install_guide.apply_install_report(report);
            reload_local_shiju_package_version(app);
            app.expert_hub.install_guide.direct_install = ShijuDirectInstallState::Installed;
            Task::batch([
                app.show_localized_toast(
                    ToastKind::Success,
                    "识矩技能已安装到 %s 个目标",
                    vec![installed_count.to_string().into()],
                ),
                start_shiju_key_sync(app, false),
            ])
        }
        Ok(_) => {
            app.expert_hub.install_guide.direct_install = ShijuDirectInstallState::Idle;
            app.show_localized_toast(
                ToastKind::Error,
                "识矩技能安装结果不完整",
                vec![],
            )
        }
        Err(error) => {
            app.expert_hub.install_guide.direct_install = ShijuDirectInstallState::Idle;
            show_skill_error(
                app,
                ToastKind::Error,
                Some("识矩技能安装失败：%s"),
                error.into(),
            )
        }
    }
}

pub async fn install_shiju_package_for_targets(
    release: ShijuPackageRelease,
    project_root: Option<PathBuf>,
    selections: Vec<SkillInstallSelection>,
) -> Result<ShijuMultiInstallReport, SkillHubError> {
    let client = reqwest::Client::builder()
        .redirect(Policy::none())
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|_| SkillHubError::localized("无法创建识矩技能下载客户端", vec![]))?;
    let mut response = client
        .get(release.download_url.as_str())
        .send()
        .await
        .map_err(|_| SkillHubError::localized("识矩技能安装包下载失败", vec![]))?;
    if !response.status().is_success() {
        return Err(SkillHubError::localized(
            "识矩技能安装包下载失败: HTTP %s",
            vec![response.status().as_u16().to_string()],
        ));
    }
    if response.content_length().is_some_and(|length| length > MAX_SHIJU_ARCHIVE_BYTES) {
        return Err(SkillHubError::localized("识矩技能安装包超过 100 MiB 限制", vec![]));
    }

    let home = home_directory()?;
    let (chunk_sender, chunk_receiver) = tokio::sync::mpsc::channel(4);
    let install_task = tokio::task::spawn_blocking(move || {
        let archive = NamedTempFile::new()
            .map_err(|_| SkillHubError::localized("无法创建识矩技能临时下载文件", vec![]))?;
        let archive = write_shiju_download_chunks(archive, chunk_receiver)?;
        install_shiju_archive_for_targets(
            archive.path(),
            &home,
            project_root.as_deref(),
            &selections,
        )
        .map_err(SkillHubError::from)
    });

    let mut written = 0u64;
    let mut download_error = None;
    loop {
        let chunk = match response.chunk().await {
            Ok(Some(chunk)) => chunk,
            Ok(None) => break,
            Err(_) => {
                download_error = Some(SkillHubError::localized("识矩技能安装包下载中断", vec![]));
                break;
            }
        };
        let chunk_len = match u64::try_from(chunk.len())
            .map_err(|_| SkillHubError::localized("识矩技能安装包大小溢出", vec![]))
        {
            Ok(chunk_len) => chunk_len,
            Err(error) => {
                download_error = Some(error);
                break;
            }
        };
        written = match written.checked_add(chunk_len) {
            Some(written) => written,
            None => {
                download_error = Some(SkillHubError::localized("识矩技能安装包大小溢出", vec![]));
                break;
            }
        };
        if written > MAX_SHIJU_ARCHIVE_BYTES {
            download_error =
                Some(SkillHubError::localized("识矩技能安装包超过 100 MiB 限制", vec![]));
            break;
        }
        if chunk_sender.send(ShijuDownloadChunk::Data(chunk.to_vec())).await.is_err() {
            download_error = Some(SkillHubError::localized("识矩技能安装写盘任务异常终止", vec![]));
            break;
        }
    }

    let completion = match download_error {
        Some(error) => ShijuDownloadChunk::Failed(error),
        None => ShijuDownloadChunk::Complete,
    };
    let _ = chunk_sender.send(completion).await;
    drop(chunk_sender);
    install_task.await.map_err(|_| SkillHubError::localized("识矩技能安装任务异常终止", vec![]))?
}

/// 顺序写入下载数据块；超过上限或读取失败时停止并返回错误。

pub(super) fn start_manual_shiju_key_sync(app: &mut SkillUpdate<'_>) -> Task<Message> {
    if app.expert_hub.install_guide.installed_selection_count() == 0 {
        return app.show_localized_toast(
            ToastKind::Error,
            "识矩技能尚未安装，无法同步登录信息",
            vec![],
        );
    }
    if !matches!(
        &app.expert_hub.install_guide.direct_install,
        ShijuDirectInstallState::Idle | ShijuDirectInstallState::Installed
    ) {
        return app.show_localized_toast(
            ToastKind::Warning,
            "请等待识矩技能安装或卸载完成后再同步",
            vec![],
        );
    }
    start_shiju_key_sync(app, true)
}

pub(super) fn start_shiju_key_sync(app: &mut SkillUpdate<'_>, requested_by_user: bool) -> Task<Message> {
    let account = app.active_account
        .map(|account| (account.id().to_string(), account.skill_hub_credential()));
    let Some((account_id, credential)) = account else {
        return if requested_by_user {
            app.show_localized_toast(
                ToastKind::Warning,
                "登录识矩账户后同步登录信息",
                vec![],
            )
        } else {
            Task::none()
        };
    };
    let selections = app.expert_hub.install_guide.selected_install_selections();
    let project_root = match selected_project_root(app.expert_hub, &selections) {
        Ok(project_root) => project_root,
        Err(error) => {
            return show_skill_error(app, ToastKind::Error, None, error.into());
        }
    };
    let home = match home_directory() {
        Ok(home) => home,
        Err(error) => {
            return show_skill_error(app, ToastKind::Error, None, error.into());
        }
    };
    let Some(operation) = app.expert_hub.install_guide.start_login_sync() else {
        return Task::none();
    };
    let result_selections = operation.selections.clone();
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                sync_shiju_key_for_targets(&home, project_root.as_deref(), &selections, &credential)
                    .map_err(SkillHubError::from)
            })
            .await
            .map_err(|_| SkillHubError::localized("识矩登录信息同步任务异常终止", vec![]))?
        },
        move |result| {
            Message::SkillHub(SkillHubMessage::ShijuKeySyncFinished {
                generation: operation.generation,
                account_id,
                selections: result_selections,
                requested_by_user,
                result,
            })
        },
    )
}

pub(super) fn finish_shiju_key_sync(
    app: &mut SkillUpdate<'_>,
    generation: u64,
    account_id: &str,
    selections: &[SkillInstallSelection],
    requested_by_user: bool,
    result: Result<ShijuKeySyncReport, SkillHubError>,
) -> Task<Message> {
    let is_current = app.expert_hub.install_guide.open
        && app.active_account.is_some_and(|account| account.id() == account_id)
        && app.expert_hub.install_guide.login_sync.as_ref().is_some_and(|operation| {
            operation.generation == generation && operation.selections == selections
        });
    if !is_current {
        return Task::none();
    }
    app.expert_hub.install_guide.login_sync = None;
    app.expert_hub.install_guide.login_sync_in_progress = false;
    match result {
        Ok(report) => {
            let updated = report
                .targets
                .iter()
                .filter(|target| target.outcome == ShijuKeySyncOutcome::Updated)
                .count();
            if updated == 0 {
                return if requested_by_user {
                    app.show_localized_toast(
                        ToastKind::Error,
                        "所选目标尚未安装识矩技能，无法同步登录信息",
                        vec![],
                    )
                } else {
                    Task::none()
                };
            }
            if requested_by_user {
                app.show_localized_toast(
                    ToastKind::Success,
                    "识矩登录信息已同步到 %s 个目标",
                    vec![updated.to_string().into()],
                )
            } else {
                app.show_localized_toast(
                    ToastKind::Success,
                    "识矩登录信息已自动同步到 %s 个目标",
                    vec![updated.to_string().into()],
                )
            }
        }
        Err(_) => {
            if requested_by_user {
                app.show_localized_toast(
                    ToastKind::Error,
                    "识矩登录信息同步失败，请检查文件权限后重试",
                    vec![],
                )
            } else {
                app.show_localized_toast(
                    ToastKind::Error,
                    "识矩登录信息自动同步失败，请检查文件权限后手动重试",
                    vec![],
                )
            }
        }
    }
}

pub(super) fn start_shiju_uninstall(app: &mut SkillUpdate<'_>) -> Task<Message> {
    let Some(generation) = app.expert_hub.install_guide.start_uninstall() else {
        return Task::none();
    };
    let selections = app.expert_hub.install_guide.direct_selections.clone();
    let project_root = match selected_project_root(app.expert_hub, &selections) {
        Ok(project_root) => project_root,
        Err(error) => {
            app.expert_hub.install_guide.direct_install = ShijuDirectInstallState::Installed;
            return show_skill_error(app, ToastKind::Error, None, error.into());
        }
    };
    let home = match home_directory() {
        Ok(home) => home,
        Err(error) => {
            app.expert_hub.install_guide.direct_install = ShijuDirectInstallState::Installed;
            return show_skill_error(app, ToastKind::Error, None, error.into());
        }
    };
    let result_selections = selections.clone();
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                uninstall_shiju_for_targets(&home, project_root.as_deref(), &selections)
                    .map_err(SkillHubError::from)
            })
            .await
            .map_err(|_| SkillHubError::localized("识矩技能卸载任务异常终止", vec![]))?
        },
        move |result| {
            Message::SkillHub(SkillHubMessage::ShijuUninstallFinished {
                generation,
                selections: result_selections,
                result,
            })
        },
    )
}

pub(super) fn finish_shiju_uninstall(
    app: &mut SkillUpdate<'_>,
    generation: u64,
    selections: &[SkillInstallSelection],
    result: Result<ShijuUninstallReport, SkillHubError>,
) -> Task<Message> {
    if !app.expert_hub.install_guide.open
        || app.expert_hub.install_guide.direct_selections != selections
        || !matches!(
            app.expert_hub.install_guide.direct_install,
            ShijuDirectInstallState::Uninstalling {
                generation: active_generation,
            } if active_generation == generation
        )
    {
        return Task::none();
    }
    match result {
        Ok(report) => {
            let removed = report.targets.iter().filter(|target| target.removed).count();
            app.expert_hub.install_guide.apply_uninstall_report(report);
            reload_local_shiju_package_version(app);
            app.expert_hub.install_guide.direct_install =
                if app.expert_hub.install_guide.installed_selection_count() == 0 {
                    ShijuDirectInstallState::Idle
                } else {
                    ShijuDirectInstallState::Installed
                };
            app.show_localized_toast(
                ToastKind::Success,
                "已从 %s 个目标卸载识矩技能",
                vec![removed.to_string().into()],
            )
        }
        Err(error) => {
            app.expert_hub.install_guide.direct_install = ShijuDirectInstallState::Installed;
            show_skill_error(
                app,
                ToastKind::Error,
                Some("识矩技能卸载失败：%s"),
                error.into(),
            )
        }
    }
}
