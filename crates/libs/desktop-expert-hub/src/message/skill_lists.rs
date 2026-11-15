use super::*;

/// 账号切换后清除账号范围内的技能状态，并按当前页面重新加载。
pub fn account_changed(app: &mut SkillUpdate<'_>) -> Task<Message> {
    let account_id = app.active_account.map(|account| account.id().to_string());
    app.clear_expert_hub_account(account_id.clone());
    if account_id.is_none() {
        if app.expert_hub.install_guide.open {
            app.expert_hub.install_guide.agent_package = LoadState::NeedsLogin;
        } else {
            mark_current_needs_login(app);
        }
        Task::none()
    } else {
        let refresh = refresh_shiju_package_version(app);
        let reload = if app.screen == HubPage::ShijuApps {
            Task::done(Message::ReloadApps)
        } else if matches!(app.screen, HubPage::ShijuSkills | HubPage::ExpertHub) {
            Task::done(Message::SkillHub(SkillHubMessage::ReloadAfterAccountChange))
        } else {
            Task::none()
        };
        let installed = load_installed_skills(app, false);
        Task::batch([refresh, reload, installed])
    }
}

pub(super) fn refresh_shiju_package_version(app: &mut SkillUpdate<'_>) -> Task<Message> {
    let Some(account) = app.active_account else {
        return Task::none();
    };
    let Some(package_id) = ShijuPackageId::current() else {
        return Task::none();
    };
    let credential = account.skill_hub_credential();
    let generation = app.expert_hub.package_status.start_refresh();
    Task::perform(
        async move {
            let client = SkillHubClient::new(credential)
                .map_err(|_| SkillHubError::localized("无法创建识矩版本刷新客户端", vec![]))?;
            let release = client
                .latest_shiju_package(package_id)
                .await
                .map_err(|_| SkillHubError::localized("无法刷新识矩服务端版本", vec![]))?;
            let version = release
                .version()
                .ok_or_else(|| SkillHubError::localized("识矩服务端版本格式无效", vec![]))?
                .to_string();
            let checked_at_unix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| SkillHubError::localized("无法记录识矩版本检测时间", vec![]))?
                .as_secs();
            PackageVersionCacheEntry::new(version, checked_at_unix)
                .map_err(|_| SkillHubError::localized("识矩服务端版本格式无效", vec![]))
        },
        move |result| {
            Message::SkillHub(SkillHubMessage::ShijuPackageVersionRefreshed { generation, result })
        },
    )
}

pub(super) fn finish_shiju_package_version_refresh(
    app: &mut SkillUpdate<'_>,
    generation: u64,
    result: Result<PackageVersionCacheEntry, SkillHubError>,
) -> Task<Message> {
    if !app.expert_hub.package_status.is_refreshing(generation) {
        return Task::none();
    }
    let result = result.and_then(|entry| {
        let path = package_version_cache_path()
            .map_err(|_| SkillHubError::localized("无法确定识矩版本缓存路径", vec![]))?;
        write_package_version_cache(&path, &entry)
            .map_err(|_| SkillHubError::localized("无法更新识矩版本缓存", vec![]))?;
        Ok(entry)
    });
    app.expert_hub.package_status.finish_refresh(generation, result);
    Task::none()
}

pub fn reload_local_shiju_package_version(app: &mut SkillUpdate<'_>) {
    let Ok(installation) = shiju_installation() else {
        return;
    };
    app.expert_hub
        .package_status
        .set_local_version(installation.and_then(|installation| installation.version));
}

pub(super) fn response_is_current(app: &SkillUpdate<'_>, generation: u64, account_id: &str) -> bool {
    app.expert_hub.request_generation == generation
        && app.expert_hub.account_id.as_deref() == Some(account_id)
}

pub(super) fn installed_response_is_current(app: &SkillUpdate<'_>, generation: u64, account_id: &str) -> bool {
    app.expert_hub.installed_request_generation == generation
        && app.expert_hub.account_id.as_deref() == Some(account_id)
}

pub(super) fn tag_catalog_response_is_current(app: &SkillUpdate<'_>, generation: u64, account_id: &str) -> bool {
    app.expert_hub.tag_catalog_generation == generation
        && app.expert_hub.account_id.as_deref() == Some(account_id)
}

pub(super) fn mark_current_needs_login(app: &mut SkillUpdate<'_>) {
    match app.expert_hub.skill_tab {
        SkillTab::Marketplace => app.expert_hub.marketplace = LoadState::NeedsLogin,
        SkillTab::Installed => app.expert_hub.installed = LoadState::NeedsLogin,
        SkillTab::System => app.expert_hub.system = LoadState::NeedsLogin,
        SkillTab::Mine => app.expert_hub.mine = LoadState::NeedsLogin,
        SkillTab::Install => app.expert_hub.install_guide.agent_package = LoadState::NeedsLogin,
    }
}

