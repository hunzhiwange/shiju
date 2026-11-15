//! 协调识矩技能列表、详情、安装、版本操作与本地缓存同步消息。

use super::skill_gateway::*;
use desktop_skill_hub::skill_hub::{
    CursorPage, ExpertHubSection, InstalledSkill, LoadState, MarketplaceSkill, MineFilterMenu,
    MineSkillStatus, PackageVersionCacheEntry, Page, PublicDetailMode,
    ShijuDirectInstallState, ShijuInstallationsReport, ShijuKeySyncOutcome,
    ShijuKeySyncReport, ShijuMultiInstallReport, ShijuPackageId, ShijuPackageRelease,
    ShijuUninstallReport, SkillActionKind, SkillCacheSyncReport, SkillDetail,
    SkillDetailState, SkillHubClient, SkillHubError, SkillInstallScope, SkillInstallSelection,
    SkillInstallTarget, SkillTab, SkillTag, SkillVersion, SkillVersionAction,
    SkillVersionDirection, SystemSkill, TenantSkill, install_shiju_archive_for_targets,
    package_version_cache_path, parse_public_markdown, resolve_skill_destinations,
    shiju_installation, shiju_installations_for_targets, sync_business_skill_cache,
    sync_shiju_key_for_targets, uninstall_shiju_for_targets, write_package_version_cache,
};
use super::{
    SkillUpdate, Effect, Message, HubPage, ToastKind,
};
use iced::Task;
use reqwest::redirect::Policy;
use std::path::{
    PathBuf,
};
use std::time::Duration;
use tempfile::NamedTempFile;
use gateway_shiju_skill::{
    ShijuMaintainedSkillQuery, ShijuSkillSearchQuery, ShijuSystemSkillQuery,
};

#[path = "skill_installation.rs"]
mod installation;
#[path = "skill_business.rs"]
mod business;
#[path = "skill_lists.rs"]
mod lists;
#[path = "skill_guide.rs"]
mod guide;
use guide::*;
use installation::*;
use business::*;
use lists::*;
pub use installation::{
    install_shiju_package_for_targets,
};
pub use guide::{active_shiju_account, selected_project_root};
pub use business::dispatch_download_with;
pub use lists::{
    account_changed, load_installed_skills, reload_local_shiju_package_version, mine_query,
    marketplace_query,
};

/// 专家中心在主标签栏中的稳定标识。
pub const EXPERT_HUB_TAB_ID: &str = "expert_hub";
/// 专家中心标签的用户可见标题。
pub const EXPERT_HUB_TITLE: &str = "专家·技能·连接器";
/// 识矩技能兼容入口的稳定标签标识。
pub const SHIJU_SKILLS_TAB_ID: &str = "shiju_skills";
/// 识矩技能兼容入口的用户可见标题。
pub const SHIJU_SKILLS_TITLE: &str = "识矩技能";
const SHIJU_PAGE_SIZE: u32 = 20;
pub use crate::skill_operations::*;

#[path = "skill_message.rs"]
mod skill_message;
pub use skill_message::SkillHubMessage;


