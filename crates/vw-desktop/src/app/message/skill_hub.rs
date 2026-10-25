//! 协调识矩技能列表、详情、安装、版本操作与本地缓存同步消息。

use crate::app::session_gateway::{
    gateway_shiju_maintained_skills_async, gateway_shiju_skill_detail_async,
    gateway_shiju_skill_installed_async, gateway_shiju_skill_readme_async,
    gateway_shiju_skill_remote_install_async, gateway_shiju_skill_remote_install_version_async,
    gateway_shiju_skill_remote_uninstall_async, gateway_shiju_skill_search_async,
    gateway_shiju_skill_tags_async, gateway_shiju_skill_versions_async,
    gateway_shiju_system_skills_async, gateway_shiju_tenant_skill_tags_async,
};
use crate::app::skill_hub::{
    CursorPage, ExpertHubSection, InstalledSkill, LoadState, MarketplaceSkill, MineFilterMenu,
    MineSkillStatus, PackageVersionCacheEntry, Page, PublicDetailMode, ShijuDirectInstallState,
    ShijuInstallGuideState, ShijuInstallationsReport, ShijuKeySyncOutcome, ShijuKeySyncReport,
    ShijuMultiInstallReport, ShijuPackageId, ShijuPackageRelease, ShijuUninstallReport,
    SkillActionKind, SkillCacheSyncReport, SkillDetail, SkillDetailState, SkillHubClient,
    SkillHubError, SkillInstallScope, SkillInstallSelection, SkillInstallTarget, SkillTab,
    SkillTag, SkillVersion, SkillVersionAction, SkillVersionDirection, SystemSkill, TenantSkill,
    install_shiju_archive_for_targets, package_version_cache_path, parse_public_markdown,
    resolve_skill_destinations, shiju_installation, shiju_installations_for_targets,
    sync_business_skill_cache, sync_shiju_key_for_targets, uninstall_shiju_for_targets,
    write_package_version_cache,
};
use crate::app::{App, AppTab, Message, Screen};
use directories::BaseDirs;
use iced::Task;
use reqwest::redirect::Policy;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::NamedTempFile;
use vw_gateway_client::{ShijuMaintainedSkillQuery, ShijuSkillSearchQuery, ShijuSystemSkillQuery};

mod public;
#[cfg(test)]
mod public_tests;
pub(crate) use public::PublicSkillMessage;

/// 专家中心在主标签栏中的稳定标识。
pub(crate) const EXPERT_HUB_TAB_ID: &str = "expert_hub";
/// 专家中心标签的用户可见标题。
pub(crate) const EXPERT_HUB_TITLE: &str = "专家·技能·连接器";
/// 识矩技能兼容入口的稳定标签标识。
pub(crate) const SHIJU_SKILLS_TAB_ID: &str = "shiju_skills";
/// 识矩技能兼容入口的用户可见标题。
pub(crate) const SHIJU_SKILLS_TITLE: &str = "识矩技能";
const SHIJU_PAGE_SIZE: u32 = 20;
const MAX_SHIJU_ARCHIVE_BYTES: u64 = 100 * 1024 * 1024;
const SHIJU_INSTALL_TARGETS: [SkillInstallTarget; 10] = [
    SkillInstallTarget::VibeWindow,
    SkillInstallTarget::AgentSkills,
    SkillInstallTarget::Codex,
    SkillInstallTarget::WorkBuddy,
    SkillInstallTarget::ClaudeCode,
    SkillInstallTarget::OpenCode,
    SkillInstallTarget::GeminiCli,
    SkillInstallTarget::GitHubCopilot,
    SkillInstallTarget::Cursor,
    SkillInstallTarget::Windsurf,
];

#[derive(Debug, Clone)]
#[allow(dead_code)]
/// 专家中心技能目录、安装向导、版本操作与公共市场的消息集合。
pub(crate) enum SkillHubMessage {
    Public(public::PublicSkillMessage),
    Open,
    OpenShijuSkills,
    OpenInstallGuide,
    CloseInstallGuide,
    CopyAgentInstall,
    AgentPackageLoaded {
        generation: u64,
        account_id: String,
        copy_after_load: bool,
        result: Result<ShijuPackageRelease, SkillHubError>,
    },
    DownloadShijuPackage(ShijuPackageId),
    ManualPackageLoaded {
        generation: u64,
        account_id: String,
        package_id: ShijuPackageId,
        result: Result<ShijuPackageRelease, SkillHubError>,
    },
    DirectInstallShiju,
    SelectShijuInstallScope(SkillInstallScope),
    SelectShijuInstallProject(String),
    ToggleShijuInstallTarget(SkillInstallTarget),
    SyncShijuKey,
    UninstallShiju,
    CancelShijuUninstall,
    ConfirmShijuUninstall,
    OpenShijuInstallDir,
    OpenShijuInstallDirFor(SkillInstallTarget),
    ShijuInstallationsLoaded {
        generation: u64,
        selections: Vec<SkillInstallSelection>,
        result: Result<ShijuInstallationsReport, String>,
    },
    DirectPackageLoaded {
        generation: u64,
        account_id: String,
        package_id: ShijuPackageId,
        selections: Vec<SkillInstallSelection>,
        project_root: Option<PathBuf>,
        result: Result<ShijuPackageRelease, SkillHubError>,
    },
    ShijuInstallFinished {
        generation: u64,
        account_id: String,
        package_id: ShijuPackageId,
        selections: Vec<SkillInstallSelection>,
        result: Result<ShijuMultiInstallReport, String>,
    },
    ShijuKeySyncFinished {
        generation: u64,
        account_id: String,
        selections: Vec<SkillInstallSelection>,
        requested_by_user: bool,
        result: Result<ShijuKeySyncReport, String>,
    },
    ShijuUninstallFinished {
        generation: u64,
        selections: Vec<SkillInstallSelection>,
        result: Result<ShijuUninstallReport, String>,
    },
    ShijuPackageVersionRefreshed {
        generation: u64,
        result: Result<PackageVersionCacheEntry, String>,
    },
    SelectSection(ExpertHubSection),
    SelectSkillTab(SkillTab),
    SearchDraftChanged(String),
    SearchSubmitted,
    MarketTagSelected(Option<u64>),
    MarketCompanyTagSelected(Option<u64>),
    FilterKeywordChanged(String),
    FilterTagChanged(Option<u64>),
    ToggleMineTagFilterMenu,
    ToggleMineCompanyTagFilterMenu,
    ToggleMineStatusFilterMenu,
    CloseMineFilterMenu,
    MineTagToggled(u64),
    MineCompanyTagToggled(u64),
    MineStatusToggled(MineSkillStatus),
    FiltersSubmitted,
    FiltersReset,
    PreviousPage,
    NextPage,
    Retry,
    ReloadAfterAccountChange,
    EnsureInstalledSkillsLoaded,
    RefreshInstalledSkills,
    OpenSkillDetail(u64),
    CloseSkillDetail,
    SetSkillReadmeMode(PublicDetailMode),
    SkillDetailLoaded {
        generation: u64,
        account_id: String,
        skill_id: u64,
        result: Result<SkillDetail, SkillHubError>,
    },
    SkillReadmeLoaded {
        generation: u64,
        account_id: String,
        skill_id: u64,
        result: Result<String, SkillHubError>,
    },
    SkillVersionsLoaded {
        generation: u64,
        account_id: String,
        skill_id: u64,
        result: Result<Vec<SkillVersion>, SkillHubError>,
    },
    InstallSkill(u64),
    UninstallSkill(u64),
    UpgradeSkill(u64),
    ChangeSkillVersion(SkillVersionAction),
    CancelSkillVersionChange,
    ConfirmSkillVersionChange,
    SkillUpgradeResolved {
        generation: u64,
        account_id: String,
        skill_id: u64,
        result: Result<SkillDetail, String>,
    },
    SkillActionFinished {
        generation: u64,
        account_id: String,
        skill_id: u64,
        action: SkillActionKind,
        result: Result<(), String>,
    },
    SkillCacheSyncFinished {
        generation: u64,
        account_id: String,
        skill_id: u64,
        result: Result<SkillCacheSyncReport, String>,
    },
    RetrySkillCacheSync,
    DownloadVersion(u64),
    MarketplaceLoaded {
        generation: u64,
        account_id: String,
        result: Result<CursorPage<MarketplaceSkill>, SkillHubError>,
    },
    InstalledLoaded {
        generation: u64,
        account_id: String,
        result: Result<Vec<InstalledSkill>, SkillHubError>,
    },
    SystemLoaded {
        generation: u64,
        account_id: String,
        result: Result<Page<SystemSkill>, SkillHubError>,
    },
    MineLoaded {
        generation: u64,
        account_id: String,
        result: Result<Page<TenantSkill>, SkillHubError>,
    },
    PlatformTagCatalogLoaded {
        generation: u64,
        account_id: String,
        result: Result<Vec<SkillTag>, SkillHubError>,
    },
    CompanyTagCatalogLoaded {
        generation: u64,
        account_id: String,
        result: Result<Vec<SkillTag>, SkillHubError>,
    },
}

