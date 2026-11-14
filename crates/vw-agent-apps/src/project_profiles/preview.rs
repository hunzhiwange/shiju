//! Pure differences over executor-authorized observations, never configuration writes.
use std::collections::{BTreeMap, BTreeSet};

use vw_api_types::project_profiles::{
    ProjectProfileCategory, ProjectProfilePayload, ProjectProfileScope,
    PROJECT_PROFILE_NO_ACTIVE_PROMPT,
};

/// Existing references and their observed effective enablement in one host/group.
/// The executor must authorize and resolve each captured category before calling
/// preview. Unreadable or unsupported state must return an error upstream, never
/// an empty map. Skill keys are installation IDs, not directory names.
#[derive(Debug, Default)]
pub struct PreviewState {
    pub providers: BTreeMap<String, bool>,
    pub mcp: BTreeMap<String, bool>,
    pub skills: BTreeMap<String, bool>,
    pub prompts: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewAction {
    /// Singleton domain operation; do not implement as independent toggles.
    Select,
    Enable,
    Disable,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewItem {
    pub category: ProjectProfileCategory,
    pub reference_id: String,
    pub action: PreviewAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfilePreview {
    pub group: ProjectProfileScope,
    pub captured: bool,
    pub items: Vec<PreviewItem>,
}

/// Read-only plan in provider/MCP/skill/prompt order, then reference ID order.
/// Missing references are diagnostic items, never executable operations. The
/// caller must revalidate versions, authorization and observations before apply;
/// this result grants no write permission and is not an execution token.
pub fn preview(
    target: &ProjectProfilePayload,
    group: ProjectProfileScope,
    current: &PreviewState,
) -> ProfilePreview {
    use ProjectProfileCategory::{Mcp, Prompt, Provider, Skill};
    let mut items = Vec::new();
    if let Some(id) = target.providers.get(group) {
        plan_selection(Provider, id, &current.providers, false, &mut items);
    }
    if let Some(ids) = target.mcp.get(group) {
        plan_toggles(Mcp, ids.iter().map(String::as_str), &current.mcp, &mut items);
    }
    if let Some(skills) = target.skills.get(group) {
        plan_toggles(
            Skill,
            skills.iter().map(|skill| skill.installation_id.as_str()),
            &current.skills,
            &mut items,
        );
    }
    if let Some(id) = target.prompts.get(group) {
        plan_selection(Prompt, id, &current.prompts, true, &mut items);
    }
    ProfilePreview { group, captured: target.scope_captured(group), items }
}

fn plan_selection(
    category: ProjectProfileCategory,
    id: &str,
    current: &BTreeMap<String, bool>,
    allow_clear: bool,
    items: &mut Vec<PreviewItem>,
) {
    if allow_clear && id == PROJECT_PROFILE_NO_ACTIVE_PROMPT {
        for (id, enabled) in current {
            if *enabled {
                items.push(PreviewItem {
                    category, reference_id: id.clone(), action: PreviewAction::Disable,
                });
            }
        }
        return;
    }
    // A dangling singleton must not deactivate the currently working selection.
    let action = match current.get(id) {
        None => Some(PreviewAction::Missing),
        Some(false) => Some(PreviewAction::Select),
        Some(true) => None,
    };
    if let Some(action) = action {
        items.push(PreviewItem { category, reference_id: id.to_owned(), action });
    }
}

fn plan_toggles<'a>(
    category: ProjectProfileCategory,
    ids: impl Iterator<Item = &'a str>,
    current: &BTreeMap<String, bool>,
    items: &mut Vec<PreviewItem>,
) {
    let target: BTreeSet<&str> = ids.collect();
    let all: BTreeSet<&str> = current.keys().map(String::as_str)
        .chain(target.iter().copied()).collect();
    for id in all {
        let desired = target.contains(id);
        let action = match current.get(id) {
            None => Some(PreviewAction::Missing),
            Some(enabled) if *enabled == desired => None,
            Some(_) if desired => Some(PreviewAction::Enable),
            Some(_) => Some(PreviewAction::Disable),
        };
        if let Some(action) = action {
            items.push(PreviewItem { category, reference_id: id.to_owned(), action });
        }
    }
}
