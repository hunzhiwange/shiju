//! 专家消息与技能领域之间的显式桥接。
use super::{Message, SkillUpdate, ToastKind};
pub use desktop_skill_hub::message::skill_hub::*;
use desktop_skill_hub::skill_hub::*;
use iced::Task;
use std::path::PathBuf;
#[path = "skill_message.rs"]
mod skill_message;
use desktop_skill_hub::message::skill_hub::SkillHubMessage as D;
pub use skill_message::SkillHubMessage;
pub fn update(app: &mut SkillUpdate<'_>, message: SkillHubMessage) -> Task<Message> {
    let message = match message {
        SkillHubMessage::Connectors(message) => return super::connectors::update(app, message),
        SkillHubMessage::Knowledge(message) => return super::knowledge::update(app, message),
        SkillHubMessage::Public(message) => return super::public::update(app, message),

        SkillHubMessage::UpgradeAllFunctions => D::UpgradeAllFunctions,
        SkillHubMessage::UpgradeMainSkill => D::UpgradeMainSkill,
        SkillHubMessage::AllFunctionsUpgraded { generation, account_id, result } => {
            D::AllFunctionsUpgraded { generation, account_id, result }
        }
        SkillHubMessage::MainUpgradePackageLoaded {
            generation,
            account_id,
            selections,
            project_root,
            result,
        } => {
            D::MainUpgradePackageLoaded { generation, account_id, selections, project_root, result }
        }
        SkillHubMessage::MainUpgradeKeySynced { generation, account_id, result } => {
            D::MainUpgradeKeySynced { generation, account_id, result }
        }
        SkillHubMessage::MainSkillUpgraded { generation, account_id, project_root, result } => {
            D::MainSkillUpgraded { generation, account_id, project_root, result }
        }
        SkillHubMessage::Open => D::Open,
        SkillHubMessage::OpenShijuSkills => D::OpenShijuSkills,
        SkillHubMessage::OpenInstalledShijuSkills => D::OpenInstalledShijuSkills,
        SkillHubMessage::OpenInstallGuide => D::OpenInstallGuide,
        SkillHubMessage::CloseInstallGuide => D::CloseInstallGuide,
        SkillHubMessage::CopyAgentInstall => D::CopyAgentInstall,
        SkillHubMessage::AgentPackageLoaded { generation, account_id, copy_after_load, result } => {
            D::AgentPackageLoaded { generation, account_id, copy_after_load, result }
        }
        SkillHubMessage::DownloadShijuPackage(value) => D::DownloadShijuPackage(value),
        SkillHubMessage::ManualPackageLoaded { generation, account_id, package_id, result } => {
            D::ManualPackageLoaded { generation, account_id, package_id, result }
        }
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
        SkillHubMessage::ShijuInstallationsLoaded { generation, selections, result } => {
            D::ShijuInstallationsLoaded { generation, selections, result }
        }
        SkillHubMessage::DirectPackageLoaded {
            generation,
            account_id,
            package_id,
            selections,
            project_root,
            result,
        } => D::DirectPackageLoaded {
            generation,
            account_id,
            package_id,
            selections,
            project_root,
            result,
        },
        SkillHubMessage::ShijuInstallFinished {
            generation,
            account_id,
            package_id,
            selections,
            result,
        } => D::ShijuInstallFinished { generation, account_id, package_id, selections, result },
        SkillHubMessage::ShijuKeySyncFinished {
            generation,
            account_id,
            selections,
            requested_by_user,
            result,
        } => D::ShijuKeySyncFinished {
            generation,
            account_id,
            selections,
            requested_by_user,
            result,
        },
        SkillHubMessage::ShijuUninstallFinished { generation, selections, result } => {
            D::ShijuUninstallFinished { generation, selections, result }
        }
        SkillHubMessage::ShijuPackageVersionRefreshed { generation, result } => {
            D::ShijuPackageVersionRefreshed { generation, result }
        }
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
        SkillHubMessage::SkillDetailLoaded { generation, account_id, skill_id, result } => {
            D::SkillDetailLoaded { generation, account_id, skill_id, result }
        }
        SkillHubMessage::SkillReadmeLoaded { generation, account_id, skill_id, result } => {
            D::SkillReadmeLoaded { generation, account_id, skill_id, result }
        }
        SkillHubMessage::SkillVersionsLoaded { generation, account_id, skill_id, result } => {
            D::SkillVersionsLoaded { generation, account_id, skill_id, result }
        }
        SkillHubMessage::InstallSkill(value) => D::InstallSkill(value),
        SkillHubMessage::UninstallSkill(value) => D::UninstallSkill(value),
        SkillHubMessage::UpgradeSkill(value) => D::UpgradeSkill(value),
        SkillHubMessage::ChangeSkillVersion(value) => D::ChangeSkillVersion(value),
        SkillHubMessage::CancelSkillVersionChange => D::CancelSkillVersionChange,
        SkillHubMessage::ConfirmSkillVersionChange => D::ConfirmSkillVersionChange,
        SkillHubMessage::SkillUpgradeResolved { generation, account_id, skill_id, result } => {
            D::SkillUpgradeResolved { generation, account_id, skill_id, result }
        }
        SkillHubMessage::SkillActionFinished {
            generation,
            account_id,
            skill_id,
            action,
            result,
        } => D::SkillActionFinished { generation, account_id, skill_id, action, result },
        SkillHubMessage::SkillCacheSyncFinished { generation, account_id, skill_id, result } => {
            D::SkillCacheSyncFinished { generation, account_id, skill_id, result }
        }
        SkillHubMessage::RetrySkillCacheSync => D::RetrySkillCacheSync,
        SkillHubMessage::DownloadVersion(value) => D::DownloadVersion(value),
        SkillHubMessage::MarketplaceLoaded { generation, account_id, result } => {
            D::MarketplaceLoaded { generation, account_id, result }
        }
        SkillHubMessage::InstalledLoaded { generation, account_id, result } => {
            D::InstalledLoaded { generation, account_id, result }
        }
        SkillHubMessage::SystemLoaded { generation, account_id, result } => {
            D::SystemLoaded { generation, account_id, result }
        }
        SkillHubMessage::MineLoaded { generation, account_id, result } => {
            D::MineLoaded { generation, account_id, result }
        }
        SkillHubMessage::PlatformTagCatalogLoaded { generation, account_id, result } => {
            D::PlatformTagCatalogLoaded { generation, account_id, result }
        }
        SkillHubMessage::CompanyTagCatalogLoaded { generation, account_id, result } => {
            D::CompanyTagCatalogLoaded { generation, account_id, result }
        }
    };
    desktop_skill_hub::message::skill_hub::update(app, message).map(map_event)
}
pub fn map_event(event: desktop_skill_hub::message::Message) -> Message {
    match event {
        desktop_skill_hub::message::Message::SkillHub(message) => {
            Message::SkillHub(map_message(message))
        }
        desktop_skill_hub::message::Message::CopyCode(text) => Message::CopyCode(text),
        desktop_skill_hub::message::Message::ReloadApps => Message::ReloadApps,
    }
}
fn map_message(message: D) -> SkillHubMessage {
    match message {
        D::UpgradeAllFunctions => SkillHubMessage::UpgradeAllFunctions,
        D::UpgradeMainSkill => SkillHubMessage::UpgradeMainSkill,
        D::AllFunctionsUpgraded { generation, account_id, result } => {
            SkillHubMessage::AllFunctionsUpgraded { generation, account_id, result }
        }
        D::MainUpgradePackageLoaded {
            generation,
            account_id,
            selections,
            project_root,
            result,
        } => SkillHubMessage::MainUpgradePackageLoaded {
            generation,
            account_id,
            selections,
            project_root,
            result,
        },
        D::MainUpgradeKeySynced { generation, account_id, result } => {
            SkillHubMessage::MainUpgradeKeySynced { generation, account_id, result }
        }
        D::MainSkillUpgraded { generation, account_id, project_root, result } => {
            SkillHubMessage::MainSkillUpgraded { generation, account_id, project_root, result }
        }
        D::Open => SkillHubMessage::Open,
        D::OpenShijuSkills => SkillHubMessage::OpenShijuSkills,
        D::OpenInstalledShijuSkills => SkillHubMessage::OpenInstalledShijuSkills,
        D::OpenInstallGuide => SkillHubMessage::OpenInstallGuide,
        D::CloseInstallGuide => SkillHubMessage::CloseInstallGuide,
        D::CopyAgentInstall => SkillHubMessage::CopyAgentInstall,
        D::AgentPackageLoaded { generation, account_id, copy_after_load, result } => {
            SkillHubMessage::AgentPackageLoaded { generation, account_id, copy_after_load, result }
        }
        D::DownloadShijuPackage(value) => SkillHubMessage::DownloadShijuPackage(value),
        D::ManualPackageLoaded { generation, account_id, package_id, result } => {
            SkillHubMessage::ManualPackageLoaded { generation, account_id, package_id, result }
        }
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
        D::ShijuInstallationsLoaded { generation, selections, result } => {
            SkillHubMessage::ShijuInstallationsLoaded { generation, selections, result }
        }
        D::DirectPackageLoaded {
            generation,
            account_id,
            package_id,
            selections,
            project_root,
            result,
        } => SkillHubMessage::DirectPackageLoaded {
            generation,
            account_id,
            package_id,
            selections,
            project_root,
            result,
        },
        D::ShijuInstallFinished { generation, account_id, package_id, selections, result } => {
            SkillHubMessage::ShijuInstallFinished {
                generation,
                account_id,
                package_id,
                selections,
                result,
            }
        }
        D::ShijuKeySyncFinished {
            generation,
            account_id,
            selections,
            requested_by_user,
            result,
        } => SkillHubMessage::ShijuKeySyncFinished {
            generation,
            account_id,
            selections,
            requested_by_user,
            result,
        },
        D::ShijuUninstallFinished { generation, selections, result } => {
            SkillHubMessage::ShijuUninstallFinished { generation, selections, result }
        }
        D::ShijuPackageVersionRefreshed { generation, result } => {
            SkillHubMessage::ShijuPackageVersionRefreshed { generation, result }
        }
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
        D::SkillDetailLoaded { generation, account_id, skill_id, result } => {
            SkillHubMessage::SkillDetailLoaded { generation, account_id, skill_id, result }
        }
        D::SkillReadmeLoaded { generation, account_id, skill_id, result } => {
            SkillHubMessage::SkillReadmeLoaded { generation, account_id, skill_id, result }
        }
        D::SkillVersionsLoaded { generation, account_id, skill_id, result } => {
            SkillHubMessage::SkillVersionsLoaded { generation, account_id, skill_id, result }
        }
        D::InstallSkill(value) => SkillHubMessage::InstallSkill(value),
        D::UninstallSkill(value) => SkillHubMessage::UninstallSkill(value),
        D::UpgradeSkill(value) => SkillHubMessage::UpgradeSkill(value),
        D::ChangeSkillVersion(value) => SkillHubMessage::ChangeSkillVersion(value),
        D::CancelSkillVersionChange => SkillHubMessage::CancelSkillVersionChange,
        D::ConfirmSkillVersionChange => SkillHubMessage::ConfirmSkillVersionChange,
        D::SkillUpgradeResolved { generation, account_id, skill_id, result } => {
            SkillHubMessage::SkillUpgradeResolved { generation, account_id, skill_id, result }
        }
        D::SkillActionFinished { generation, account_id, skill_id, action, result } => {
            SkillHubMessage::SkillActionFinished {
                generation,
                account_id,
                skill_id,
                action,
                result,
            }
        }
        D::SkillCacheSyncFinished { generation, account_id, skill_id, result } => {
            SkillHubMessage::SkillCacheSyncFinished { generation, account_id, skill_id, result }
        }
        D::RetrySkillCacheSync => SkillHubMessage::RetrySkillCacheSync,
        D::DownloadVersion(value) => SkillHubMessage::DownloadVersion(value),
        D::MarketplaceLoaded { generation, account_id, result } => {
            SkillHubMessage::MarketplaceLoaded { generation, account_id, result }
        }
        D::InstalledLoaded { generation, account_id, result } => {
            SkillHubMessage::InstalledLoaded { generation, account_id, result }
        }
        D::SystemLoaded { generation, account_id, result } => {
            SkillHubMessage::SystemLoaded { generation, account_id, result }
        }
        D::MineLoaded { generation, account_id, result } => {
            SkillHubMessage::MineLoaded { generation, account_id, result }
        }
        D::PlatformTagCatalogLoaded { generation, account_id, result } => {
            SkillHubMessage::PlatformTagCatalogLoaded { generation, account_id, result }
        }
        D::CompanyTagCatalogLoaded { generation, account_id, result } => {
            SkillHubMessage::CompanyTagCatalogLoaded { generation, account_id, result }
        }
    }
}
pub fn show_skill_error(
    app: &mut SkillUpdate<'_>,
    kind: ToastKind,
    source: Option<&str>,
    error: SkillHubError,
) -> Task<Message> {
    desktop_skill_hub::message::skill_hub::show_skill_error(app, kind, source, error).map(map_event)
}
pub fn account_changed(app: &mut SkillUpdate<'_>) -> Task<Message> {
    desktop_skill_hub::message::skill_hub::account_changed(app).map(map_event)
}

pub use super::public::PublicSkillMessage;