/// 归约技能中心消息，并用请求代次与账号 ID 屏蔽迟到的异步响应。
pub(crate) fn update(app: &mut App, message: SkillHubMessage) -> Task<Message> {
    match message {
        SkillHubMessage::Public(message) => public::update(app, message),
        SkillHubMessage::Open => {
            app.expert_hub.reset_navigation();
            open_expert_hub_tab(app);
            public::ensure_loaded(app)
        }
        SkillHubMessage::OpenShijuSkills => {
            app.expert_hub.install_guide.close();
            app.expert_hub.skill_tab = SkillTab::Marketplace;
            app.expert_hub.detail = None;
            open_shiju_skills_tab(app);
            load_current_tab(app)
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
                let suites = public::update(
                    app,
                    public::PublicSkillMessage::SelectTab(
                        crate::app::skill_hub::PublicSkillTab::Suites,
                    ),
                );
                Task::batch([suites, public::ensure_loaded(app)])
            } else if section == ExpertHubSection::Skills {
                public::ensure_loaded(app)
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
            if app.expert_hub.install_guide.open {
                load_agent_package(app, false)
            } else {
                load_current_tab(app)
            }
        }
        SkillHubMessage::EnsureInstalledSkillsLoaded => load_installed_skills(app, false),
        SkillHubMessage::RefreshInstalledSkills => load_installed_skills(app, true),
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
                    let Some(account) = app.account.active_account() else {
                        return Task::none();
                    };
                    let credential = account.skill_hub_credential();
                    Task::batch([
                        load_skill_versions(
                            credential.clone(),
                            generation,
                            account_id.clone(),
                            skill_id,
                            canonical_skill_id,
                        ),
                        load_skill_readme(
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
                        detail.detail = LoadState::Error(error.to_string());
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
                        detail.readme = LoadState::Error(error.to_string());
                    }
                }
            }
            Task::none()
        }
        SkillHubMessage::SkillVersionsLoaded { generation, account_id, skill_id, result } => {
            if detail_response_is_current(app, generation, &account_id, skill_id)
                && let Some(detail) = app.expert_hub.detail.as_mut()
            {
                detail.versions = result
                    .map(LoadState::Loaded)
                    .unwrap_or_else(|error| LoadState::Error(error.to_string()));
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
            match dispatch_download_with(app, version_id, |url| {
                open::that(url).map_err(|_| "无法打开技能下载地址".to_string())
            }) {
                Ok(()) => app.show_info_toast("已在系统浏览器中打开技能下载"),
                Err(error) => app.show_error_toast(error),
            }
        }
        SkillHubMessage::MarketplaceLoaded { generation, account_id, result } => {
            if response_is_current(app, generation, &account_id) {
                app.expert_hub.marketplace = result
                    .map(LoadState::Loaded)
                    .unwrap_or_else(|error| LoadState::Error(error.to_string()));
            }
            Task::none()
        }
        SkillHubMessage::InstalledLoaded { generation, account_id, result } => {
            if installed_response_is_current(app, generation, &account_id) {
                app.expert_hub.installed = result
                    .map(LoadState::Loaded)
                    .unwrap_or_else(|error| LoadState::Error(error.to_string()));
            }
            Task::none()
        }
        SkillHubMessage::SystemLoaded { generation, account_id, result } => {
            if response_is_current(app, generation, &account_id) {
                app.expert_hub.system = result
                    .map(LoadState::Loaded)
                    .unwrap_or_else(|error| LoadState::Error(error.to_string()));
            }
            Task::none()
        }
        SkillHubMessage::MineLoaded { generation, account_id, result } => {
            if response_is_current(app, generation, &account_id) {
                app.expert_hub.mine = result
                    .map(LoadState::Loaded)
                    .unwrap_or_else(|error| LoadState::Error(error.to_string()));
            }
            Task::none()
        }
        SkillHubMessage::PlatformTagCatalogLoaded { generation, account_id, result } => {
            if tag_catalog_response_is_current(app, generation, &account_id) {
                app.expert_hub.platform_tag_catalog = result
                    .map(LoadState::Loaded)
                    .unwrap_or_else(|error| LoadState::Error(error.to_string()));
            }
            Task::none()
        }
        SkillHubMessage::CompanyTagCatalogLoaded { generation, account_id, result } => {
            if tag_catalog_response_is_current(app, generation, &account_id) {
                app.expert_hub.company_tag_catalog = result
                    .map(LoadState::Loaded)
                    .unwrap_or_else(|error| LoadState::Error(error.to_string()));
            }
            Task::none()
        }
    }
}

