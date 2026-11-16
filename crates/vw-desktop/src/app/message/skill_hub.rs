//! 协调识矩技能列表、详情、安装、版本操作与本地缓存同步消息。

use crate::app::skill_hub::{
    CursorPage, ExpertHubSection, InstalledSkill, LoadState, MarketplaceSkill,
    MineSkillStatus, PackageVersionCacheEntry, Page, PublicDetailMode,
    ShijuInstallationsReport, ShijuKeySyncReport,
    ShijuMultiInstallReport, ShijuPackageId, ShijuPackageRelease, ShijuUninstallReport,
    SkillActionKind, SkillCacheSyncReport, SkillDetail, SkillHubError,
    SkillInstallScope, SkillInstallSelection, SkillInstallTarget, SkillTab, SkillTag,
    SkillVersion, SkillVersionAction, SystemSkill, TenantSkill,     };
use crate::app::{
    App, AppTab, Message, Screen,
};
#[cfg(test)]
use crate::app::skill_hub::ShijuDirectInstallState;
use iced::Task;
use desktop_expert_hub::message::skill_hub::{self as domain, SkillHubMessage as D};
use desktop_expert_hub::message::Message as DomainEvent;

use std::path::{
    PathBuf,
};

use gateway_shiju_skill::{
    ShijuMaintainedSkillQuery, ShijuSkillSearchQuery,
};

pub(crate) mod connectors;
pub(crate) mod knowledge;
mod public;
#[cfg(test)]
mod public_tests;
mod upgrades;
#[cfg(test)]
mod upgrades_tests;
pub(crate) use public::PublicSkillMessage;
pub(crate) use public::selectable_expert_skill;

pub(crate) use desktop_expert_hub::message::skill_hub::{
    EXPERT_HUB_TAB_ID, EXPERT_HUB_TITLE, SHIJU_SKILLS_TAB_ID, SHIJU_SKILLS_TITLE,
};
pub(super) use desktop_expert_hub::skill_operations::*;
pub(super) use desktop_expert_hub::message::skill_hub::install_shiju_package_for_targets;
use desktop_expert_hub::message::{
    SkillUpdate, HubPage, Effect,
};

