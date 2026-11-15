use crate::types::*;
use gateway_desktop_settings::{DesktopPluginCatalogEntryDto, DesktopSkillCatalogEntryDto, DesktopSkillDetailDto};
pub(super) fn map_skill_kind(kind: &str) -> SkillsCatalogKind {
    match kind {
        "recommended" => SkillsCatalogKind::Recommended,
        "personal" => SkillsCatalogKind::Personal,
        _ => SkillsCatalogKind::System,
    }
}

pub fn map_catalog_items(items: Vec<DesktopSkillCatalogEntryDto>) -> Vec<SkillsCatalogItem> {
    items
        .into_iter()
        .map(|item| SkillsCatalogItem {
            id: item.id,
            title: item.title,
            description: item.description,
            kind: map_skill_kind(&item.kind),
            resource_count: item.resource_count,
            installed: item.installed,
            enabled: item.enabled,
            source: item.source,
            source_path: item.source_path,
        })
        .collect()
}

pub(super) fn map_plugin_catalog_items(items: Vec<DesktopPluginCatalogEntryDto>) -> Vec<PluginCatalogItem> {
    items
        .into_iter()
        .map(|item| PluginCatalogItem {
            id: item.id,
            display_name: item.display_name,
            version: item.version,
            description: item.description,
            enabled: item.enabled,
            component_count: item.component_count,
            source_path: item.source_path,
            dependency_checks: item.dependency_checks,
            origin: item.origin,
            hook_review: item.hook_review,
        })
        .collect()
}

pub(super) fn map_skill_detail(detail: DesktopSkillDetailDto) -> SkillsSelectedDetail {
    SkillsSelectedDetail {
        id: detail.id,
        title: detail.title,
        description: detail.description,
        kind: map_skill_kind(&detail.kind),
        installed: detail.installed,
        enabled: detail.enabled,
        source: detail.source,
        source_path: detail.source_path,
        document_name: detail.document_name,
        document_content: detail.document_content,
        can_install: detail.can_install,
        can_toggle: detail.can_toggle,
        can_delete: detail.can_delete,
    }
}
