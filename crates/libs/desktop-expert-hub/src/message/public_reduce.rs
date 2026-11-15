use super::*;

/// 纯化处理公共市场状态转换，并返回后续请求而不直接执行网络操作。
pub fn reduce(
    state: &mut PublicSkillMarketState,
    message: PublicSkillMessage,
) -> Vec<PublicRequest> {
    match message {
        PublicSkillMessage::LoadSources => vec![load_sources(state)],
        PublicSkillMessage::SourcesLoaded { generation, result } => {
            if state.sources_generation != generation {
                return Vec::new();
            }
            match result {
                Ok(sources) => {
                    let default_source = sources
                        .iter()
                        .find(|source| source.source == "skillhub")
                        .or_else(|| sources.first())
                        .map(|source| source.source.clone());
                    state.sources = LoadState::Loaded(sources);
                    if let Some(source) = default_source {
                        state.current_source = None;
                        state.select_source(&source);
                        load_current_tab(state, false)
                    } else {
                        state.current_source = None;
                        Vec::new()
                    }
                }
                Err(error) => {
                    state.sources = LoadState::Error(error.into());
                    Vec::new()
                }
            }
        }
        PublicSkillMessage::SelectSource(source) => {
            if state.select_source(&source) {
                load_current_tab(state, false)
            } else {
                Vec::new()
            }
        }
        PublicSkillMessage::SelectTab(tab) => {
            state.tab = tab;
            load_current_tab(state, false)
        }
        PublicSkillMessage::RotateFeatured => {
            state.rotate_featured();
            Vec::new()
        }
        PublicSkillMessage::QueryDraftChanged(query) => {
            state.query.keyword_draft = query;
            Vec::new()
        }
        PublicSkillMessage::SubmitQuery => {
            state.submit_query();
            if state.tab == PublicSkillTab::Recommended {
                state.tab = PublicSkillTab::SkillHub;
            }
            load_current_tab(state, true)
        }
        PublicSkillMessage::SelectCategory(category) => {
            state.query.category = category.filter(|category| !category.trim().is_empty());
            state.submit_query();
            load_current_tab(state, true)
        }
        PublicSkillMessage::SelectSuiteCategory(category) => {
            state.suites_category = category.filter(|category| !category.trim().is_empty());
            state.suites_page = 1;
            state.suites_generation = state.suites_generation.wrapping_add(1);
            state.suites = LoadState::Idle;
            load_current_tab(state, true)
        }
        PublicSkillMessage::SelectSort(sort) => {
            state.query.sort = sort;
            state.submit_query();
            load_current_tab(state, true)
        }
        PublicSkillMessage::PreviousPage => change_page(state, false),
        PublicSkillMessage::NextPage => change_page(state, true),
        PublicSkillMessage::Retry => {
            if matches!(state.sources, LoadState::Error(_) | LoadState::Idle) {
                vec![load_sources(state)]
            } else {
                load_current_tab(state, true)
            }
        }
        PublicSkillMessage::OpenInstalled => {
            state.installed_open = true;
            state.close_detail();
            Vec::new()
        }
        PublicSkillMessage::CloseInstalled => {
            state.installed_open = false;
            state.delete_confirmation_open = false;
            Vec::new()
        }
        PublicSkillMessage::SetInstalledSelected { skill_id, selected } => {
            state.set_installed_selected(&skill_id, selected);
            Vec::new()
        }
        PublicSkillMessage::SelectAllInstalled => {
            state.select_all_installed();
            Vec::new()
        }
        PublicSkillMessage::ClearInstalledSelection => {
            state.clear_installed_selection();
            Vec::new()
        }
        PublicSkillMessage::RequestBatchDelete => {
            if !state.installed_selection.is_empty() && state.batch_action.is_none() {
                state.delete_confirmation_open = true;
            }
            Vec::new()
        }
        PublicSkillMessage::CancelBatchDelete => {
            state.delete_confirmation_open = false;
            Vec::new()
        }
        PublicSkillMessage::InstallCurrent
        | PublicSkillMessage::SelectExpertForChat
        | PublicSkillMessage::StartBatchEnabled(_)
        | PublicSkillMessage::ToggleInstalled { .. }
        | PublicSkillMessage::ConfirmBatchDelete => Vec::new(),
        PublicSkillMessage::OpenExpertInstall => {
            if let Some(detail) = state.detail.as_mut()
                && matches!(detail.target, PublicDetailTarget::Suite { .. })
                && !detail.install_is_busy()
                && matches!(detail.install, PublicInstallState::Idle)
            {
                detail.install_confirmation_open = true;
            }
            Vec::new()
        }
        PublicSkillMessage::CancelExpertInstall => {
            if let Some(detail) = state.detail.as_mut() {
                detail.install_confirmation_open = false;
            }
            Vec::new()
        }
        PublicSkillMessage::OpenSkillDetail(locator) => {
            if state.current_source.as_deref() != Some(&locator.source) {
                return Vec::new();
            }
            let generation = state.open_skill_detail(locator.clone());
            vec![
                PublicRequest::SkillDetail { locator: locator.clone(), generation },
                PublicRequest::SkillFile { locator, generation },
            ]
        }
        PublicSkillMessage::OpenSuiteDetail { source, slug } => {
            if state.current_source.as_deref() != Some(&source) {
                return Vec::new();
            }
            let generation = state.open_suite_detail(source.clone(), slug.clone());
            vec![PublicRequest::SuiteDetail { source, slug, generation }]
        }
        PublicSkillMessage::CloseDetail => {
            state.close_detail();
            Vec::new()
        }
        PublicSkillMessage::SetDetailMode(mode) => {
            if let Some(detail) = state.detail.as_mut() {
                detail.mode = mode;
            }
            Vec::new()
        }
        PublicSkillMessage::SelectDetailInstallScope(scope) => {
            if let Some(detail) = state.detail.as_mut() {
                detail.select_install_scope(scope);
            }
            Vec::new()
        }
        PublicSkillMessage::SelectDetailInstallProject(project_path) => {
            if let Some(detail) = state.detail.as_mut() {
                detail.select_install_project_path(project_path);
            }
            Vec::new()
        }
        PublicSkillMessage::ToggleDetailInstallTarget(target) => {
            if let Some(detail) = state.detail.as_mut() {
                detail.toggle_install_target(target);
            }
            Vec::new()
        }
        PublicSkillMessage::SkillDetailLoaded { locator, generation, result } => {
            if let Some(detail) = current_skill_detail(state, &locator, generation) {
                detail.skill_detail = result
                    .map(LoadState::Loaded)
                    .unwrap_or_else(|error| LoadState::Error(error.into()));
            }
            Vec::new()
        }
        PublicSkillMessage::SkillFileLoaded { locator, generation, result } => {
            if let Some(detail) = current_skill_detail(state, &locator, generation) {
                match result {
                    Ok(file) => {
                        detail.markdown_items = parse_public_markdown(&file.content);
                        detail.skill_file = LoadState::Loaded(file);
                    }
                    Err(error) => detail.skill_file = LoadState::Error(error.into()),
                }
            }
            Vec::new()
        }
        PublicSkillMessage::SuiteDetailLoaded { source, slug, generation, result } => {
            if let Some(detail) = current_suite_detail(state, &source, &slug, generation) {
                match result {
                    Ok(suite) => {
                        detail.markdown_items = parse_public_markdown(&suite.content);
                        detail.suite_detail = LoadState::Loaded(suite);
                    }
                    Err(error) => detail.suite_detail = LoadState::Error(error.into()),
                }
            }
            Vec::new()
        }
        PublicSkillMessage::InstallFinished { target, generation, project_path, result } => {
            if !install_response_is_current(state, &target, generation) {
                return Vec::new();
            }
            let detail = state.detail.as_mut().expect("current detail must exist");
            match result {
                Ok(paths) => {
                    detail.install = PublicInstallState::Installed { paths };
                    if project_path.trim().is_empty() {
                        return Vec::new();
                    }
                    state.installed_generation = state.installed_generation.wrapping_add(1);
                    let generation = state.installed_generation;
                    state.installed = LoadState::Loading { generation };
                    vec![PublicRequest::RefreshInstalled { project_path, generation }]
                }
                Err(_) => {
                    detail.install = PublicInstallState::Idle;
                    Vec::new()
                }
            }
        }
        PublicSkillMessage::InstalledRefreshed { generation, result } => {
            if state.installed_generation == generation {
                match result {
                    Ok(items) => {
                        let manageable = items
                            .iter()
                            .filter(|item| item.installed && item.source_path.is_some())
                            .map(|item| item.id.as_str())
                            .collect::<std::collections::BTreeSet<_>>();
                        state
                            .installed_selection
                            .retain(|skill_id| manageable.contains(skill_id.as_str()));
                        state.installed = LoadState::Loaded(items);
                    }
                    Err(error) => state.installed = LoadState::Error(error.into()),
                }
            }
            Vec::new()
        }
        PublicSkillMessage::BatchItemFinished { generation, project_path, skill_id, result } => {
            finish_batch_item(state, generation, project_path, skill_id, result)
        }
        PublicSkillMessage::RecommendedLoaded { source, generation, result } => {
            if response_is_current(state, &source, generation, state.recommended_generation) {
                state.featured_offset = 0;
                state.recommended = result
                    .map(LoadState::Loaded)
                    .unwrap_or_else(|error| LoadState::Error(error.into()));
            }
            Vec::new()
        }
        PublicSkillMessage::CategoriesLoaded { source, generation, result } => {
            if response_is_current(state, &source, generation, state.categories_generation) {
                state.categories = result
                    .map(LoadState::Loaded)
                    .unwrap_or_else(|error| LoadState::Error(error.into()));
            }
            Vec::new()
        }
        PublicSkillMessage::MarketLoaded { source, generation, result } => {
            if response_is_current(state, &source, generation, state.market_generation) {
                state.market = result
                    .map(LoadState::Loaded)
                    .unwrap_or_else(|error| LoadState::Error(error.into()));
            }
            Vec::new()
        }
        PublicSkillMessage::SuitesLoaded { source, generation, result } => {
            if response_is_current(state, &source, generation, state.suites_generation) {
                state.suites = result
                    .map(LoadState::Loaded)
                    .unwrap_or_else(|error| LoadState::Error(error.into()));
            }
            Vec::new()
        }
    }
}