/// 打开或激活专家中心标签，并保留当前分区选择。
pub(crate) fn open_expert_hub_tab(app: &mut App) {
    if !app.open_tabs.iter().any(|tab| tab.id == EXPERT_HUB_TAB_ID) {
        app.open_tabs.push(AppTab {
            id: EXPERT_HUB_TAB_ID.to_string(),
            title: EXPERT_HUB_TITLE.to_string(),
            screen: Screen::ExpertHub,
            project_path: None,
        });
    }
    app.active_tab_id = Some(EXPERT_HUB_TAB_ID.to_string());
    app.screen = Screen::ExpertHub;
}

fn open_shiju_skills_tab(app: &mut App) {
    if !app.open_tabs.iter().any(|tab| tab.id == SHIJU_SKILLS_TAB_ID) {
        app.open_tabs.push(AppTab {
            id: SHIJU_SKILLS_TAB_ID.to_string(),
            title: SHIJU_SKILLS_TITLE.to_string(),
            screen: Screen::ShijuSkills,
            project_path: None,
        });
    }
    app.active_tab_id = Some(SHIJU_SKILLS_TAB_ID.to_string());
    app.screen = Screen::ShijuSkills;
}

fn open_install_guide(app: &mut App) -> Task<Message> {
    app.account.menu_open = false;
    app.account.details_open = false;
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

fn start_installation_load(app: &mut App) -> Task<Message> {
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
                    .map_err(|error| error.to_string())
            })
            .await
            .map_err(|_| "识矩技能安装状态检查任务异常终止".to_string())?
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

fn finish_installation_load(
    app: &mut App,
    generation: u64,
    _selections: &[SkillInstallSelection],
    result: Result<ShijuInstallationsReport, String>,
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
            app.show_error_toast(format!("无法检查识矩技能安装状态：{error}"))
        }
    }
}

fn all_install_selections(include_project: bool) -> Vec<SkillInstallSelection> {
    let mut selections = Vec::with_capacity(if include_project {
        SHIJU_INSTALL_TARGETS.len() * 2
    } else {
        SHIJU_INSTALL_TARGETS.len()
    });
    for target in SHIJU_INSTALL_TARGETS {
        selections.push(SkillInstallSelection { target, scope: SkillInstallScope::Global });
        if include_project {
            selections.push(SkillInstallSelection { target, scope: SkillInstallScope::Project });
        }
    }
    selections
}

fn home_directory() -> Result<PathBuf, String> {
    BaseDirs::new()
        .map(|directories| directories.home_dir().to_path_buf())
        .ok_or_else(|| "无法确定用户主目录".to_string())
}

fn selected_project_root(
    app: &App,
    selections: &[SkillInstallSelection],
) -> Result<Option<PathBuf>, String> {
    if selections.iter().any(|selection| selection.scope == SkillInstallScope::Project) {
        let project = app
            .expert_hub
            .install_guide
            .selected_project_path
            .as_deref()
            .filter(|path| !path.trim().is_empty())
            .ok_or_else(|| "项目级技能安装需要选择项目".to_string())?;
        return Ok(Some(PathBuf::from(project)));
    }
    Ok(None)
}

fn select_shiju_install_scope(app: &mut App, scope: SkillInstallScope) -> Task<Message> {
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
            return app.show_error_toast("项目级技能安装需要选择项目");
        }
    }
    if app.expert_hub.install_guide.install_scope == scope {
        return Task::none();
    }
    if app.expert_hub.install_guide.is_busy() {
        return app.show_warning_toast("请等待当前安装、卸载或同步操作完成");
    }
    app.expert_hub.install_guide.select_scope(scope);
    Task::none()
}