pub(super) fn mine_tag_option_order(app: &SkillUpdate<'_>) -> Vec<u64> {
    let mut ids = Vec::new();
    if let LoadState::Loaded(tags) = &app.expert_hub.platform_tag_catalog {
        for tag in tags {
            if !ids.contains(&tag.id) {
                ids.push(tag.id);
            }
        }
    }
    for selected_id in &app.expert_hub.mine_tag_ids_draft {
        if !ids.contains(selected_id) {
            ids.push(*selected_id);
        }
    }
    ids
}

pub(super) fn mine_company_tag_option_order(app: &SkillUpdate<'_>) -> Vec<u64> {
    let mut ids = Vec::new();
    if let LoadState::Loaded(tags) = &app.expert_hub.company_tag_catalog {
        for tag in tags {
            if !ids.contains(&tag.id) {
                ids.push(tag.id);
            }
        }
    }
    for selected_id in &app.expert_hub.mine_company_tag_ids_draft {
        if !ids.contains(selected_id) {
            ids.push(*selected_id);
        }
    }
    ids
}

/// 从已提交的“我的技能”筛选状态构造稳定网关查询。
pub fn mine_query(state: &desktop_skill_hub::skill_hub::ExpertHubState) -> ShijuMaintainedSkillQuery {
    let keyword = state.mine_filter.submitted_keyword.clone();
    ShijuMaintainedSkillQuery {
        page: state.mine_page,
        page_size: SHIJU_PAGE_SIZE,
        keyword: (!keyword.is_empty()).then_some(keyword),
        tag_ids: state.submitted_mine_tag_ids.clone(),
        company_tag_ids: state.submitted_mine_company_tag_ids.clone(),
        statuses: state
            .submitted_mine_statuses
            .iter()
            .map(|status| status.as_str().to_string())
            .collect(),
    }
}

pub(super) fn load_current_tab(app: &mut SkillUpdate<'_>) -> Task<Message> {
    let gateway = app.gateway;
    if app.expert_hub.skill_tab == SkillTab::Installed {
        return load_installed_skills(app, true);
    }
    if app.expert_hub.skill_tab == SkillTab::Marketplace {
        return Task::batch([
            load_marketplace_skills(app),
            load_installed_skills(app, false),
            load_mine_tag_catalogs(app),
        ]);
    }

    let active = app.active_account
        .map(|account| (account.id().to_string(), account.skill_hub_credential()));
    let Some((account_id, credential)) = active else {
        if app.expert_hub.account_id.is_some() {
            app.clear_expert_hub_account(None);
        }
        mark_current_needs_login(app);
        return Task::none();
    };

    if app.expert_hub.account_id.as_deref() != Some(account_id.as_str()) {
        app.clear_expert_hub_account(Some(account_id.clone()));
    }
    let generation = app.expert_hub.next_request_generation();
    let tab = app.expert_hub.skill_tab;
    match tab {
        SkillTab::Marketplace => Task::none(),
        SkillTab::Installed => Task::none(),
        SkillTab::System => {
            app.expert_hub.system = LoadState::Loading { generation };
            let page = app.expert_hub.system_page;
            let name = app.expert_hub.system_filter.submitted_keyword.clone();
            let tag_id = app.expert_hub.system_filter.submitted_tag_id;
            Task::perform(
                async move {
                    let credential = credential
                        .gateway_credential()
                        .map_err(|error| SkillHubError::from(error.to_string()))?;
                    let query = ShijuSystemSkillQuery {
                        page,
                        page_size: SHIJU_PAGE_SIZE,
                        name: (!name.is_empty()).then_some(name),
                        tag_id,
                    };
                    gateway_shiju_system_skills_async(gateway, &credential, &query)
                        .await
                        .map(Page::from)
                        .map_err(SkillHubError::from)
                },
                move |result| {
                    Message::SkillHub(SkillHubMessage::SystemLoaded {
                        generation,
                        account_id,
                        result,
                    })
                },
            )
        }
        SkillTab::Mine => {
            app.expert_hub.mine = LoadState::Loading { generation };
            let query = mine_query(app.expert_hub);
            let list_task = Task::perform(
                async move {
                    let credential = credential
                        .gateway_credential()
                        .map_err(|error| SkillHubError::from(error.to_string()))?;
                    gateway_shiju_maintained_skills_async(gateway, &credential, &query)
                        .await
                        .map(Page::from)
                        .map_err(SkillHubError::from)
                },
                move |result| {
                    Message::SkillHub(SkillHubMessage::MineLoaded {
                        generation,
                        account_id,
                        result,
                    })
                },
            );
            Task::batch([list_task, load_mine_tag_catalogs(app)])
        }
        SkillTab::Install => load_agent_package(app, false),
    }
}