pub(super) fn load_sources(state: &mut PublicSkillMarketState) -> PublicRequest {
    state.sources_generation = state.sources_generation.wrapping_add(1);
    let generation = state.sources_generation;
    state.sources = LoadState::Loading { generation };
    PublicRequest::Sources { generation }
}

pub(super) fn load_current_tab(state: &mut PublicSkillMarketState, force: bool) -> Vec<PublicRequest> {
    let Some(source) = state.current_source.clone() else {
        return Vec::new();
    };
    match state.tab {
        PublicSkillTab::Recommended => {
            if !force && matches!(state.recommended, LoadState::Loaded(_)) {
                Vec::new()
            } else {
                state.recommended_generation = state.recommended_generation.wrapping_add(1);
                let generation = state.recommended_generation;
                state.recommended = LoadState::Loading { generation };
                vec![PublicRequest::Recommended { source, generation }]
            }
        }
        PublicSkillTab::SkillHub => {
            let mut requests = Vec::new();
            if force || !matches!(state.categories, LoadState::Loaded(_)) {
                state.categories_generation = state.categories_generation.wrapping_add(1);
                let generation = state.categories_generation;
                state.categories = LoadState::Loading { generation };
                requests.push(PublicRequest::Categories { source: source.clone(), generation });
            }
            if force || !matches!(state.market, LoadState::Loaded(_)) {
                state.market_generation = state.market_generation.wrapping_add(1);
                let generation = state.market_generation;
                state.market = LoadState::Loading { generation };
                requests.push(PublicRequest::Market {
                    source,
                    generation,
                    query: market_query(state, state.query.page),
                });
            }
            requests
        }
        PublicSkillTab::Suites => {
            if !force && matches!(state.suites, LoadState::Loaded(_)) {
                Vec::new()
            } else {
                state.suites_generation = state.suites_generation.wrapping_add(1);
                let generation = state.suites_generation;
                state.suites = LoadState::Loading { generation };
                vec![PublicRequest::Suites {
                    source,
                    generation,
                    query: SkillMarketSkillQuery {
                        category: state.suites_category.clone(),
                        ..market_query(state, state.suites_page)
                    },
                }]
            }
        }
    }
}