fn select_shiju_install_project(app: &mut App, project_path: String) -> Task<Message> {
    if app.expert_hub.install_guide.is_busy() {
        return app.show_warning_toast("请等待当前安装、卸载或同步操作完成");
    }
    let project_path = project_path.trim();
    if project_path.is_empty()
        || !app
            .project_path
            .iter()
            .chain(app.recent_projects.iter())
            .any(|candidate| candidate.trim() == project_path)
    {
        return app.show_error_toast("请选择已打开或最近使用的项目");
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

fn toggle_shiju_install_target(app: &mut App, target: SkillInstallTarget) -> Task<Message> {
    if app.expert_hub.install_guide.is_busy() {
        return app.show_warning_toast("请等待当前安装、卸载或同步操作完成");
    }
    let is_selected = app.expert_hub.install_guide.selected_targets.contains(&target);
    if is_selected && app.expert_hub.install_guide.selected_targets.len() == 1 {
        return app.show_warning_toast("至少选择一个技能安装目标");
    }
    app.expert_hub.install_guide.toggle_target(target);
    Task::none()
}

fn open_selected_shiju_install_dir(app: &mut App) -> Task<Message> {
    let target = app.expert_hub.install_guide.selected_targets.iter().copied().find(|target| {
        app.expert_hub
            .install_guide
            .installation(app.expert_hub.install_guide.install_scope, *target)
            .is_some()
    });
    let Some(target) = target else {
        return app.show_warning_toast("所选目标尚未安装识矩技能");
    };
    open_shiju_install_dir_for(app, target)
}

fn open_shiju_install_dir_for(app: &mut App, target: SkillInstallTarget) -> Task<Message> {
    let Some(directory) = app
        .expert_hub
        .install_guide
        .installation(app.expert_hub.install_guide.install_scope, target)
        .map(|installation| installation.installed_dir.clone())
    else {
        return app.show_warning_toast("指定目标尚未安装识矩技能");
    };
    match open_shiju_install_dir_with(&directory, |path| {
        open::that(path).map_err(|_| "无法打开识矩技能目录".to_string())
    }) {
        Ok(()) => app.show_info_toast("已打开识矩技能目录"),
        Err(error) => app.show_error_toast(error),
    }
}

fn active_shiju_account(app: &mut App) -> Option<(String, crate::app::account::SecretSkey)> {
    let active = app
        .account
        .active_account()
        .map(|account| (account.id().to_string(), account.skill_hub_credential()));
    let Some((account_id, credential)) = active else {
        app.expert_hub.install_guide.agent_package = LoadState::NeedsLogin;
        return None;
    };
    if app.expert_hub.account_id.as_deref() != Some(account_id.as_str()) {
        app.expert_hub.clear_for_account(Some(account_id.clone()));
    }
    Some((account_id, credential))
}

fn load_agent_package(app: &mut App, copy_after_load: bool) -> Task<Message> {
    let Some(package_id) = app.expert_hub.install_guide.current_package else {
        app.expert_hub.install_guide.agent_package =
            LoadState::Error("当前系统或架构暂不支持自动安装".to_string());
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

fn finish_agent_package_load(
    app: &mut App,
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
            let text = agent_install_text(&app.expert_hub.install_guide);
            if copy_after_load {
                Task::batch([
                    Task::done(Message::CopyCode(text)),
                    app.show_success_toast("安装文字已复制"),
                ])
            } else {
                Task::none()
            }
        }
        Err(error) => {
            app.expert_hub.install_guide.agent_package = LoadState::Error(error.to_string());
            if copy_after_load {
                app.show_error_toast("安装文字准备失败，请稍后重试")
            } else {
                Task::none()
            }
        }
    }
}

/// 返回适合复制给代理的安装命令；尚未加载包信息时返回空文本。
pub(crate) fn agent_install_text(state: &ShijuInstallGuideState) -> String {
    match &state.agent_package {
        LoadState::Loaded(release) => agent_install_text_for_release(release),
        LoadState::Loading { .. } => "正在生成当前设备的安装地址…".to_string(),
        LoadState::NeedsLogin => "登录识矩账户后即可生成安装文字。".to_string(),
        LoadState::Error(_) => "安装地址生成失败，请稍后重试。".to_string(),
        LoadState::Idle => "正在准备安装文字…".to_string(),
    }
}

fn agent_install_text_for_release(release: &ShijuPackageRelease) -> String {
    format!(
        "请帮我安装识矩技能。\n\n下载地址：{}\n\n请下载该 zip，将解压后的技能包放入当前 Agent 的技能目录。",
        release.download_url.as_str()
    )
}

fn start_manual_download(app: &mut App, package_id: ShijuPackageId) -> Task<Message> {
    let Some((account_id, credential)) = active_shiju_account(app) else {
        return app.show_warning_toast("登录识矩账户后下载安装包");
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

fn finish_manual_download(
    app: &mut App,
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
            open::that(url).map_err(|_| "无法打开识矩技能下载地址".to_string())
        }) {
            Ok(()) => app.show_info_toast("已在系统浏览器中打开安装包"),
            Err(error) => app.show_error_toast(error),
        },
        Err(error) => app.show_error_toast(format!("安装包地址生成失败：{error}")),
    }
}

/// 将识矩包流式传给受限写盘任务，并限制累计下载大小。
pub(super) fn dispatch_shiju_download_with(
    release: &ShijuPackageRelease,
    opener: impl FnOnce(&str) -> Result<(), String>,
) -> Result<(), String> {
    opener(release.download_url.as_str()).map_err(|_| "无法打开识矩技能下载地址".to_string())
}

pub(super) fn open_shiju_install_dir_with(
    directory: &Path,
    opener: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<(), String> {
    if !directory.is_dir() {
        return Err("识矩技能安装目录不存在".to_string());
    }
    opener(directory).map_err(|_| "无法打开识矩技能目录".to_string())
}

fn start_direct_install(app: &mut App) -> Task<Message> {
    let Some(package_id) = app.expert_hub.install_guide.current_package else {
        return app.show_error_toast("当前系统或架构暂不支持直接安装");
    };
    let Some((account_id, credential)) = active_shiju_account(app) else {
        return app.show_warning_toast("登录识矩账户后直接安装");
    };
    let selections = app.expert_hub.install_guide.selected_install_selections();
    let project_root = match selected_project_root(app, &selections) {
        Ok(project_root) => project_root,
        Err(error) => return app.show_error_toast(error),
    };
    let home = match home_directory() {
        Ok(home) => home,
        Err(error) => return app.show_error_toast(error),
    };
    if let Err(error) = resolve_skill_destinations(&home, project_root.as_deref(), &selections) {
        return app.show_error_toast(error.to_string());
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

fn finish_direct_package_load(
    app: &mut App,
    generation: u64,
    account_id: &str,
    package_id: ShijuPackageId,
    selections: Vec<SkillInstallSelection>,
    project_root: Option<PathBuf>,
    result: Result<ShijuPackageRelease, SkillHubError>,
) -> Task<Message> {
    let is_current = app.expert_hub.install_guide.open
        && app.expert_hub.account_id.as_deref() == Some(account_id)
        && selected_project_root(app, &selections).is_ok_and(|current| current == project_root)
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
            return app.show_error_toast(format!("直接安装准备失败：{error}"));
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

fn finish_direct_install(
    app: &mut App,
    generation: u64,
    account_id: &str,
    package_id: ShijuPackageId,
    selections: &[SkillInstallSelection],
    result: Result<ShijuMultiInstallReport, String>,
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
                app.show_success_toast(format!("识矩技能已安装到 {installed_count} 个目标")),
                start_shiju_key_sync(app, false),
            ])
        }
        Ok(_) => {
            app.expert_hub.install_guide.direct_install = ShijuDirectInstallState::Idle;
            app.show_error_toast("识矩技能安装结果不完整")
        }
        Err(error) => {
            app.expert_hub.install_guide.direct_install = ShijuDirectInstallState::Idle;
            app.show_error_toast(format!("识矩技能安装失败：{error}"))
        }
    }
}

async fn install_shiju_package_for_targets(
    release: ShijuPackageRelease,
    project_root: Option<PathBuf>,
    selections: Vec<SkillInstallSelection>,
) -> Result<ShijuMultiInstallReport, String> {
    let client = reqwest::Client::builder()
        .redirect(Policy::none())
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|_| "无法创建识矩技能下载客户端".to_string())?;
    let mut response = client
        .get(release.download_url.as_str())
        .send()
        .await
        .map_err(|_| "识矩技能安装包下载失败".to_string())?;
    if !response.status().is_success() {
        return Err(format!("识矩技能安装包下载失败: HTTP {}", response.status().as_u16()));
    }
    if response.content_length().is_some_and(|length| length > MAX_SHIJU_ARCHIVE_BYTES) {
        return Err("识矩技能安装包超过 100 MiB 限制".to_string());
    }

    let home = home_directory()?;
    let (chunk_sender, chunk_receiver) = tokio::sync::mpsc::channel(4);
    let install_task = tokio::task::spawn_blocking(move || {
        let archive =
            NamedTempFile::new().map_err(|_| "无法创建识矩技能临时下载文件".to_string())?;
        let archive = write_shiju_download_chunks(archive, chunk_receiver)?;
        install_shiju_archive_for_targets(
            archive.path(),
            &home,
            project_root.as_deref(),
            &selections,
        )
        .map_err(|error| error.to_string())
    });

    let mut written = 0u64;
    let mut download_error = None;
    loop {
        let chunk = match response.chunk().await {
            Ok(Some(chunk)) => chunk,
            Ok(None) => break,
            Err(_) => {
                download_error = Some("识矩技能安装包下载中断".to_string());
                break;
            }
        };
        let chunk_len = match u64::try_from(chunk.len())
            .map_err(|_| "识矩技能安装包大小溢出".to_string())
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
                download_error = Some("识矩技能安装包大小溢出".to_string());
                break;
            }
        };
        if written > MAX_SHIJU_ARCHIVE_BYTES {
            download_error = Some("识矩技能安装包超过 100 MiB 限制".to_string());
            break;
        }
        if chunk_sender.send(ShijuDownloadChunk::Data(chunk.to_vec())).await.is_err() {
            download_error = Some("识矩技能安装写盘任务异常终止".to_string());
            break;
        }
    }

    let completion = match download_error {
        Some(error) => ShijuDownloadChunk::Failed(error),
        None => ShijuDownloadChunk::Complete,
    };
    let _ = chunk_sender.send(completion).await;
    drop(chunk_sender);
    install_task.await.map_err(|_| "识矩技能安装任务异常终止".to_string())?
}

