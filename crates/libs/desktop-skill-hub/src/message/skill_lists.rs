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

pub(super) fn response_is_current(
    app: &SkillUpdate<'_>,
    generation: u64,
    account_id: &str,
) -> bool {
    app.expert_hub.request_generation == generation
        && app.expert_hub.account_id.as_deref() == Some(account_id)
}

pub(super) fn installed_response_is_current(
    app: &SkillUpdate<'_>,
    generation: u64,
    account_id: &str,
) -> bool {
    app.expert_hub.installed_request_generation == generation
        && app.expert_hub.account_id.as_deref() == Some(account_id)
}

pub(super) fn tag_catalog_response_is_current(
    app: &SkillUpdate<'_>,
    generation: u64,
    account_id: &str,
) -> bool {
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
pub fn mine_query(state: &crate::skill_hub::ExpertHubState) -> ShijuMaintainedSkillQuery {
    let keyword = state.mine_filter.submitted_keyword.clone();
    ShijuMaintainedSkillQuery {
        page: state.mine_page,
        page_size: state.mine_page_size,
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
            load_marketplace_skills(app, None),
            load_featured_skills(app, false),
            load_installed_skills(app, false),
            load_mine_tag_catalogs(app),
        ]);
    }

    let active = app
        .active_account
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

pub(super) fn load_marketplace_skills(app: &mut SkillUpdate<'_>, cursor: Option<String>) -> Task<Message> {
    let gateway = app.gateway;
    let active = app
        .active_account
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
    let append = cursor.is_some();
    app.expert_hub.market_loading_more = append;
    app.expert_hub.market_more_error = None;
    if !append {
        app.expert_hub.marketplace = LoadState::Loading { generation };
    }
    let mut query = marketplace_query(app.expert_hub);
    query.cursor = cursor;
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
            Message::SkillHub(if append {
                SkillHubMessage::Catalog(SkillCatalogMessage::MarketplaceAppended { generation, account_id, result })
            } else {
                SkillHubMessage::MarketplaceLoaded { generation, account_id, result }
            })
        },
    )
}

pub(super) fn load_featured_skills(app: &mut SkillUpdate<'_>, refresh: bool) -> Task<Message> {
    let state = &mut app.expert_hub;
    if state.featured_loading || (!refresh && !state.featured_skills.is_empty()) {
        return Task::none();
    }
    let Some(account) = app.active_account else {
        return Task::none();
    };
    let account_id = account.id().to_string();
    let credential = account.skill_hub_credential();
    let exclude_ids = state.featured_skills.iter().map(|skill| skill.id).collect();
    state.featured_generation = state.featured_generation.wrapping_add(1);
    let generation = state.featured_generation;
    state.featured_loading = true;
    Task::perform(
        async move { SkillHubClient::new(credential)?.recommendations(exclude_ids).await },
        move |result| Message::SkillHub(SkillHubMessage::Catalog(SkillCatalogMessage::FeaturedLoaded {
            generation, account_id, refresh, result,
        })),
    )
}

/// 从市场关键词与两类单选标签构造首批查询。
pub fn marketplace_query(state: &crate::skill_hub::ExpertHubState) -> ShijuSkillSearchQuery {
    let query = state.submitted_search.clone();
    ShijuSkillSearchQuery {
        query: (!query.is_empty()).then_some(query),
        cursor: None,
        page_size: SHIJU_PAGE_SIZE,
        tag_ids: state.market_tag_id,
        company_tag_ids: state.market_company_tag_id,
        distribution_source: state.market_source,
    }
}