pub(super) fn change_page(state: &mut PublicSkillMarketState, next: bool) -> Vec<PublicRequest> {
    let changed = match state.tab {
        PublicSkillTab::SkillHub => {
            let can_advance = matches!(
                &state.market,
                LoadState::Loaded(page)
                    if u64::from(page.page) * u64::from(page.page_size) < page.total
            );
            if next && can_advance {
                state.query.page = state.query.page.saturating_add(1);
                true
            } else if !next && state.query.page > 1 {
                state.query.page -= 1;
                true
            } else {
                false
            }
        }
        PublicSkillTab::Suites => {
            let can_advance = matches!(
                &state.suites,
                LoadState::Loaded(page)
                    if u64::from(page.page) * u64::from(page.page_size) < page.total
            );
            if next && can_advance {
                state.suites_page = state.suites_page.saturating_add(1);
                true
            } else if !next && state.suites_page > 1 {
                state.suites_page -= 1;
                true
            } else {
                false
            }
        }
        PublicSkillTab::Recommended => false,
    };
    if changed { load_current_tab(state, true) } else { Vec::new() }
}

pub(super) fn market_query(state: &PublicSkillMarketState, page: u32) -> SkillMarketSkillQuery {
    SkillMarketSkillQuery {
        keyword: (!state.query.submitted_keyword.is_empty())
            .then(|| state.query.submitted_keyword.clone()),
        category: state.query.category.clone(),
        namespace: None,
        sort: state.query.sort,
        page,
        page_size: MARKET_PAGE_SIZE,
    }
}

