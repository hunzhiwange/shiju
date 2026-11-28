//! Host bridge for scoped workspace effects; completions retain their original envelope.
use crate::app::{
    App, Message,
    message::{SettingsMessage, settings::AppsCatalogMessage},
};
use desktop_settings_skills::third_party::{
    self,
    workspace::{Effect, Event},
};
use iced::Task;
use vw_api_app_skills::{
    bindings_api::*, discovery_api::*, install_api::*, repos_api::*, search_api::*,
};

pub(crate) fn execute(
    app: &mut App,
    scope: third_party::Scope,
    catalog: u64,
    effect: Effect,
) -> Task<Message> {
    if let Effect::Documentation(url) = &effect {
        let safe = reqwest::Url::parse(url).ok().is_some_and(|u| {
            matches!(u.scheme(), "http" | "https")
                && u.host_str().is_some()
                && u.username().is_empty()
                && u.password().is_none()
        });
        return if safe {
            Task::done(Message::View(crate::app::message::view::ViewMessage::OpenUrlExternal(
                url.clone(),
            )))
        } else {
            app.show_message_toast(
                crate::app::state::ToastKind::Error,
                desktop_i18n::settings_extensions::feedback_message(&("文档地址无效")),
            )
        };
    }
    if let Effect::Error(error) = &effect {
        return app.show_message_toast(
            crate::app::state::ToastKind::Error,
            desktop_i18n::settings_extensions::feedback_message(&(error.clone())),
        );
    }
    if let Effect::Uninstall(id) = &effect {
        return crate::app::message::settings::skills::update(
            app,
            SettingsMessage::SkillsUpdates(
                desktop_settings_skills::updates_view::Event::OpenForUninstall(id.clone()),
            ),
        );
    }
    if let Effect::OpenDirectory(id) = &effect {
        let query = SkillReposQuery {
            project_id: vw_api_transport::local_settings_project(scope.project.as_deref()).into(),
            target_host_id: scope.host.clone(),
        };
        let token = app.apps_catalog.token.clone();
        let change =
            desktop_settings_skills::update::storage_modal::Change::OpenSkillDirectory(id.clone());
        return Task::perform(
            async move {
                desktop_session::session_gateway::gateway_skill_storage_request(
                    crate::app::config::gateway_client_endpoint(),
                    query,
                    token,
                    change,
                )
                .await
            },
            move |result| {
                message(third_party::Message {
                    scope: scope.clone(),
                    catalog,
                    action: third_party::Action::Workspace(Event::DirectoryOpened(
                        result.map(|_| ()),
                    )),
                })
            },
        );
    }
    if matches!(effect, Effect::None) {
        return Task::none();
    }
    let page = &app.skills_settings.workspace;
    let available = page.available_targets();
    let targets = page
        .bindings
        .targets
        .iter()
        .filter(|target| available.contains(target))
        .cloned()
        .collect::<Vec<_>>();
    let token = app.apps_catalog.token.clone();
    let request_scope = SkillReposQuery {
        project_id: vw_api_transport::local_settings_project(scope.project.as_deref()).into(),
        target_host_id: scope.host.clone(),
    };
    Task::perform(
        async move {
            let endpoint = crate::app::config::gateway_client_endpoint();
            // Even client construction failures must complete the matching request.
            let client = vw_gateway_client::GatewayClient::new(endpoint.clone());
            match effect {
                Effect::Load(g) => {
                    let result = async {
                        let client = client?;
                        let skills = gateway_app_skills::GatewayAppSkillsClient::new(
                            client.transport().clone(),
                        );
                        let repos = skills.skill_repos_list(&request_scope).await?;
                        let inventory = skills
                            .agent_apps_skills_inventory(&request_scope, &token)
                            .await
                            .map_err(|e| e.to_string())?;
                        if repos.scope != request_scope || inventory.scope != request_scope {
                            return Err("工作页响应目标不匹配".into());
                        }
                        Ok((repos, inventory))
                    }
                    .await;
                    Event::Loaded(g, result)
                }
                Effect::Discover(g, force_refresh) => Event::Discovered(
                    g,
                    async {
                        let client = client?;
                        gateway_app_skills::GatewayAppSkillsClient::new(client.transport().clone())
                            .agent_apps_skills_discover(
                                &DiscoverSkillsRequest {
                                    scope: request_scope,
                                    repo_id: None,
                                    force_refresh,
                                },
                                &token,
                            )
                            .await
                            .map_err(|e| e.to_string())
                    }
                    .await,
                ),
                Effect::Search(g, query, offset) => Event::Searched(
                    g,
                    async {
                        let client = client?;
                        gateway_app_skills::GatewayAppSkillsClient::new(client.transport().clone())
                            .agent_apps_market_search(
                                &MarketSearchQuery {
                                    project_id: request_scope.project_id,
                                    target_host_id: request_scope.target_host_id,
                                    query,
                                    limit: 50,
                                    offset,
                                },
                                &token,
                            )
                            .await
                            .map_err(|e| e.to_string())
                    }
                    .await,
                ),
                Effect::Install {
                    generation,
                    candidate,
                    register,
                    inventory_version,
                    repository_version,
                } => Event::Installed(
                    generation,
                    async {
                        let client = client?;
                        let skills = gateway_app_skills::GatewayAppSkillsClient::new(
                            client.transport().clone(),
                        );
                        if register {
                            let saved = skills
                                .skill_repo_save(&SaveSkillRepoRequest {
                                    scope: request_scope.clone(),
                                    expected_version: repository_version,
                                    repo: SkillRepoEntry {
                                        owner: candidate.owner.clone(),
                                        name: candidate.repo.clone(),
                                        branch: candidate.branch.clone(),
                                        enabled: true,
                                    },
                                })
                                .await?;
                            if saved.scope != request_scope {
                                return Err("仓库响应目标不匹配".into());
                            }
                        }
                        let directory = candidate.destination().to_owned();
                        desktop_session::session_gateway::gateway_skills_install_view(
                            endpoint.clone(),
                            request_scope.clone(),
                            Some(InstallSkillRequest {
                                scope: request_scope.clone(),
                                expected_version: inventory_version,
                                source: SkillInstallSource::Git {
                                    repo_id: candidate.repo_id(),
                                    selection: candidate.directory.clone(),
                                    directory: directory.clone(),
                                },
                            }),
                            token.clone(),
                        )
                        .await?;
                        let inventory = skills
                            .agent_apps_skills_inventory(&request_scope, &token)
                            .await
                            .map_err(|e| e.to_string())?;
                        let source = format!(
                            "{}/{} · {} · {}",
                            candidate.owner, candidate.repo, candidate.branch, candidate.directory
                        );
                        if inventory.scope != request_scope
                            || !inventory.skills.iter().any(|s| {
                                s.id == directory && s.present && s.source.as_ref() == Some(&source)
                            })
                        {
                            return Err("安装任务结束，但主副本尚未核对，请刷新目录".into());
                        }
                        if targets.is_empty() {
                            return Ok("主副本安装完成（未选择应用分发）".into());
                        }
                        let readback =
                            desktop_session::session_gateway::gateway_skill_bindings_request(
                                endpoint,
                                request_scope,
                                token,
                                Some(desktop_settings_skills::update::bindings_view::Change::Bind(
                                    directory.clone(),
                                    SkillBindingsRequest {
                                        scope: inventory.scope,
                                        targets: targets.clone(),
                                        enabled: true,
                                        mode: SkillBindingMode::Auto,
                                    },
                                )),
                            )
                            .await?;
                        if let Some(error) = readback.error {
                            return Err(format!("主副本已安装；应用分发需核对：{error}"));
                        }
                        if !desktop_settings_skills::update::bindings_view::complete_success(
                            &readback.results,
                            &targets,
                            &directory,
                            true,
                        ) {
                            return Err(format!(
                                "主副本已安装；应用分发部分失败：{:?}，请在应用动作中核对",
                                readback.results
                            ));
                        }
                        Ok("主副本安装及所选应用分发完成".into())
                    }
                    .await,
                ),
                Effect::None
                | Effect::Documentation(_)
                | Effect::OpenDirectory(_)
                | Effect::Uninstall(_)
                | Effect::Error(_) => unreachable!(),
            }
        },
        move |event| {
            message(third_party::Message {
                scope: scope.clone(),
                catalog,
                action: third_party::Action::Workspace(event),
            })
        },
    )
}