pub fn load_installed_skills(app: &mut SkillUpdate<'_>, force: bool) -> Task<Message> {
    let gateway = app.gateway;
    let active = app
        .active_account
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

pub(super) fn defer_catalog_search(app: &mut SkillUpdate<'_>, tab: SkillTab) -> Task<Message> {
    let state = &mut app.expert_hub;
    state.catalog_search_generation = state.catalog_search_generation.wrapping_add(1);
    let generation = state.catalog_search_generation;
    Task::perform(
        async { tokio::time::sleep(Duration::from_millis(300)).await },
        move |()| Message::SkillHub(SkillHubMessage::Catalog(SkillCatalogMessage::SearchReady {
            tab, generation,
        })),
    )
}

pub(super) fn update_catalog(app: &mut SkillUpdate<'_>, message: SkillCatalogMessage) -> Task<Message> {
    match message {
        SkillCatalogMessage::SearchReady { tab, generation } => {
            let state = &app.expert_hub;
            if app.screen != HubPage::ShijuSkills || state.skill_tab != tab
                || state.catalog_search_generation != generation
            {
                return Task::none();
            }
            match tab {
                SkillTab::Marketplace if state.search_draft.trim() != state.submitted_search => {
                    update(app, SkillHubMessage::SearchSubmitted)
                }
                SkillTab::Mine if state.mine_filter.keyword_draft.trim()
                    != state.mine_filter.submitted_keyword =>
                {
                    app.expert_hub.mine_filter.submit();
                    app.expert_hub.mine_page = 1;
                    load_current_tab(app)
                }
                _ => Task::none(),
            }
        }
        SkillCatalogMessage::InstalledSearchChanged(value) => {
            app.expert_hub.installed_search = value;
            Task::none()
        }
        SkillCatalogMessage::MarketSourceSelected(source) => {
            if app.expert_hub.skill_tab != SkillTab::Marketplace {
                return Task::none();
            }
            app.expert_hub.market_source = source;
            load_marketplace_skills(app, None)
        }
        SkillCatalogMessage::RefreshFeatured => load_featured_skills(app, true),
        SkillCatalogMessage::LoadMoreMarketplace => {
            let state = &app.expert_hub;
            if state.skill_tab != SkillTab::Marketplace || state.market_loading_more {
                return Task::none();
            }
            let LoadState::Loaded(page) = &state.marketplace else {
                return Task::none();
            };
            match page.next_cursor.clone().filter(|_| page.has_more) {
                Some(cursor) => load_marketplace_skills(app, Some(cursor)),
                None => Task::none(),
            }
        }
        SkillCatalogMessage::EditMaintainedSkill(id) => maintained::edit(app, id),
        SkillCatalogMessage::OpenMaintainedSkillAction(id, kind) => maintained::open(app, id, kind),
        SkillCatalogMessage::CloseMaintainedSkillAction => maintained::close(app),
        SkillCatalogMessage::MaintainedVisibilityAllChanged(value) => {
            maintained::change(app, |state| state.all = value)
        }
        SkillCatalogMessage::MaintainedVisibilityDepartmentsChanged(value) => {
            maintained::change(app, |state| state.department_ids = value)
        }
        SkillCatalogMessage::MaintainedVisibilityUsersChanged(value) => {
            maintained::change(app, |state| state.user_ids = value)
        }
        SkillCatalogMessage::ConfirmMaintainedSkillAction => maintained::submit(app),
        SkillCatalogMessage::MaintainedSkillActionFinished { generation, account_id, skill_id, result } => {
            maintained::finish(app, generation, &account_id, skill_id, result)
        }
        SkillCatalogMessage::MarketCategoriesReset => {
            if app.expert_hub.skill_tab != SkillTab::Marketplace {
                return Task::none();
            }
            app.expert_hub.select_market_tag(None);
            app.expert_hub.select_market_company_tag(None);
            load_marketplace_skills(app, None)
        }
        SkillCatalogMessage::MineStatusSelected(status) => {
            if app.expert_hub.skill_tab != SkillTab::Mine {
                return Task::none();
            }
            app.expert_hub.mine_statuses_draft =
                status.map(|status| vec![status]).unwrap_or_else(|| MineSkillStatus::ALL.to_vec());
            app.expert_hub.submit_mine_filters();
            app.expert_hub.mine_page = 1;
            load_current_tab(app)
        }
        SkillCatalogMessage::MinePageSizeSelected(size) => {
            if app.expert_hub.skill_tab != SkillTab::Mine
                || !matches!(size, 10 | 20 | 50 | 100)
                || app.expert_hub.mine_page_size == size
            {
                return Task::none();
            }
            app.expert_hub.mine_page_size = size;
            app.expert_hub.mine_page = 1;
            load_current_tab(app)
        }
        SkillCatalogMessage::FeaturedLoaded { generation, account_id, refresh, result } => {
            let state = &mut app.expert_hub;
            if state.featured_generation != generation
                || state.account_id.as_deref() != Some(account_id.as_str())
            {
                return Task::none();
            }
            state.featured_loading = false;
            match result {
                Ok(items) if !items.is_empty() => {
                    state.featured_skills = items;
                }
                Ok(_) if refresh => return app.feedback(ToastKind::Info,
                    desktop_i18n::format::Message::new("暂无更多精选技能", vec![])),

                Err(error) if refresh => return app.feedback(ToastKind::Error, error.message()),
                _ => {}
            }
            Task::none()
        }
        SkillCatalogMessage::MarketplaceAppended { generation, account_id, result } => {
            if response_is_current(app, generation, &account_id) {
                app.expert_hub.market_loading_more = false;
                match result {
                    Ok(next) => {
                        if let LoadState::Loaded(page) = &mut app.expert_hub.marketplace {
                            let mut known: std::collections::HashSet<_> =
                                page.items.iter().map(|item| item.id).collect();
                            page.items.extend(next.items.into_iter().filter(|item| known.insert(item.id)));
                            page.has_more = next.has_more && next.next_cursor != page.next_cursor;
                            page.next_cursor = next.next_cursor;
                        }
                    }
                    Err(error) => app.expert_hub.market_more_error = Some(error),
                }
            }
            Task::none()
        }
    }
}