#[derive(Debug, Clone)]
#[allow(dead_code)]
/// 专家中心技能目录、安装向导、版本操作与公共市场的消息集合。
pub(crate) enum SkillHubMessage {
    Connectors(connectors::ConnectorMessage),
    Knowledge(knowledge::KnowledgeMessage),
    Public(public::PublicSkillMessage),
    Open,
    OpenShijuSkills,
    OpenInstalledShijuSkills,
    UpgradeAllFunctions,
    UpgradeMainSkill,
    AllFunctionsUpgraded {
        generation: u64,
        account_id: String,
        result: Result<upgrades::FunctionUpgradeReport, SkillHubError>,
    },
    MainUpgradePackageLoaded {
        generation: u64,
        account_id: String,
        selections: Vec<SkillInstallSelection>,
        project_root: Option<PathBuf>,
        result: Result<ShijuPackageRelease, SkillHubError>,
    },
    MainUpgradeKeySynced {
        generation: u64,
        account_id: String,
        result: Result<ShijuKeySyncReport, SkillHubError>,
    },
    MainSkillUpgraded {
        generation: u64,
        account_id: String,
        project_root: Option<PathBuf>,
        result: Result<ShijuMultiInstallReport, SkillHubError>,
    },
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
        result: Result<ShijuInstallationsReport, SkillHubError>,
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
        result: Result<ShijuMultiInstallReport, SkillHubError>,
    },
    ShijuKeySyncFinished {
        generation: u64,
        account_id: String,
        selections: Vec<SkillInstallSelection>,
        requested_by_user: bool,
        result: Result<ShijuKeySyncReport, SkillHubError>,
    },
    ShijuUninstallFinished {
        generation: u64,
        selections: Vec<SkillInstallSelection>,
        result: Result<ShijuUninstallReport, SkillHubError>,
    },
    ShijuPackageVersionRefreshed {
        generation: u64,
        result: Result<PackageVersionCacheEntry, SkillHubError>,
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
        result: Result<SkillDetail, SkillHubError>,
    },
    SkillActionFinished {
        generation: u64,
        account_id: String,
        skill_id: u64,
        action: SkillActionKind,
        result: Result<(), SkillHubError>,
    },
    SkillCacheSyncFinished {
        generation: u64,
        account_id: String,
        skill_id: u64,
        result: Result<SkillCacheSyncReport, crate::app::skill_hub::SkillCacheSyncError>,
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

/// 桌面只分派尚未迁移的相邻子域，并映射本域输入/输出。
pub(crate) fn update(app: &mut App, message: SkillHubMessage) -> Task<Message> {
    let message = match message {
        SkillHubMessage::Connectors(message) => return connectors::update(app, message),
        SkillHubMessage::Knowledge(message) => return knowledge::update(app, message),
        SkillHubMessage::Public(message) => return public::update(app, message),
        SkillHubMessage::UpgradeAllFunctions => D::UpgradeAllFunctions,
        SkillHubMessage::UpgradeMainSkill => D::UpgradeMainSkill,
        SkillHubMessage::AllFunctionsUpgraded { generation, account_id, result } => D::AllFunctionsUpgraded { generation, account_id, result },
        SkillHubMessage::MainUpgradePackageLoaded { generation, account_id, selections, project_root, result } => D::MainUpgradePackageLoaded { generation, account_id, selections, project_root, result },
        SkillHubMessage::MainUpgradeKeySynced { generation, account_id, result } => D::MainUpgradeKeySynced { generation, account_id, result },
        SkillHubMessage::MainSkillUpgraded { generation, account_id, project_root, result } => D::MainSkillUpgraded { generation, account_id, project_root, result },
        SkillHubMessage::Open => D::Open,
        SkillHubMessage::OpenShijuSkills => D::OpenShijuSkills,
        SkillHubMessage::OpenInstalledShijuSkills => D::OpenInstalledShijuSkills,
        SkillHubMessage::OpenInstallGuide => D::OpenInstallGuide,
        SkillHubMessage::CloseInstallGuide => D::CloseInstallGuide,
        SkillHubMessage::CopyAgentInstall => D::CopyAgentInstall,
        SkillHubMessage::AgentPackageLoaded { generation, account_id, copy_after_load, result } => D::AgentPackageLoaded { generation, account_id, copy_after_load, result },
        SkillHubMessage::DownloadShijuPackage(value) => D::DownloadShijuPackage(value),
        SkillHubMessage::ManualPackageLoaded { generation, account_id, package_id, result } => D::ManualPackageLoaded { generation, account_id, package_id, result },
        SkillHubMessage::DirectInstallShiju => D::DirectInstallShiju,
        SkillHubMessage::SelectShijuInstallScope(value) => D::SelectShijuInstallScope(value),
        SkillHubMessage::SelectShijuInstallProject(value) => D::SelectShijuInstallProject(value),
        SkillHubMessage::ToggleShijuInstallTarget(value) => D::ToggleShijuInstallTarget(value),
        SkillHubMessage::SyncShijuKey => D::SyncShijuKey,
        SkillHubMessage::UninstallShiju => D::UninstallShiju,
        SkillHubMessage::CancelShijuUninstall => D::CancelShijuUninstall,
        SkillHubMessage::ConfirmShijuUninstall => D::ConfirmShijuUninstall,
        SkillHubMessage::OpenShijuInstallDir => D::OpenShijuInstallDir,
        SkillHubMessage::OpenShijuInstallDirFor(value) => D::OpenShijuInstallDirFor(value),
        SkillHubMessage::ShijuInstallationsLoaded { generation, selections, result } => D::ShijuInstallationsLoaded { generation, selections, result },
        SkillHubMessage::DirectPackageLoaded { generation, account_id, package_id, selections, project_root, result } => D::DirectPackageLoaded { generation, account_id, package_id, selections, project_root, result },
        SkillHubMessage::ShijuInstallFinished { generation, account_id, package_id, selections, result } => D::ShijuInstallFinished { generation, account_id, package_id, selections, result },
        SkillHubMessage::ShijuKeySyncFinished { generation, account_id, selections, requested_by_user, result } => D::ShijuKeySyncFinished { generation, account_id, selections, requested_by_user, result },
        SkillHubMessage::ShijuUninstallFinished { generation, selections, result } => D::ShijuUninstallFinished { generation, selections, result },
        SkillHubMessage::ShijuPackageVersionRefreshed { generation, result } => D::ShijuPackageVersionRefreshed { generation, result },
        SkillHubMessage::SelectSection(value) => D::SelectSection(value),
        SkillHubMessage::SelectSkillTab(value) => D::SelectSkillTab(value),
        SkillHubMessage::SearchDraftChanged(value) => D::SearchDraftChanged(value),
        SkillHubMessage::SearchSubmitted => D::SearchSubmitted,
        SkillHubMessage::MarketTagSelected(value) => D::MarketTagSelected(value),
        SkillHubMessage::MarketCompanyTagSelected(value) => D::MarketCompanyTagSelected(value),
        SkillHubMessage::FilterKeywordChanged(value) => D::FilterKeywordChanged(value),
        SkillHubMessage::FilterTagChanged(value) => D::FilterTagChanged(value),
        SkillHubMessage::ToggleMineTagFilterMenu => D::ToggleMineTagFilterMenu,
        SkillHubMessage::ToggleMineCompanyTagFilterMenu => D::ToggleMineCompanyTagFilterMenu,
        SkillHubMessage::ToggleMineStatusFilterMenu => D::ToggleMineStatusFilterMenu,
        SkillHubMessage::CloseMineFilterMenu => D::CloseMineFilterMenu,
        SkillHubMessage::MineTagToggled(value) => D::MineTagToggled(value),
        SkillHubMessage::MineCompanyTagToggled(value) => D::MineCompanyTagToggled(value),
        SkillHubMessage::MineStatusToggled(value) => D::MineStatusToggled(value),
        SkillHubMessage::FiltersSubmitted => D::FiltersSubmitted,
        SkillHubMessage::FiltersReset => D::FiltersReset,
        SkillHubMessage::PreviousPage => D::PreviousPage,
        SkillHubMessage::NextPage => D::NextPage,
        SkillHubMessage::Retry => D::Retry,
        SkillHubMessage::ReloadAfterAccountChange => D::ReloadAfterAccountChange,
        SkillHubMessage::EnsureInstalledSkillsLoaded => D::EnsureInstalledSkillsLoaded,
        SkillHubMessage::RefreshInstalledSkills => D::RefreshInstalledSkills,
        SkillHubMessage::OpenSkillDetail(value) => D::OpenSkillDetail(value),
        SkillHubMessage::CloseSkillDetail => D::CloseSkillDetail,
        SkillHubMessage::SetSkillReadmeMode(value) => D::SetSkillReadmeMode(value),
        SkillHubMessage::SkillDetailLoaded { generation, account_id, skill_id, result } => D::SkillDetailLoaded { generation, account_id, skill_id, result },
        SkillHubMessage::SkillReadmeLoaded { generation, account_id, skill_id, result } => D::SkillReadmeLoaded { generation, account_id, skill_id, result },
        SkillHubMessage::SkillVersionsLoaded { generation, account_id, skill_id, result } => D::SkillVersionsLoaded { generation, account_id, skill_id, result },
        SkillHubMessage::InstallSkill(value) => D::InstallSkill(value),
        SkillHubMessage::UninstallSkill(value) => D::UninstallSkill(value),
        SkillHubMessage::UpgradeSkill(value) => D::UpgradeSkill(value),
        SkillHubMessage::ChangeSkillVersion(value) => D::ChangeSkillVersion(value),
        SkillHubMessage::CancelSkillVersionChange => D::CancelSkillVersionChange,
        SkillHubMessage::ConfirmSkillVersionChange => D::ConfirmSkillVersionChange,
        SkillHubMessage::SkillUpgradeResolved { generation, account_id, skill_id, result } => D::SkillUpgradeResolved { generation, account_id, skill_id, result },
        SkillHubMessage::SkillActionFinished { generation, account_id, skill_id, action, result } => D::SkillActionFinished { generation, account_id, skill_id, action, result },
        SkillHubMessage::SkillCacheSyncFinished { generation, account_id, skill_id, result } => D::SkillCacheSyncFinished { generation, account_id, skill_id, result },
        SkillHubMessage::RetrySkillCacheSync => D::RetrySkillCacheSync,
        SkillHubMessage::DownloadVersion(value) => D::DownloadVersion(value),
        SkillHubMessage::MarketplaceLoaded { generation, account_id, result } => D::MarketplaceLoaded { generation, account_id, result },
        SkillHubMessage::InstalledLoaded { generation, account_id, result } => D::InstalledLoaded { generation, account_id, result },
        SkillHubMessage::SystemLoaded { generation, account_id, result } => D::SystemLoaded { generation, account_id, result },
        SkillHubMessage::MineLoaded { generation, account_id, result } => D::MineLoaded { generation, account_id, result },
        SkillHubMessage::PlatformTagCatalogLoaded { generation, account_id, result } => D::PlatformTagCatalogLoaded { generation, account_id, result },
        SkillHubMessage::CompanyTagCatalogLoaded { generation, account_id, result } => D::CompanyTagCatalogLoaded { generation, account_id, result },
    };
    run_domain(app, |input| domain::update(input, message))
}
pub(crate) fn map_domain(message: D) -> SkillHubMessage {
    match message {
        D::Connectors(message) => SkillHubMessage::Connectors(message),
        D::Knowledge(message) => SkillHubMessage::Knowledge(message),
        D::Public(message) => SkillHubMessage::Public(message),
            D::UpgradeAllFunctions => SkillHubMessage::UpgradeAllFunctions,
            D::UpgradeMainSkill => SkillHubMessage::UpgradeMainSkill,
            D::AllFunctionsUpgraded { generation, account_id, result } => SkillHubMessage::AllFunctionsUpgraded { generation, account_id, result },
            D::MainUpgradePackageLoaded { generation, account_id, selections, project_root, result } => SkillHubMessage::MainUpgradePackageLoaded { generation, account_id, selections, project_root, result },
            D::MainUpgradeKeySynced { generation, account_id, result } => SkillHubMessage::MainUpgradeKeySynced { generation, account_id, result },
            D::MainSkillUpgraded { generation, account_id, project_root, result } => SkillHubMessage::MainSkillUpgraded { generation, account_id, project_root, result },
            D::Open => SkillHubMessage::Open,
            D::OpenShijuSkills => SkillHubMessage::OpenShijuSkills,
            D::OpenInstalledShijuSkills => SkillHubMessage::OpenInstalledShijuSkills,
            D::OpenInstallGuide => SkillHubMessage::OpenInstallGuide,
            D::CloseInstallGuide => SkillHubMessage::CloseInstallGuide,
            D::CopyAgentInstall => SkillHubMessage::CopyAgentInstall,
            D::AgentPackageLoaded { generation, account_id, copy_after_load, result } => SkillHubMessage::AgentPackageLoaded { generation, account_id, copy_after_load, result },
            D::DownloadShijuPackage(value) => SkillHubMessage::DownloadShijuPackage(value),
            D::ManualPackageLoaded { generation, account_id, package_id, result } => SkillHubMessage::ManualPackageLoaded { generation, account_id, package_id, result },
            D::DirectInstallShiju => SkillHubMessage::DirectInstallShiju,
            D::SelectShijuInstallScope(value) => SkillHubMessage::SelectShijuInstallScope(value),
            D::SelectShijuInstallProject(value) => SkillHubMessage::SelectShijuInstallProject(value),
            D::ToggleShijuInstallTarget(value) => SkillHubMessage::ToggleShijuInstallTarget(value),
            D::SyncShijuKey => SkillHubMessage::SyncShijuKey,
            D::UninstallShiju => SkillHubMessage::UninstallShiju,
            D::CancelShijuUninstall => SkillHubMessage::CancelShijuUninstall,
            D::ConfirmShijuUninstall => SkillHubMessage::ConfirmShijuUninstall,
            D::OpenShijuInstallDir => SkillHubMessage::OpenShijuInstallDir,
            D::OpenShijuInstallDirFor(value) => SkillHubMessage::OpenShijuInstallDirFor(value),
            D::ShijuInstallationsLoaded { generation, selections, result } => SkillHubMessage::ShijuInstallationsLoaded { generation, selections, result },
            D::DirectPackageLoaded { generation, account_id, package_id, selections, project_root, result } => SkillHubMessage::DirectPackageLoaded { generation, account_id, package_id, selections, project_root, result },
            D::ShijuInstallFinished { generation, account_id, package_id, selections, result } => SkillHubMessage::ShijuInstallFinished { generation, account_id, package_id, selections, result },
            D::ShijuKeySyncFinished { generation, account_id, selections, requested_by_user, result } => SkillHubMessage::ShijuKeySyncFinished { generation, account_id, selections, requested_by_user, result },
            D::ShijuUninstallFinished { generation, selections, result } => SkillHubMessage::ShijuUninstallFinished { generation, selections, result },
            D::ShijuPackageVersionRefreshed { generation, result } => SkillHubMessage::ShijuPackageVersionRefreshed { generation, result },
            D::SelectSection(value) => SkillHubMessage::SelectSection(value),
            D::SelectSkillTab(value) => SkillHubMessage::SelectSkillTab(value),
            D::SearchDraftChanged(value) => SkillHubMessage::SearchDraftChanged(value),
            D::SearchSubmitted => SkillHubMessage::SearchSubmitted,
            D::MarketTagSelected(value) => SkillHubMessage::MarketTagSelected(value),
            D::MarketCompanyTagSelected(value) => SkillHubMessage::MarketCompanyTagSelected(value),
            D::FilterKeywordChanged(value) => SkillHubMessage::FilterKeywordChanged(value),
            D::FilterTagChanged(value) => SkillHubMessage::FilterTagChanged(value),
            D::ToggleMineTagFilterMenu => SkillHubMessage::ToggleMineTagFilterMenu,
            D::ToggleMineCompanyTagFilterMenu => SkillHubMessage::ToggleMineCompanyTagFilterMenu,
            D::ToggleMineStatusFilterMenu => SkillHubMessage::ToggleMineStatusFilterMenu,
            D::CloseMineFilterMenu => SkillHubMessage::CloseMineFilterMenu,
            D::MineTagToggled(value) => SkillHubMessage::MineTagToggled(value),
            D::MineCompanyTagToggled(value) => SkillHubMessage::MineCompanyTagToggled(value),
            D::MineStatusToggled(value) => SkillHubMessage::MineStatusToggled(value),
            D::FiltersSubmitted => SkillHubMessage::FiltersSubmitted,
            D::FiltersReset => SkillHubMessage::FiltersReset,
            D::PreviousPage => SkillHubMessage::PreviousPage,
            D::NextPage => SkillHubMessage::NextPage,
            D::Retry => SkillHubMessage::Retry,
            D::ReloadAfterAccountChange => SkillHubMessage::ReloadAfterAccountChange,
            D::EnsureInstalledSkillsLoaded => SkillHubMessage::EnsureInstalledSkillsLoaded,
            D::RefreshInstalledSkills => SkillHubMessage::RefreshInstalledSkills,
            D::OpenSkillDetail(value) => SkillHubMessage::OpenSkillDetail(value),
            D::CloseSkillDetail => SkillHubMessage::CloseSkillDetail,
            D::SetSkillReadmeMode(value) => SkillHubMessage::SetSkillReadmeMode(value),
            D::SkillDetailLoaded { generation, account_id, skill_id, result } => SkillHubMessage::SkillDetailLoaded { generation, account_id, skill_id, result },
            D::SkillReadmeLoaded { generation, account_id, skill_id, result } => SkillHubMessage::SkillReadmeLoaded { generation, account_id, skill_id, result },
            D::SkillVersionsLoaded { generation, account_id, skill_id, result } => SkillHubMessage::SkillVersionsLoaded { generation, account_id, skill_id, result },
            D::InstallSkill(value) => SkillHubMessage::InstallSkill(value),
            D::UninstallSkill(value) => SkillHubMessage::UninstallSkill(value),
            D::UpgradeSkill(value) => SkillHubMessage::UpgradeSkill(value),
            D::ChangeSkillVersion(value) => SkillHubMessage::ChangeSkillVersion(value),
            D::CancelSkillVersionChange => SkillHubMessage::CancelSkillVersionChange,
            D::ConfirmSkillVersionChange => SkillHubMessage::ConfirmSkillVersionChange,
            D::SkillUpgradeResolved { generation, account_id, skill_id, result } => SkillHubMessage::SkillUpgradeResolved { generation, account_id, skill_id, result },
            D::SkillActionFinished { generation, account_id, skill_id, action, result } => SkillHubMessage::SkillActionFinished { generation, account_id, skill_id, action, result },
            D::SkillCacheSyncFinished { generation, account_id, skill_id, result } => SkillHubMessage::SkillCacheSyncFinished { generation, account_id, skill_id, result },
            D::RetrySkillCacheSync => SkillHubMessage::RetrySkillCacheSync,
            D::DownloadVersion(value) => SkillHubMessage::DownloadVersion(value),
            D::MarketplaceLoaded { generation, account_id, result } => SkillHubMessage::MarketplaceLoaded { generation, account_id, result },
            D::InstalledLoaded { generation, account_id, result } => SkillHubMessage::InstalledLoaded { generation, account_id, result },
            D::SystemLoaded { generation, account_id, result } => SkillHubMessage::SystemLoaded { generation, account_id, result },
            D::MineLoaded { generation, account_id, result } => SkillHubMessage::MineLoaded { generation, account_id, result },
            D::PlatformTagCatalogLoaded { generation, account_id, result } => SkillHubMessage::PlatformTagCatalogLoaded { generation, account_id, result },
            D::CompanyTagCatalogLoaded { generation, account_id, result } => SkillHubMessage::CompanyTagCatalogLoaded { generation, account_id, result },
    }
}
fn gateway() -> Result<gateway_http::GatewayTransport, String> {
    Ok(crate::app::gateway_client()?.transport().clone())
}
fn with_domain<T>(app: &mut App, operation: impl FnOnce(&mut SkillUpdate<'_>) -> T) -> (T, Vec<Effect>) {
    let mut input = SkillUpdate {
        expert_hub: &mut app.expert_hub,
        active_account: app.account.active_account(),
        project_path: &app.navigation_tabs.project_path,
        recent_projects: &app.recent_projects,
        screen: match app.navigation_tabs.screen {
            Screen::ExpertHub => HubPage::ExpertHub,
            Screen::ShijuSkills => HubPage::ShijuSkills,
            Screen::ShijuApps => HubPage::ShijuApps,
            _ => HubPage::Other,
        },
        has_project_chat: app.navigation_tabs.open_tabs.iter().any(|tab| tab.project_path.as_deref() == app.navigation_tabs.project_path.as_deref() && tab.screen == Screen::Project),
        desktop_locale: app.desktop_locale,
        gateway,
        effects: Vec::new(),
    };
    let result = operation(&mut input);
    (result, input.effects)
}
fn apply_effects(app: &mut App, effects: Vec<Effect>) -> Task<Message> {
    let mut tasks = Vec::new();
    for effect in effects {
        tasks.push(match effect {
            Effect::SelectExpertForChat(skill_id) => {
                app.current_session_runtime_mut().tool_selector.select_manual_skill(&skill_id);
                if let Some(tab) = app.navigation_tabs.open_tabs.iter().find(|tab| tab.project_path.as_deref() == app.navigation_tabs.project_path.as_deref() && tab.screen == Screen::Project) {
                    app.navigation_tabs.active_tab_id = Some(tab.id.clone());
                    app.navigation_tabs.screen = Screen::Project;
                }
                Task::none()
            }
            Effect::OpenExpertHub => { open_expert_hub_tab(app); Task::none() }
            Effect::OpenSkills => { open_shiju_skills_tab(app); Task::none() }
            Effect::CloseAccountMenu => { app.account.menu_open = false; app.account.details_open = false; Task::none() }
            Effect::ClearAccount(id) => { app.shiju_apps.clear_for_account(id); Task::none() }
            Effect::EnsurePublic => public::ensure_loaded(app),
            Effect::SelectPublicSuites => public::update(app, PublicSkillMessage::SelectTab(crate::app::skill_hub::PublicSkillTab::Suites)),
            Effect::EnsureConnectors => connectors::ensure_loaded(app),
            Effect::EnsureKnowledge => knowledge::ensure_loaded(app),
            Effect::ReloadConnectors => connectors::load(app),
            Effect::ReloadKnowledge => knowledge::load(app),
            Effect::Feedback(kind, message, locale) => {
                let kind = match kind {
                    desktop_expert_hub::message::ToastKind::Info => crate::app::state::ToastKind::Info,
                    desktop_expert_hub::message::ToastKind::Success => crate::app::state::ToastKind::Success,
                    desktop_expert_hub::message::ToastKind::Warning => crate::app::state::ToastKind::Warning,
                    desktop_expert_hub::message::ToastKind::Error => crate::app::state::ToastKind::Error,
                };
                let fallback = message.render(crate::app::i18n::I18n::new(locale)).expect("valid skill feedback");
                let task = app.show_toast(kind, fallback);
                if let Some(toast) = &mut app.active_toast { toast.localized_message = Some(message); }
                task
            }
        });
    }
    Task::batch(tasks)
}
fn run_domain(app: &mut App, operation: impl FnOnce(&mut SkillUpdate<'_>) -> Task<DomainEvent>) -> Task<Message> {
    let (task, effects) = with_domain(app, operation);
    let effects = apply_effects(app, effects);
    Task::batch([task.map(|event| match event {
        DomainEvent::SkillHub(message) => Message::SkillHub(map_domain(message)),
        DomainEvent::CopyCode(text) => Message::CopyCode(text),
        DomainEvent::ReloadApps => Message::ShijuApps(crate::app::message::ShijuAppsMessage::ReloadAfterAccountChange),
    }), effects])
}
pub(crate) fn account_changed(app: &mut App) -> Task<Message> { run_domain(app, domain::account_changed) }
pub(super) fn dispatch_download_with(app: &App, version_id: u64, opener: impl FnOnce(&str) -> Result<(), SkillHubError>) -> Result<(), SkillHubError> { domain::dispatch_download_with(&app.expert_hub, version_id, opener) }
pub(super) fn mine_query(app: &App) -> ShijuMaintainedSkillQuery { domain::mine_query(&app.expert_hub) }
pub(super) fn marketplace_query(app: &App) -> ShijuSkillSearchQuery { domain::marketplace_query(&app.expert_hub) }

pub(crate) fn open_expert_hub_tab(app: &mut App) {
    if !app.navigation_tabs.open_tabs.iter().any(|tab| tab.id == EXPERT_HUB_TAB_ID) {
        app.navigation_tabs.open_tabs.push(AppTab {
            id: EXPERT_HUB_TAB_ID.to_string(),
            title: crate::app::i18n::I18n::new(app.desktop_locale).__(EXPERT_HUB_TITLE).to_string(),
            screen: Screen::ExpertHub,
            project_path: None,
        });
    }
    app.navigation_tabs.active_tab_id = Some(EXPERT_HUB_TAB_ID.to_string());
    app.navigation_tabs.screen = Screen::ExpertHub;
}

fn open_shiju_skills_tab(app: &mut App) {
    if !app.navigation_tabs.open_tabs.iter().any(|tab| tab.id == SHIJU_SKILLS_TAB_ID) {
        app.navigation_tabs.open_tabs.push(AppTab {
            id: SHIJU_SKILLS_TAB_ID.to_string(),
            title: crate::app::i18n::I18n::new(app.desktop_locale)
                .__(SHIJU_SKILLS_TITLE)
                .to_string(),
            screen: Screen::ShijuSkills,
            project_path: None,
        });
    }
    app.navigation_tabs.active_tab_id = Some(SHIJU_SKILLS_TAB_ID.to_string());
    app.navigation_tabs.screen = Screen::ShijuSkills;
}