pub(super) fn view(app: &App) -> iced::Element<'_, Message> {
    let applications = app.apps_catalog.rows.iter().map(|row| &row.entry).collect::<Vec<_>>();
    desktop_settings_skills::third_party::view(
        desktop_i18n::I18n::new(app.general_settings.desktop_locale),
        &app.skills_settings.workspace,
        &applications,
        desktop_settings_apps::system_settings_apps::list::brand_icon,
    )
    .map(message)
}

fn message(event: third_party::Message) -> Message {
    Message::Settings(SettingsMessage::SkillsWorkspace(event))
}

pub(super) fn open(app: &mut App) -> Task<Message> {
    use crate::app::message::settings;
    use third_party::Action;
    let catalog_load = settings::apps_catalog::update(app, AppsCatalogMessage::EnsureLoaded);
    let input = settings::skills::update_input(app);
    let scope = input.context.map(|context| third_party::Scope {
        context,
        project: input.project,
        host: input.host,
        directory: input.directory,
    });
    if app.skills_settings.workspace.scope() == scope.as_ref() {
        return catalog_load;
    }
    app.apps_catalog.navigation_generation = app.apps_catalog.navigation_generation.wrapping_add(1);
    let catalog = app.apps_catalog.navigation_generation;
    let state = &mut app.skills_settings.workspace;
    state.sync(scope, catalog);
    let Some(scope) = state.scope().cloned() else {
        return catalog_load;
    };
    let scoped = |action| third_party::Message { scope: scope.clone(), catalog, action };
    let inventory =
        update(app, scoped(Action::Workspace(Event::Tab(third_party::workspace::Tab::Installed))));
    let targets = update(
        app,
        scoped(Action::Binding(desktop_settings_skills::update::bindings_view::Event::Open)),
    );
    Task::batch([catalog_load, inventory, targets])
}