#[derive(Debug)]
/// 下载响应产生的数据块、完成标记或读取错误。
pub(super) enum ShijuDownloadChunk {
    Data(Vec<u8>),
    Complete,
    Failed(String),
}

/// 顺序写入下载数据块；超过上限或读取失败时停止并返回错误。
pub(super) fn write_shiju_download_chunks(
    mut archive: NamedTempFile,
    mut chunks: tokio::sync::mpsc::Receiver<ShijuDownloadChunk>,
) -> Result<NamedTempFile, String> {
    loop {
        match chunks.blocking_recv() {
            Some(ShijuDownloadChunk::Data(chunk)) => {
                archive
                    .write_all(&chunk)
                    .map_err(|_| "无法写入识矩技能临时下载文件".to_string())?;
            }
            Some(ShijuDownloadChunk::Complete) => {
                archive.flush().map_err(|_| "无法写完识矩技能临时下载文件".to_string())?;
                return Ok(archive);
            }
            Some(ShijuDownloadChunk::Failed(error)) => return Err(error),
            None => return Err("识矩技能安装包下载未完成".to_string()),
        }
    }
}

fn install_report_matches_selections(
    report: &ShijuMultiInstallReport,
    selections: &[SkillInstallSelection],
) -> bool {
    report.installs.len() == selections.len()
        && selections.iter().all(|selection| {
            report.installs.iter().any(|install| {
                install.destination.scope == selection.scope
                    && install.destination.target == selection.target
            })
        })
}

fn start_manual_shiju_key_sync(app: &mut App) -> Task<Message> {
    if app.expert_hub.install_guide.installed_selection_count() == 0 {
        return app.show_error_toast("识矩技能尚未安装，无法同步登录信息");
    }
    if !matches!(
        &app.expert_hub.install_guide.direct_install,
        ShijuDirectInstallState::Idle | ShijuDirectInstallState::Installed
    ) {
        return app.show_warning_toast("请等待识矩技能安装或卸载完成后再同步");
    }
    start_shiju_key_sync(app, true)
}

fn start_shiju_key_sync(app: &mut App, requested_by_user: bool) -> Task<Message> {
    let account = app
        .account
        .active_account()
        .map(|account| (account.id().to_string(), account.skill_hub_credential()));
    let Some((account_id, credential)) = account else {
        return if requested_by_user {
            app.show_warning_toast("登录识矩账户后同步登录信息")
        } else {
            Task::none()
        };
    };
    let selections = app.expert_hub.install_guide.selected_install_selections();
    let project_root = match selected_project_root(app, &selections) {
        Ok(project_root) => project_root,
        Err(error) => return app.show_error_toast(error),
    };
    let home = match home_directory() {
        Ok(home) => home,
        Err(error) => return app.show_error_toast(error),
    };
    let Some(operation) = app.expert_hub.install_guide.start_login_sync() else {
        return Task::none();
    };
    let result_selections = operation.selections.clone();
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                sync_shiju_key_for_targets(&home, project_root.as_deref(), &selections, &credential)
                    .map_err(|error| error.to_string())
            })
            .await
            .map_err(|_| "识矩登录信息同步任务异常终止".to_string())?
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

fn finish_shiju_key_sync(
    app: &mut App,
    generation: u64,
    account_id: &str,
    selections: &[SkillInstallSelection],
    requested_by_user: bool,
    result: Result<ShijuKeySyncReport, String>,
) -> Task<Message> {
    let is_current = app.expert_hub.install_guide.open
        && app.account.active_account().is_some_and(|account| account.id() == account_id)
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
                    app.show_error_toast("所选目标尚未安装识矩技能，无法同步登录信息")
                } else {
                    Task::none()
                };
            }
            if requested_by_user {
                app.show_success_toast(format!("识矩登录信息已同步到 {updated} 个目标"))
            } else {
                app.show_success_toast(format!("识矩登录信息已自动同步到 {updated} 个目标"))
            }
        }
        Err(_) => {
            if requested_by_user {
                app.show_error_toast("识矩登录信息同步失败，请检查文件权限后重试")
            } else {
                app.show_error_toast("识矩登录信息自动同步失败，请检查文件权限后手动重试")
            }
        }
    }
}

fn start_shiju_uninstall(app: &mut App) -> Task<Message> {
    let Some(generation) = app.expert_hub.install_guide.start_uninstall() else {
        return Task::none();
    };
    let selections = app.expert_hub.install_guide.direct_selections.clone();
    let project_root = match selected_project_root(app, &selections) {
        Ok(project_root) => project_root,
        Err(error) => {
            app.expert_hub.install_guide.direct_install = ShijuDirectInstallState::Installed;
            return app.show_error_toast(error);
        }
    };
    let home = match home_directory() {
        Ok(home) => home,
        Err(error) => {
            app.expert_hub.install_guide.direct_install = ShijuDirectInstallState::Installed;
            return app.show_error_toast(error);
        }
    };
    let result_selections = selections.clone();
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                uninstall_shiju_for_targets(&home, project_root.as_deref(), &selections)
                    .map_err(|error| error.to_string())
            })
            .await
            .map_err(|_| "识矩技能卸载任务异常终止".to_string())?
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

