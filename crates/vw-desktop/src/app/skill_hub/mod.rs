mod cache_sync;
mod client;
mod installer;
mod models;
mod package_version_cache;
mod state;

pub(crate) use cache_sync::{SkillCacheSyncReport, sync_business_skill_cache};
pub(crate) use client::{SkillHubClient, SkillHubError};
pub(crate) use installer::{
    ResolvedSkillDestination, ShijuInstallItem, ShijuInstallReport, ShijuInstallation,
    ShijuInstallationsReport, ShijuKeySyncOutcome, ShijuKeySyncReport, ShijuMultiInstallReport,
    ShijuTargetInstallation, ShijuTargetKeySync, ShijuTargetUninstall, ShijuUninstallReport,
    SkillInstallScope, SkillInstallSelection, SkillInstallTarget,
    install_shiju_archive_for_targets, install_shiju_package, resolve_skill_destinations,
    shiju_install_dir, shiju_install_path_label, shiju_installation,
    shiju_installations_for_targets, sync_shiju_key, sync_shiju_key_for_targets,
    uninstall_shiju_for_targets, uninstall_shiju_package, version_is_newer,
};
pub(crate) use models::{
    CursorPage, InstalledSkill, MarketplaceSkill, Page, ShijuPackageId, ShijuPackageRelease,
    SkillDetail, SkillDownloadUrl, SkillTag, SkillVersion, SkillVersionContent, SystemSkill,
    TenantSkill,
};
pub(crate) use package_version_cache::{
    PackageVersionCacheEntry, load_package_version_cache, package_version_cache_path,
    write_package_version_cache,
};
pub(crate) use state::{
    ExpertHubSection, ExpertHubState, LoadState, MineFilterMenu, MineSkillStatus,
    PublicBatchActionKind, PublicBatchActionSnapshot, PublicBatchActionState, PublicBatchSummary,
    PublicDetailMode, PublicDetailState,
    PublicDetailTarget, PublicInstallAvailability, PublicInstallState, PublicMarketQuery,
    PublicSkillMarketState, PublicSkillTab, ShijuDirectInstallState, ShijuInstallGuideState,
    ShijuManualDownloadState, ShijuPackageRefreshState, ShijuPackageStatus, SkillActionKind,
    SkillActionState, SkillCacheSyncState, SkillDetailState, SkillListFilter, SkillTab,
    SkillVersionAction, SkillVersionDirection, parse_public_markdown,
};

#[cfg(test)]
mod cache_sync_tests;
#[cfg(test)]
mod client_tests;
#[cfg(test)]
mod installer_tests;
#[cfg(test)]
mod models_tests;
#[cfg(test)]
mod package_version_cache_tests;
#[cfg(test)]
mod state_tests;