pub(crate) fn update(
    app: &mut App,
    event: desktop_settings_skills::third_party::Message,
) -> Task<Message> {
    if !super::workspace_visible(app) {
        return Task::none();
    }
    use desktop_settings_skills::{
        third_party::{Action, DirectoryAction, Effect},
        update::bindings_view,
    };
    let mut input = crate::app::message::settings::skills::update_input(app);
    input.catalog = app.apps_catalog.navigation_generation;
    use desktop_settings_skills::third_party::workspace::Event as WorkspaceEvent;
    if matches!(
        &event.action,
        Action::Workspace(WorkspaceEvent::Install(_) | WorkspaceEvent::ConfirmRepository)
            | Action::Directory(DirectoryAction::Import)
    ) {
        let page = &app.skills_settings.workspace;
        if page.awaiting_target_snapshot() || app.apps_catalog.loading {
            return app.show_message_toast(
                crate::app::state::ToastKind::Error,
                desktop_i18n::settings_extensions::feedback_message(
                    &(if page.bindings.busy || app.apps_catalog.loading {
                        "正在读取当前项目的安装目标，请稍候"
                    } else {
                        "安装目标读取失败，请在「安装到」中重试或选择「仅主副本」"
                    }),
                ),
            );
        }
    }
    let inventory_readback =
        matches!(&event.action, Action::Workspace(WorkspaceEvent::Loaded(_, Ok(_))));
    let snapshot_version =
        app.skills_settings.workspace.bindings.snapshot.as_ref().map(|snapshot| snapshot.version);
    match desktop_settings_skills::third_party::update(
        &mut app.skills_settings.workspace,
        &input,
        event,
    ) {
        Effect::None => Task::none(),
        Effect::Workspace { scope, catalog, effect } => {
            let page = &app.skills_settings.workspace;
            let reload_bindings = inventory_readback
                && !page.bindings.busy
                && page
                    .workspace
                    .inventory
                    .as_ref()
                    .zip(page.bindings.snapshot.as_ref())
                    .is_some_and(|(inventory, snapshot)| inventory.version != snapshot.version);
            let refresh = reload_bindings.then(|| desktop_settings_skills::third_party::Message {
                scope: scope.clone(),
                catalog,
                action: Action::Binding(bindings_view::Event::Refresh),
            });
            let workspace = execute(app, scope, catalog, effect);
            if let Some(event) = refresh {
                return Task::batch([workspace, update(app, event)]);
            }
            workspace
        }
        Effect::Binding { scope, catalog, effect } => match effect {
            bindings_view::Effect::None => Task::none(),
            bindings_view::Effect::Error(error) => {
                let toast = app.show_message_toast(
                    crate::app::state::ToastKind::Error,
                    desktop_i18n::settings_extensions::feedback_message(&(error)),
                );
                let versions_changed = app
                    .skills_settings
                    .workspace
                    .bindings
                    .snapshot
                    .as_ref()
                    .zip(app.skills_settings.workspace.workspace.inventory.as_ref())
                    .is_some_and(|(snapshot, inventory)| {
                        Some(snapshot.version) != snapshot_version
                            && snapshot.version != inventory.version
                    });
                if versions_changed {
                    return Task::batch([
                        toast,
                        crate::app::message::settings::skills::refresh_workspace(app),
                    ]);
                }
                toast
            }
            bindings_view::Effect::Success(notice) => Task::batch([
                app.show_message_toast(
                    crate::app::state::ToastKind::Success,
                    desktop_i18n::settings_extensions::feedback_message(&(notice)),
                ),
                crate::app::message::settings::skills::refresh_workspace(app),
            ]),
            bindings_view::Effect::Request { generation, scope: request_scope, change } => {
                let token = app.apps_catalog.token.clone();
                Task::perform(
                    async move {
                        desktop_session::session_gateway::gateway_skill_bindings_request(
                            crate::app::config::gateway_client_endpoint(),
                            request_scope,
                            token,
                            change,
                        )
                        .await
                    },
                    move |result| {
                        message(third_party::Message {
                            scope: scope.clone(),
                            catalog,
                            action: Action::Binding(bindings_view::Event::Completed(
                                generation, result,
                            )),
                        })
                    },
                )
            }
        },
        Effect::Directory { action, .. } => {
            use crate::app::message::settings::skills;
            let event = match action {
                DirectoryAction::Claude => {
                    SettingsMessage::SkillsClaudeConfig(skills::claude_view::Event::Open)
                }
                DirectoryAction::Repositories => {
                    SettingsMessage::SkillRepositories(skills::repo_modal::Event::Open)
                }
                DirectoryAction::Import => {
                    SettingsMessage::SkillsImport(skills::import_modal::Event::Open)
                }
                DirectoryAction::Install => {
                    SettingsMessage::SkillsInstall(skills::install_view::Event::Open)
                }
                DirectoryAction::Market => {
                    SettingsMessage::SkillsMarket(skills::market_view::Event::Open)
                }
                DirectoryAction::Updates => {
                    SettingsMessage::SkillsUpdates(skills::updates_view::Event::Open)
                }
                DirectoryAction::Jobs => {
                    SettingsMessage::SkillsJobs(skills::jobs_view::Event::Open)
                }
                DirectoryAction::Storage => {
                    SettingsMessage::SkillsStorage(skills::storage_modal::Event::Open)
                }
                DirectoryAction::Backup => {
                    SettingsMessage::SkillsBackups(skills::backup_modal::Event::Open)
                }
            };
            let opening_import =
                matches!(action, DirectoryAction::Import) && !app.skills_settings.import.open;
            let task = skills::update(app, event);
            if opening_import && app.skills_settings.import.open {
                let page = &app.skills_settings.workspace;
                let import = &mut app.skills_settings.import;
                import.available_targets = page.available_targets();
                import.targets = page
                    .bindings
                    .targets
                    .iter()
                    .filter(|target| import.available_targets.contains(target))
                    .cloned()
                    .collect();
                let scan_sources = import
                    .targets
                    .iter()
                    .map(|target| target.app_id)
                    .filter(|id| desktop_settings_skills::import_modal::APPS.contains(id));
                if let Some(source) = scan_sources
                    .clone()
                    .find(|id| *id == vw_api_types::agent_apps::AppId::Codex)
                    .or_else(|| scan_sources.into_iter().next())
                {
                    import.apps.push(source);
                    let scan = skills::update(
                        app,
                        SettingsMessage::SkillsImport(skills::import_modal::Event::Scan),
                    );
                    return Task::batch([task, scan]);
                }
            }
            if matches!(action, DirectoryAction::Backup) {
                // The list readback validates these choices against the same scope.
                app.skills_settings.backups.targets =
                    app.skills_settings.workspace.bindings.targets.clone();
            }
            task
        }
    }
}