fn finish_shiju_uninstall(
    app: &mut App,
    generation: u64,
    selections: &[SkillInstallSelection],
    result: Result<ShijuUninstallReport, String>,
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
            app.show_success_toast(format!("已从 {removed} 个目标卸载识矩技能"))
        }
        Err(error) => {
            app.expert_hub.install_guide.direct_install = ShijuDirectInstallState::Installed;
            app.show_error_toast(format!("识矩技能卸载失败：{error}"))
        }
    }
}

fn open_skill_detail(app: &mut App, skill_id: u64) -> Task<Message> {
    let Some(account) = app.account.active_account() else {
        return app.show_error_toast("登录识矩账户后查看技能详情");
    };
    let account_id = account.id().to_string();
    let credential = account.skill_hub_credential();
    if app.expert_hub.account_id.as_deref() != Some(account_id.as_str()) {
        app.expert_hub.clear_for_account(Some(account_id.clone()));
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
            let dto = gateway_shiju_skill_detail_async(&credential, skill_id)
                .await
                .map_err(SkillHubError::from)?;
            SkillDetail::try_from(dto).map_err(SkillHubError::from)
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

fn load_skill_readme(
    credential: crate::app::account::SecretSkey,
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
            gateway_shiju_skill_readme_async(&credential, canonical_skill_id)
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

fn load_skill_versions(
    credential: crate::app::account::SecretSkey,
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
            gateway_shiju_skill_versions_async(&credential, canonical_skill_id)
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

fn detail_response_is_current(app: &App, generation: u64, account_id: &str, skill_id: u64) -> bool {
    app.expert_hub.account_id.as_deref() == Some(account_id)
        && app
            .expert_hub
            .detail
            .as_ref()
            .is_some_and(|detail| detail.generation == generation && detail.skill_id == skill_id)
}

fn start_skill_action(app: &mut App, skill_id: u64, action: SkillActionKind) -> Task<Message> {
    let Some(account) = app.account.active_account() else {
        return app.show_error_toast("登录识矩账户后管理技能");
    };
    let account_id = account.id().to_string();
    if app.expert_hub.account_id.as_deref() != Some(account_id.as_str()) {
        app.expert_hub.clear_for_account(Some(account_id.clone()));
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
                    gateway_shiju_skill_remote_install_async(&credential, skill_id).await
                }
                SkillActionKind::Uninstall => {
                    gateway_shiju_skill_remote_uninstall_async(&credential, skill_id).await
                }
                SkillActionKind::ResolveUpgrade { .. } | SkillActionKind::Version(_) => {
                    Err("无效的技能操作".to_string())
                }
            }
        },
        move |result| {
            Message::SkillHub(SkillHubMessage::SkillActionFinished {
                generation,
                account_id,
                skill_id,
                action,
                result,
            })
        },
    )
}

fn start_quick_upgrade(app: &mut App, skill_id: u64) -> Task<Message> {
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
    let Some(account) = app.account.active_account() else {
        return app.show_error_toast("登录识矩账户后管理技能");
    };
    let account_id = account.id().to_string();
    if app.expert_hub.account_id.as_deref() != Some(account_id.as_str()) {
        app.expert_hub.clear_for_account(Some(account_id.clone()));
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
            let dto = gateway_shiju_skill_detail_async(&credential, skill_id).await?;
            SkillDetail::try_from(dto).map_err(|error| error.to_string())
        },
        move |result| {
            Message::SkillHub(SkillHubMessage::SkillUpgradeResolved {
                generation,
                account_id,
                skill_id,
                result,
            })
        },
    )
}

fn finish_quick_upgrade_resolution(
    app: &mut App,
    generation: u64,
    account_id: &str,
    skill_id: u64,
    result: Result<SkillDetail, String>,
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
            return app.show_error_toast("技能升级失败：详情响应与目标技能不匹配");
        }
        Err(error) => {
            app.expert_hub.action = None;
            return app.show_error_toast(format!("技能升级失败：{error}"));
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
    let Some(account) = app.account.active_account() else {
        app.expert_hub.action = None;
        return app.show_error_toast("登录识矩账户后管理技能");
    };
    dispatch_version_action(
        account.skill_hub_credential(),
        generation,
        account_id.to_string(),
        action,
    )
}

fn start_version_action(app: &mut App, action: SkillVersionAction) -> Task<Message> {
    let Some(account) = app.account.active_account() else {
        return app.show_error_toast("登录识矩账户后管理技能");
    };
    let account_id = account.id().to_string();
    if app.expert_hub.account_id.as_deref() != Some(account_id.as_str()) {
        app.expert_hub.clear_for_account(Some(account_id.clone()));
    }
    let credential = account.skill_hub_credential();
    let direction = action.direction;
    let Some(generation) = app.expert_hub.request_version_action(action.clone()) else {
        return Task::none();
    };
    if direction == SkillVersionDirection::Rollback {
        return Task::none();
    }
    dispatch_version_action(credential, generation, account_id, action)
}

fn confirm_version_action(app: &mut App) -> Task<Message> {
    let Some(action) = app.expert_hub.version_confirmation.clone() else {
        return Task::none();
    };
    let Some(account) = app.account.active_account() else {
        return app.show_error_toast("登录识矩账户后管理技能");
    };
    let account_id = account.id().to_string();
    if app.expert_hub.account_id.as_deref() != Some(account_id.as_str()) {
        app.expert_hub.clear_for_account(Some(account_id));
        return Task::none();
    }
    let credential = account.skill_hub_credential();
    let Some(generation) = app.expert_hub.confirm_version_action() else {
        return Task::none();
    };
    dispatch_version_action(credential, generation, account_id, action)
}

fn dispatch_version_action(
    credential: crate::app::account::SecretSkey,
    generation: u64,
    account_id: String,
    action: SkillVersionAction,
) -> Task<Message> {
    let skill_id = action.skill_id;
    let request_action = action.clone();
    Task::perform(
        async move {
            let credential = credential.gateway_credential().map_err(|error| error.to_string())?;
            gateway_shiju_skill_remote_install_version_async(
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
                result,
            })
        },
    )
}