/// 归约技能中心消息，并用请求代次与账号 ID 屏蔽迟到的异步响应。
pub fn update(app: &mut SkillUpdate<'_>, message: SkillHubMessage) -> Task<Message> {
    if (app.expert_hub.function_upgrade.is_some() || app.expert_hub.main_upgrade.is_some())
        && matches!(
            &message,
            SkillHubMessage::DirectInstallShiju
                | SkillHubMessage::SyncShijuKey
                | SkillHubMessage::ConfirmShijuUninstall
                | SkillHubMessage::RetrySkillCacheSync
        )
    {
        return Task::none();
    }
    match message {
        SkillHubMessage::Connectors(message) => super::connectors::update(app, message),
        SkillHubMessage::Knowledge(message) => super::knowledge::update(app, message),
        SkillHubMessage::Public(message) => super::public::update(app, message),
        SkillHubMessage::Open => {
            app.expert_hub.reset_navigation();
            open_expert_hub_tab(app);
            app.effect(Effect::EnsurePublic)
        }
        SkillHubMessage::OpenShijuSkills => {
            app.expert_hub.install_guide.close();
            app.expert_hub.skill_tab = SkillTab::Marketplace;
            app.expert_hub.detail = None;
            open_shiju_skills_tab(app);
            load_current_tab(app)
        }
        SkillHubMessage::OpenInstalledShijuSkills => {
            app.expert_hub.install_guide.close();
            app.expert_hub.skill_tab = SkillTab::Installed;
            app.expert_hub.detail = None;
            open_shiju_skills_tab(app);
            Task::batch([load_current_tab(app), refresh_shiju_package_version(app)])
        }
        SkillHubMessage::OpenInstallGuide => open_install_guide(app),
        SkillHubMessage::CloseInstallGuide => {
            app.expert_hub.install_guide.close();
            app.expert_hub.skill_tab = SkillTab::Marketplace;
            load_current_tab(app)
        }
        SkillHubMessage::CopyAgentInstall => load_agent_package(app, true),
        SkillHubMessage::AgentPackageLoaded { generation, account_id, copy_after_load, result } => {
            finish_agent_package_load(app, generation, &account_id, copy_after_load, result)
        }
        SkillHubMessage::DownloadShijuPackage(package_id) => start_manual_download(app, package_id),
        SkillHubMessage::ManualPackageLoaded { generation, account_id, package_id, result } => {
            finish_manual_download(app, generation, &account_id, package_id, result)
        }
        SkillHubMessage::DirectInstallShiju => start_direct_install(app),
        SkillHubMessage::SelectShijuInstallScope(scope) => select_shiju_install_scope(app, scope),
        SkillHubMessage::SelectShijuInstallProject(project_path) => {
            select_shiju_install_project(app, project_path)
        }
        SkillHubMessage::ToggleShijuInstallTarget(target) => {
            toggle_shiju_install_target(app, target)
        }
        SkillHubMessage::SyncShijuKey => start_manual_shiju_key_sync(app),
        SkillHubMessage::UninstallShiju => {
            if app.expert_hub.install_guide.is_installed
                && matches!(
                    app.expert_hub.install_guide.direct_install,
                    ShijuDirectInstallState::Idle | ShijuDirectInstallState::Installed
                )
            {
                app.expert_hub.install_guide.uninstall_confirmation_open = true;
            }
            Task::none()
        }
        SkillHubMessage::CancelShijuUninstall => {
            app.expert_hub.install_guide.uninstall_confirmation_open = false;
            Task::none()
        }
        SkillHubMessage::ConfirmShijuUninstall => {
            if !app.expert_hub.install_guide.uninstall_confirmation_open {
                return Task::none();
            }
            app.expert_hub.install_guide.uninstall_confirmation_open = false;
            start_shiju_uninstall(app)
        }
        SkillHubMessage::OpenShijuInstallDir => open_selected_shiju_install_dir(app),
        SkillHubMessage::OpenShijuInstallDirFor(target) => open_shiju_install_dir_for(app, target),
        SkillHubMessage::ShijuInstallationsLoaded { generation, selections, result } => {
            finish_installation_load(app, generation, &selections, result)
        }
        SkillHubMessage::DirectPackageLoaded {
            generation,
            account_id,
            package_id,
            selections,
            project_root,
            result,
        } => finish_direct_package_load(
            app,
            generation,
            &account_id,
            package_id,
            selections,
            project_root,
            result,
        ),
        SkillHubMessage::ShijuInstallFinished {
            generation,
            account_id,
            package_id,
            selections,
            result,
        } => finish_direct_install(app, generation, &account_id, package_id, &selections, result),
        SkillHubMessage::ShijuKeySyncFinished {
            generation,
            account_id,
            selections,
            requested_by_user,
            result,
        } => finish_shiju_key_sync(
            app,
            generation,
            &account_id,
            &selections,
            requested_by_user,
            result,
        ),
        SkillHubMessage::ShijuUninstallFinished { generation, selections, result } => {
            finish_shiju_uninstall(app, generation, &selections, result)
        }
        SkillHubMessage::ShijuPackageVersionRefreshed { generation, result } => {
            finish_shiju_package_version_refresh(app, generation, result)
        }
        SkillHubMessage::SelectSection(section) => {
            open_expert_hub_tab(app);
            if app.expert_hub.section != section {
                app.expert_hub.public_market.close_detail();
            }
            app.expert_hub.install_guide.close();
            app.expert_hub.close_mine_filter_menu();
            if app.expert_hub.skill_tab == SkillTab::Install {
                app.expert_hub.skill_tab = SkillTab::Marketplace;
            }
            app.expert_hub.section = section;
            if section == ExpertHubSection::Experts {
                let suites = app.effect(Effect::SelectPublicSuites);
                Task::batch([suites, app.effect(Effect::EnsurePublic)])
            } else if section == ExpertHubSection::Connectors {
                app.effect(Effect::EnsureConnectors)
            } else if section == ExpertHubSection::Knowledge {
                app.effect(Effect::EnsureKnowledge)
            } else if section == ExpertHubSection::Skills {
                app.effect(Effect::EnsurePublic)
            } else {
                Task::none()
            }
        }
        SkillHubMessage::SelectSkillTab(tab) => {
            if tab == SkillTab::Install {
                return open_install_guide(app);
            }
            app.expert_hub.install_guide.close();
            app.expert_hub.close_mine_filter_menu();
            app.expert_hub.skill_tab = tab;
            load_current_tab(app)
        }
        SkillHubMessage::SearchDraftChanged(query) => {
            app.expert_hub.search_draft = query;
            Task::none()
        }
        SkillHubMessage::SearchSubmitted => {
            app.expert_hub.install_guide.close();
            app.expert_hub.close_mine_filter_menu();
            app.expert_hub.skill_tab = SkillTab::Marketplace;
            app.expert_hub.submitted_search = app.expert_hub.search_draft.trim().to_string();
            load_current_tab(app)
        }
        SkillHubMessage::MarketTagSelected(tag_id) => {
            if app.expert_hub.skill_tab != SkillTab::Marketplace {
                return Task::none();
            }
            app.expert_hub.select_market_tag(tag_id);
            load_current_tab(app)
        }
        SkillHubMessage::MarketCompanyTagSelected(tag_id) => {
            if app.expert_hub.skill_tab != SkillTab::Marketplace {
                return Task::none();
            }
            app.expert_hub.select_market_company_tag(tag_id);
            load_current_tab(app)
        }
        SkillHubMessage::FilterKeywordChanged(keyword) => {
            match app.expert_hub.skill_tab {
                SkillTab::System => app.expert_hub.system_filter.keyword_draft = keyword,
                SkillTab::Mine => app.expert_hub.mine_filter.keyword_draft = keyword,
                SkillTab::Marketplace | SkillTab::Installed | SkillTab::Install => {}
            }
            Task::none()
        }
        SkillHubMessage::FilterTagChanged(tag_id) => {
            match app.expert_hub.skill_tab {
                SkillTab::System => app.expert_hub.system_filter.tag_id_draft = tag_id,
                SkillTab::Mine => {}
                SkillTab::Marketplace | SkillTab::Installed | SkillTab::Install => {}
            }
            Task::none()
        }
        SkillHubMessage::ToggleMineTagFilterMenu => {
            if app.expert_hub.skill_tab == SkillTab::Mine {
                app.expert_hub.toggle_mine_filter_menu(MineFilterMenu::Tags);
            }
            Task::none()
        }
        SkillHubMessage::ToggleMineCompanyTagFilterMenu => {
            if app.expert_hub.skill_tab == SkillTab::Mine {
                app.expert_hub.toggle_mine_filter_menu(MineFilterMenu::CompanyTags);
            }
            Task::none()
        }
        SkillHubMessage::ToggleMineStatusFilterMenu => {
            if app.expert_hub.skill_tab == SkillTab::Mine {
                app.expert_hub.toggle_mine_filter_menu(MineFilterMenu::Statuses);
            }
            Task::none()
        }
        SkillHubMessage::CloseMineFilterMenu => {
            app.expert_hub.close_mine_filter_menu();
            Task::none()
        }
        SkillHubMessage::MineTagToggled(tag_id) => {
            if app.expert_hub.skill_tab == SkillTab::Mine {
                let option_order = mine_tag_option_order(app);
                app.expert_hub.toggle_mine_tag(tag_id, &option_order);
            }
            Task::none()
        }
        SkillHubMessage::MineCompanyTagToggled(tag_id) => {
            if app.expert_hub.skill_tab == SkillTab::Mine {
                let option_order = mine_company_tag_option_order(app);
                app.expert_hub.toggle_mine_company_tag(tag_id, &option_order);
            }
            Task::none()
        }
        SkillHubMessage::MineStatusToggled(status) => {
            if app.expert_hub.skill_tab == SkillTab::Mine {
                app.expert_hub.toggle_mine_status(status);
            }
            Task::none()
        }
        SkillHubMessage::FiltersSubmitted => {
            match app.expert_hub.skill_tab {
                SkillTab::System => {
                    app.expert_hub.system_filter.submit();
                    app.expert_hub.system_page = 1;
                }
                SkillTab::Mine => {
                    app.expert_hub.submit_mine_filters();
                    app.expert_hub.mine_page = 1;
                }
                SkillTab::Marketplace | SkillTab::Installed | SkillTab::Install => {
                    return Task::none();
                }
            }
            load_current_tab(app)
        }
        SkillHubMessage::FiltersReset => {
            match app.expert_hub.skill_tab {
                SkillTab::Marketplace => {
                    app.expert_hub.search_draft.clear();
                    app.expert_hub.submitted_search.clear();
                    app.expert_hub.select_market_tag(None);
                    app.expert_hub.select_market_company_tag(None);
                }
                SkillTab::System => {
                    app.expert_hub.system_filter.reset();
                    app.expert_hub.system_page = 1;
                }
                SkillTab::Mine => {
                    app.expert_hub.reset_mine_filters();
                    app.expert_hub.mine_page = 1;
                }
                SkillTab::Installed | SkillTab::Install => return Task::none(),
            }
            load_current_tab(app)
        }
        SkillHubMessage::PreviousPage => {
            let changed = match app.expert_hub.skill_tab {
                SkillTab::System if app.expert_hub.system_page > 1 => {
                    app.expert_hub.system_page -= 1;
                    true
                }
                SkillTab::Mine if app.expert_hub.mine_page > 1 => {
                    app.expert_hub.mine_page -= 1;
                    true
                }
                _ => false,
            };
            if changed {
                app.expert_hub.close_mine_filter_menu();
                load_current_tab(app)
            } else {
                Task::none()
            }
        }
        SkillHubMessage::NextPage => {
            let changed = match app.expert_hub.skill_tab {
                SkillTab::System if page_has_next(&app.expert_hub.system) => {
                    app.expert_hub.system_page = app.expert_hub.system_page.saturating_add(1);
                    true
                }
                SkillTab::Mine if page_has_next(&app.expert_hub.mine) => {
                    app.expert_hub.mine_page = app.expert_hub.mine_page.saturating_add(1);
                    true
                }
                _ => false,
            };
            if changed {
                app.expert_hub.close_mine_filter_menu();
                load_current_tab(app)
            } else {
                Task::none()
            }
        }
        SkillHubMessage::Retry | SkillHubMessage::ReloadAfterAccountChange => {
            if app.screen == HubPage::ExpertHub
                && app.expert_hub.section == ExpertHubSection::Connectors
            {
                app.effect(Effect::ReloadConnectors)
            } else if app.screen == HubPage::ExpertHub
                && app.expert_hub.section == ExpertHubSection::Knowledge
            {
                app.effect(Effect::ReloadKnowledge)
            } else if app.expert_hub.install_guide.open {
                load_agent_package(app, false)
            } else {
                load_current_tab(app)
            }
        }
        SkillHubMessage::EnsureInstalledSkillsLoaded => load_installed_skills(app, false),
        SkillHubMessage::RefreshInstalledSkills => {
            reload_local_shiju_package_version(app);
            Task::batch([load_installed_skills(app, true), refresh_shiju_package_version(app)])
        }
        SkillHubMessage::OpenSkillDetail(skill_id) => open_skill_detail(app, skill_id),
        SkillHubMessage::CloseSkillDetail => {
            app.expert_hub.detail = None;
            app.expert_hub.cancel_version_action();
            Task::none()
        }
        SkillHubMessage::SetSkillReadmeMode(mode) => {
            if let Some(detail) = app.expert_hub.detail.as_mut() {
                detail.readme_mode = mode;
            }
            Task::none()
        }
        SkillHubMessage::SkillDetailLoaded { generation, account_id, skill_id, result } => {
            if !detail_response_is_current(app, generation, &account_id, skill_id) {
                return Task::none();
            }
            match result {
                Ok(skill_detail) => {
                    let canonical_skill_id = skill_detail.id;
                    if let Some(detail) = app.expert_hub.detail.as_mut() {
                        detail.detail = LoadState::Loaded(skill_detail);
                    }
                    let Some(account) = app.active_account else {
                        return Task::none();
                    };
                    let credential = account.skill_hub_credential();
                    Task::batch([
                        load_skill_versions(app.gateway,
                            credential.clone(),
                            generation,
                            account_id.clone(),
                            skill_id,
                            canonical_skill_id,
                        ),
                        load_skill_readme(app.gateway,
                            credential,
                            generation,
                            account_id,
                            skill_id,
                            canonical_skill_id,
                        ),
                    ])
                }
                Err(error) => {
                    if let Some(detail) = app.expert_hub.detail.as_mut() {
                        detail.detail = LoadState::Error(error);
                    }
                    Task::none()
                }
            }
        }
        SkillHubMessage::SkillReadmeLoaded { generation, account_id, skill_id, result } => {
            if detail_response_is_current(app, generation, &account_id, skill_id)
                && let Some(detail) = app.expert_hub.detail.as_mut()
            {
                match result {
                    Ok(content) => {
                        detail.readme_items = parse_public_markdown(&content);
                        detail.readme = LoadState::Loaded(content);
                    }
                    Err(error) => {
                        detail.readme_items.clear();
                        detail.readme = LoadState::Error(error);
                    }
                }
            }
            Task::none()
        }
        SkillHubMessage::SkillVersionsLoaded { generation, account_id, skill_id, result } => {
            if detail_response_is_current(app, generation, &account_id, skill_id)
                && let Some(detail) = app.expert_hub.detail.as_mut()
            {
                detail.versions =
                    result.map(LoadState::Loaded).unwrap_or_else(|error| LoadState::Error(error));
            }
            Task::none()
        }
        SkillHubMessage::InstallSkill(skill_id) => {
            start_skill_action(app, skill_id, SkillActionKind::Install)
        }
        SkillHubMessage::UninstallSkill(skill_id) => {
            start_skill_action(app, skill_id, SkillActionKind::Uninstall)
        }
        SkillHubMessage::UpgradeSkill(skill_id) => start_quick_upgrade(app, skill_id),
        SkillHubMessage::ChangeSkillVersion(action) => start_version_action(app, action),
        SkillHubMessage::CancelSkillVersionChange => {
            app.expert_hub.cancel_version_action();
            Task::none()
        }
        SkillHubMessage::ConfirmSkillVersionChange => confirm_version_action(app),
        SkillHubMessage::SkillUpgradeResolved { generation, account_id, skill_id, result } => {
            finish_quick_upgrade_resolution(app, generation, &account_id, skill_id, result)
        }
        SkillHubMessage::SkillActionFinished {
            generation,
            account_id,
            skill_id,
            action,
            result,
        } => finish_skill_action(app, generation, &account_id, skill_id, action, result),
        SkillHubMessage::SkillCacheSyncFinished { generation, account_id, skill_id, result } => {
            finish_skill_cache_sync(app, generation, &account_id, skill_id, result)
        }
        SkillHubMessage::RetrySkillCacheSync => retry_skill_cache_sync(app),
        SkillHubMessage::DownloadVersion(version_id) => {
            match dispatch_download_with(app.expert_hub, version_id, |url| {
                open::that(url)
                    .map_err(|_| SkillHubError::localized("无法打开技能下载地址", vec![]))
            }) {
                Ok(()) => app.show_localized_toast(
                    ToastKind::Info,
                    "已在系统浏览器中打开技能下载",
                    vec![],
                ),
                Err(error) => {
                    show_skill_error(app, ToastKind::Error, None, error.into())
                }
            }
        }
        SkillHubMessage::MarketplaceLoaded { generation, account_id, result } => {
            if response_is_current(app, generation, &account_id) {
                app.expert_hub.marketplace =
                    result.map(LoadState::Loaded).unwrap_or_else(|error| LoadState::Error(error));
            }
            Task::none()
        }
        SkillHubMessage::InstalledLoaded { generation, account_id, result } => {
            if installed_response_is_current(app, generation, &account_id) {
                app.expert_hub.installed =
                    result.map(LoadState::Loaded).unwrap_or_else(|error| LoadState::Error(error));
            }
            Task::none()
        }
        SkillHubMessage::SystemLoaded { generation, account_id, result } => {
            if response_is_current(app, generation, &account_id) {
                app.expert_hub.system =
                    result.map(LoadState::Loaded).unwrap_or_else(|error| LoadState::Error(error));
            }
            Task::none()
        }
        SkillHubMessage::MineLoaded { generation, account_id, result } => {
            if response_is_current(app, generation, &account_id) {
                app.expert_hub.mine =
                    result.map(LoadState::Loaded).unwrap_or_else(|error| LoadState::Error(error));
            }
            Task::none()
        }
        SkillHubMessage::PlatformTagCatalogLoaded { generation, account_id, result } => {
            if tag_catalog_response_is_current(app, generation, &account_id) {
                app.expert_hub.platform_tag_catalog =
                    result.map(LoadState::Loaded).unwrap_or_else(|error| LoadState::Error(error));
            }
            Task::none()
        }
        SkillHubMessage::CompanyTagCatalogLoaded { generation, account_id, result } => {
            if tag_catalog_response_is_current(app, generation, &account_id) {
                app.expert_hub.company_tag_catalog =
                    result.map(LoadState::Loaded).unwrap_or_else(|error| LoadState::Error(error));
            }
            Task::none()
        }
    }
}

/// 打开或激活专家中心标签，并保留当前分区选择。
pub fn show_skill_error(
    app: &mut SkillUpdate<'_>,
    kind: ToastKind,
    source: Option<&str>,
    error: SkillHubError,
) -> Task<Message> {
    let message = match source {
        Some(source) => {
            desktop_i18n::format::Message::nested(source.to_string(), vec![error.message()])
        }
        None => error.message(),
    };
    app.feedback(kind, message)
}

#[cfg(test)]
#[path = "skill_hub_tests.rs"]
mod tests;