pub(super) fn load_mine_tag_catalogs(app: &mut SkillUpdate<'_>) -> Task<Message> {
    let gateway = app.gateway;
    let should_load =
        matches!(app.expert_hub.platform_tag_catalog, LoadState::Idle | LoadState::Error(_))
            || matches!(app.expert_hub.company_tag_catalog, LoadState::Idle | LoadState::Error(_));
    if !should_load {
        return Task::none();
    }
    let Some(account) = app.active_account else {
        return Task::none();
    };
    let account_id = account.id().to_string();
    let credential = account.skill_hub_credential();
    let generation = app.expert_hub.next_tag_catalog_generation();
    app.expert_hub.platform_tag_catalog = LoadState::Loading { generation };
    app.expert_hub.company_tag_catalog = LoadState::Loading { generation };

    let platform_account_id = account_id.clone();
    let platform_credential = credential.clone();
    let platform = Task::perform(
        async move {
            let credential = platform_credential
                .gateway_credential()
                .map_err(|error| SkillHubError::from(error.to_string()))?;
            gateway_shiju_skill_tags_async(gateway, &credential)
                .await
                .map(|catalog| catalog.items.into_iter().map(SkillTag::from).collect())
                .map_err(SkillHubError::from)
        },
        move |result| {
            Message::SkillHub(SkillHubMessage::PlatformTagCatalogLoaded {
                generation,
                account_id: platform_account_id,
                result,
            })
        },
    );
    let company = Task::perform(
        async move {
            let credential = credential
                .gateway_credential()
                .map_err(|error| SkillHubError::from(error.to_string()))?;
            gateway_shiju_tenant_skill_tags_async(gateway, &credential)
                .await
                .map(|catalog| catalog.items.into_iter().map(SkillTag::from).collect())
                .map_err(SkillHubError::from)
        },
        move |result| {
            Message::SkillHub(SkillHubMessage::CompanyTagCatalogLoaded {
                generation,
                account_id,
                result,
            })
        },
    );
    Task::batch([platform, company])
}

pub(super) fn load_marketplace_skills(app: &mut SkillUpdate<'_>) -> Task<Message> {
    let gateway = app.gateway;
    let active = app.active_account
        .map(|account| (account.id().to_string(), account.skill_hub_credential()));
    let Some((account_id, credential)) = active else {
        if app.expert_hub.account_id.is_some() {
            app.clear_expert_hub_account(None);
        }
        app.expert_hub.marketplace = LoadState::NeedsLogin;
        return Task::none();
    };
    if app.expert_hub.account_id.as_deref() != Some(account_id.as_str()) {
        app.clear_expert_hub_account(Some(account_id.clone()));
    }
    let generation = app.expert_hub.next_request_generation();
    app.expert_hub.marketplace = LoadState::Loading { generation };
    let query = marketplace_query(app.expert_hub);
    Task::perform(
        async move {
            let credential = credential
                .gateway_credential()
                .map_err(|error| SkillHubError::from(error.to_string()))?;
            gateway_shiju_skill_search_async(gateway, &credential, &query)
                .await
                .map(CursorPage::from)
                .map_err(SkillHubError::from)
        },
        move |result| {
            Message::SkillHub(SkillHubMessage::MarketplaceLoaded { generation, account_id, result })
        },
    )
}

/// 从市场关键词与两类单选标签构造首批查询。
pub fn marketplace_query(state: &desktop_skill_hub::skill_hub::ExpertHubState) -> ShijuSkillSearchQuery {
    let query = state.submitted_search.clone();
    ShijuSkillSearchQuery {
        query: (!query.is_empty()).then_some(query),
        cursor: None,
        page_size: SHIJU_PAGE_SIZE,
        tag_ids: state.market_tag_id,
        company_tag_ids: state.market_company_tag_id,
    }
}

pub fn load_installed_skills(app: &mut SkillUpdate<'_>, force: bool) -> Task<Message> {
    let gateway = app.gateway;
    let active = app.active_account
        .map(|account| (account.id().to_string(), account.skill_hub_credential()));
    let Some((account_id, credential)) = active else {
        if app.expert_hub.account_id.is_some() {
            app.clear_expert_hub_account(None);
        }
        app.expert_hub.installed = LoadState::NeedsLogin;
        return Task::none();
    };

    if app.expert_hub.account_id.as_deref() != Some(account_id.as_str()) {
        app.clear_expert_hub_account(Some(account_id.clone()));
    }
    if !force
        && matches!(app.expert_hub.installed, LoadState::Loading { .. } | LoadState::Loaded(_))
    {
        return Task::none();
    }

    let generation = app.expert_hub.next_installed_request_generation();
    app.expert_hub.installed = LoadState::Loading { generation };
    Task::perform(
        async move {
            let credential = credential
                .gateway_credential()
                .map_err(|error| SkillHubError::from(error.to_string()))?;
            gateway_shiju_skill_installed_async(gateway, &credential)
                .await
                .map(|items| items.into_iter().map(InstalledSkill::from).collect())
                .map_err(SkillHubError::from)
        },
        move |result| {
            Message::SkillHub(SkillHubMessage::InstalledLoaded { generation, account_id, result })
        },
    )
}