fn finish_skill_action(
    app: &mut App,
    generation: u64,
    account_id: &str,
    skill_id: u64,
    action: SkillActionKind,
    result: Result<(), String>,
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
                app.show_info_toast("服务端已更新，正在同步本地缓存"),
                start_skill_cache_sync(
                    app,
                    skill_id,
                    matches!(action, SkillActionKind::Version(_)),
                ),
            ])
        }
        Ok(()) => Task::batch([app.show_success_toast("技能卸载成功"), load_current_tab(app)]),
        Err(error) => {
            let label = match &action {
                SkillActionKind::Install => "技能安装失败",
                SkillActionKind::Uninstall => "技能卸载失败",
                SkillActionKind::ResolveUpgrade { .. } => "技能升级失败",
                SkillActionKind::Version(version) => match version.direction {
                    SkillVersionDirection::Upgrade => "技能升级失败",
                    SkillVersionDirection::Rollback => "技能回退失败",
                    SkillVersionDirection::Switch => "技能版本切换失败",
                },
            };
            app.show_error_toast(format!("{label}：{error}"))
        }
    }
}

fn start_skill_cache_sync(app: &mut App, skill_id: u64, refresh_detail: bool) -> Task<Message> {
    let Some(account_id) = app.account.active_account().map(|account| account.id().to_string())
    else {
        return app.show_warning_toast("登录识矩账户后重试本地缓存同步");
    };
    if app.expert_hub.account_id.as_deref() != Some(account_id.as_str()) {
        app.expert_hub.clear_for_account(Some(account_id));
        return Task::none();
    }
    let Some(generation) = app.expert_hub.start_cache_sync(skill_id, refresh_detail) else {
        return Task::none();
    };
    Task::perform(
        async move {
            let installation = tokio::task::spawn_blocking(shiju_installation)
                .await
                .map_err(|_| "主识矩技能安装状态检查异常终止".to_string())?
                .map_err(|_| "无法验证主识矩技能安装状态".to_string())?
                .ok_or_else(|| "未安装主识矩技能，请前往识矩技能页安装后重试".to_string())?;
            sync_business_skill_cache(&installation.installed_dir, skill_id)
                .await
                .map_err(|error| error.to_string())
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

fn retry_skill_cache_sync(app: &mut App) -> Task<Message> {
    let Some((skill_id, refresh_detail)) = app.expert_hub.cache_sync.pending_retry_target() else {
        return Task::none();
    };
    Task::batch([
        app.show_info_toast("正在重试本地缓存同步"),
        start_skill_cache_sync(app, skill_id, refresh_detail),
    ])
}

fn finish_skill_cache_sync(
    app: &mut App,
    generation: u64,
    account_id: &str,
    skill_id: u64,
    result: Result<SkillCacheSyncReport, String>,
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
            app.show_success_toast("技能与本地缓存同步成功"),
            refresh_skill_cache_views(app, skill_id, refresh_detail.unwrap_or(false)),
        ]),
        Err(error) => {
            app.show_warning_toast(format!("服务端已更新，本地缓存待同步：{error}，请重试"))
        }
    }
}

fn refresh_skill_cache_views(app: &mut App, skill_id: u64, refresh_detail: bool) -> Task<Message> {
    if refresh_detail {
        return refresh_version_action_views(app, skill_id);
    }
    Task::batch([load_marketplace_skills(app), load_installed_skills(app, true)])
}

fn refresh_version_action_views(app: &mut App, skill_id: u64) -> Task<Message> {
    let detail_task =
        if app.expert_hub.detail.as_ref().is_some_and(|detail| detail.skill_id == skill_id) {
            open_skill_detail(app, skill_id)
        } else {
            Task::none()
        };
    Task::batch([load_marketplace_skills(app), load_installed_skills(app, true), detail_task])
}

/// 下载技能版本到临时文件，并将完成结果映射回技能消息。
pub(super) fn dispatch_download_with(
    app: &App,
    version_id: u64,
    opener: impl FnOnce(&str) -> Result<(), String>,
) -> Result<(), String> {
    let version = app
        .expert_hub
        .detail
        .as_ref()
        .and_then(|detail| match &detail.versions {
            LoadState::Loaded(versions) => versions.iter().find(|version| version.id == version_id),
            _ => None,
        })
        .ok_or_else(|| "技能版本下载地址不可用".to_string())?;
    let download_url =
        version.download_url.as_ref().ok_or_else(|| "技能版本下载地址不可用".to_string())?;
    opener(download_url.as_str()).map_err(|_| "无法打开技能下载地址".to_string())
}

/// 账号切换后清除账号范围内的技能状态，并按当前页面重新加载。
pub(crate) fn account_changed(app: &mut App) -> Task<Message> {
    let account_id = app.account.active_account().map(|account| account.id().to_string());
    app.expert_hub.clear_for_account(account_id.clone());
    if account_id.is_none() {
        if app.expert_hub.install_guide.open {
            app.expert_hub.install_guide.agent_package = LoadState::NeedsLogin;
        } else {
            mark_current_needs_login(app);
        }
        Task::none()
    } else {
        let refresh = refresh_shiju_package_version(app);
        let reload = if app.screen == Screen::ShijuApps {
            Task::done(Message::ShijuApps(
                crate::app::message::ShijuAppsMessage::ReloadAfterAccountChange,
            ))
        } else if matches!(app.screen, Screen::ShijuSkills | Screen::ExpertHub) {
            Task::done(Message::SkillHub(SkillHubMessage::ReloadAfterAccountChange))
        } else {
            Task::none()
        };
        Task::batch([refresh, reload])
    }
}

fn refresh_shiju_package_version(app: &mut App) -> Task<Message> {
    let Some(account) = app.account.active_account() else {
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
                .map_err(|_| "无法创建识矩版本刷新客户端".to_string())?;
            let release = client
                .latest_shiju_package(package_id)
                .await
                .map_err(|_| "无法刷新识矩服务端版本".to_string())?;
            let version =
                release.version().ok_or_else(|| "识矩服务端版本格式无效".to_string())?.to_string();
            let checked_at_unix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| "无法记录识矩版本检测时间".to_string())?
                .as_secs();
            PackageVersionCacheEntry::new(version, checked_at_unix)
                .map_err(|_| "识矩服务端版本格式无效".to_string())
        },
        move |result| {
            Message::SkillHub(SkillHubMessage::ShijuPackageVersionRefreshed { generation, result })
        },
    )
}

fn finish_shiju_package_version_refresh(
    app: &mut App,
    generation: u64,
    result: Result<PackageVersionCacheEntry, String>,
) -> Task<Message> {
    if !app.expert_hub.package_status.is_refreshing(generation) {
        return Task::none();
    }
    let result = result.and_then(|entry| {
        let path =
            package_version_cache_path().map_err(|_| "无法确定识矩版本缓存路径".to_string())?;
        write_package_version_cache(&path, &entry)
            .map_err(|_| "无法更新识矩版本缓存".to_string())?;
        Ok(entry)
    });
    app.expert_hub.package_status.finish_refresh(generation, result);
    Task::none()
}