pub(super) fn response_is_current(
    state: &PublicSkillMarketState,
    source: &str,
    generation: u64,
    current_generation: u64,
) -> bool {
    state.current_source.as_deref() == Some(source) && generation == current_generation
}

pub(super) fn current_skill_detail<'a>(
    state: &'a mut PublicSkillMarketState,
    locator: &SkillMarketSkillLocatorDto,
    generation: u64,
) -> Option<&'a mut desktop_skill_hub::skill_hub::PublicDetailState> {
    let detail = state.detail.as_mut()?;
    (detail.generation == generation && detail.target == PublicDetailTarget::Skill(locator.clone()))
        .then_some(detail)
}

pub(super) fn current_suite_detail<'a>(
    state: &'a mut PublicSkillMarketState,
    source: &str,
    slug: &str,
    generation: u64,
) -> Option<&'a mut desktop_skill_hub::skill_hub::PublicDetailState> {
    let detail = state.detail.as_mut()?;
    (detail.generation == generation
        && detail.target
            == PublicDetailTarget::Suite { source: source.to_string(), slug: slug.to_string() })
    .then_some(detail)
}

pub(super) fn install_response_is_current(
    state: &PublicSkillMarketState,
    target: &PublicDetailTarget,
    generation: u64,
) -> bool {
    state.detail.as_ref().is_some_and(|detail| {
        detail.target == *target
            && matches!(
                detail.install,
                PublicInstallState::Installing {
                    generation: active,
                } if active == generation
            )
    })
}
