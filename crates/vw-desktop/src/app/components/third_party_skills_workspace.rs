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
            app.show_error_toast("文档地址无效")
        };
    }
    if matches!(effect, Effect::None) {
        return Task::none();
    }
    let targets = app.apps_catalog.third_party_skills.bindings.targets.clone();
    let input = crate::app::message::settings::skills::update_input(app);
    let snapshot = app.apps_catalog.third_party_skills.bindings.snapshot.clone();
    let targets = targets
        .into_iter()
        .filter(|target| {
            input.targets.contains(target)
                && snapshot.as_ref().is_some_and(|s| {
                    s.capabilities.iter().any(|c| c.app_id == target.app_id && c.supported)
                })
        })
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
                Effect::None | Effect::Documentation(_) => unreachable!(),
            }
        },
        move |event| {
            Message::Settings(SettingsMessage::AppsCatalog(AppsCatalogMessage::ThirdPartySkills(
                third_party::Message {
                    scope: scope.clone(),
                    catalog,
                    action: third_party::Action::Workspace(event),
                },
            )))
        },
    )
}