fn reload_local_shiju_package_version(app: &mut App) {
    let Ok(installation) = shiju_installation() else {
        return;
    };
    app.expert_hub
        .package_status
        .set_local_version(installation.and_then(|installation| installation.version));
}

fn page_has_next<T>(state: &LoadState<Page<T>>) -> bool {
    match state {
        LoadState::Loaded(page) => {
            u64::from(page.page).saturating_mul(u64::from(page.page_size)) < page.total
        }
        _ => true,
    }
}

fn response_is_current(app: &App, generation: u64, account_id: &str) -> bool {
    app.expert_hub.request_generation == generation
        && app.expert_hub.account_id.as_deref() == Some(account_id)
}

fn installed_response_is_current(app: &App, generation: u64, account_id: &str) -> bool {
    app.expert_hub.installed_request_generation == generation
        && app.expert_hub.account_id.as_deref() == Some(account_id)
}

fn tag_catalog_response_is_current(app: &App, generation: u64, account_id: &str) -> bool {
    app.expert_hub.tag_catalog_generation == generation
        && app.expert_hub.account_id.as_deref() == Some(account_id)
}

fn mark_current_needs_login(app: &mut App) {
    match app.expert_hub.skill_tab {
        SkillTab::Marketplace => app.expert_hub.marketplace = LoadState::NeedsLogin,
        SkillTab::Installed => app.expert_hub.installed = LoadState::NeedsLogin,
        SkillTab::System => app.expert_hub.system = LoadState::NeedsLogin,
        SkillTab::Mine => app.expert_hub.mine = LoadState::NeedsLogin,
        SkillTab::Install => app.expert_hub.install_guide.agent_package = LoadState::NeedsLogin,
    }
}

fn mine_tag_option_order(app: &App) -> Vec<u64> {
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

fn mine_company_tag_option_order(app: &App) -> Vec<u64> {
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
pub(super) fn mine_query(app: &App) -> ShijuMaintainedSkillQuery {
    let keyword = app.expert_hub.mine_filter.submitted_keyword.clone();
    ShijuMaintainedSkillQuery {
        page: app.expert_hub.mine_page,
        page_size: SHIJU_PAGE_SIZE,
        keyword: (!keyword.is_empty()).then_some(keyword),
        tag_ids: app.expert_hub.submitted_mine_tag_ids.clone(),
        company_tag_ids: app.expert_hub.submitted_mine_company_tag_ids.clone(),
        statuses: app
            .expert_hub
            .submitted_mine_statuses
            .iter()
            .map(|status| status.as_str().to_string())
            .collect(),
    }
}

fn load_current_tab(app: &mut App) -> Task<Message> {
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

    let active = app
        .account
        .active_account()
        .map(|account| (account.id().to_string(), account.skill_hub_credential()));
    let Some((account_id, credential)) = active else {
        if app.expert_hub.account_id.is_some() {
            app.expert_hub.clear_for_account(None);
        }
        mark_current_needs_login(app);
        return Task::none();
    };

    if app.expert_hub.account_id.as_deref() != Some(account_id.as_str()) {
        app.expert_hub.clear_for_account(Some(account_id.clone()));
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
                    gateway_shiju_system_skills_async(&credential, &query)
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
            let query = mine_query(app);
            let list_task = Task::perform(
                async move {
                    let credential = credential
                        .gateway_credential()
                        .map_err(|error| SkillHubError::from(error.to_string()))?;
                    gateway_shiju_maintained_skills_async(&credential, &query)
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

fn load_mine_tag_catalogs(app: &mut App) -> Task<Message> {
    let should_load =
        matches!(app.expert_hub.platform_tag_catalog, LoadState::Idle | LoadState::Error(_))
            || matches!(app.expert_hub.company_tag_catalog, LoadState::Idle | LoadState::Error(_));
    if !should_load {
        return Task::none();
    }
    let Some(account) = app.account.active_account() else {
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
            gateway_shiju_skill_tags_async(&credential)
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
            gateway_shiju_tenant_skill_tags_async(&credential)
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

fn load_marketplace_skills(app: &mut App) -> Task<Message> {
    let active = app
        .account
        .active_account()
        .map(|account| (account.id().to_string(), account.skill_hub_credential()));
    let Some((account_id, credential)) = active else {
        if app.expert_hub.account_id.is_some() {
            app.expert_hub.clear_for_account(None);
        }
        app.expert_hub.marketplace = LoadState::NeedsLogin;
        return Task::none();
    };
    if app.expert_hub.account_id.as_deref() != Some(account_id.as_str()) {
        app.expert_hub.clear_for_account(Some(account_id.clone()));
    }
    let generation = app.expert_hub.next_request_generation();
    app.expert_hub.marketplace = LoadState::Loading { generation };
    let query = marketplace_query(app);
    Task::perform(
        async move {
            let credential = credential
                .gateway_credential()
                .map_err(|error| SkillHubError::from(error.to_string()))?;
            gateway_shiju_skill_search_async(&credential, &query)
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
pub(super) fn marketplace_query(app: &App) -> ShijuSkillSearchQuery {
    let query = app.expert_hub.submitted_search.clone();
    ShijuSkillSearchQuery {
        query: (!query.is_empty()).then_some(query),
        cursor: None,
        page_size: SHIJU_PAGE_SIZE,
        tag_ids: app.expert_hub.market_tag_id,
        company_tag_ids: app.expert_hub.market_company_tag_id,
    }
}

fn load_installed_skills(app: &mut App, force: bool) -> Task<Message> {
    let active = app
        .account
        .active_account()
        .map(|account| (account.id().to_string(), account.skill_hub_credential()));
    let Some((account_id, credential)) = active else {
        if app.expert_hub.account_id.is_some() {
            app.expert_hub.clear_for_account(None);
        }
        app.expert_hub.installed = LoadState::NeedsLogin;
        return Task::none();
    };

    if app.expert_hub.account_id.as_deref() != Some(account_id.as_str()) {
        app.expert_hub.clear_for_account(Some(account_id.clone()));
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
            gateway_shiju_skill_installed_async(&credential)
                .await
                .map(|items| items.into_iter().map(InstalledSkill::from).collect())
                .map_err(SkillHubError::from)
        },
        move |result| {
            Message::SkillHub(SkillHubMessage::InstalledLoaded { generation, account_id, result })
        },
    )
}
