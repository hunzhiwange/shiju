//! Project only inventories belonging to the current catalog scope.
use crate::app::App;
use desktop_settings_memory::system_settings_device_sync::selection::{Choice, SelectionRow};
use vw_api_project_profiles::ProjectProfileScope;

pub(super) fn inventory(app: &App) -> Vec<SelectionRow> {
    let mut rows = Vec::new();
    let context = app.apps_catalog.context.as_ref();
    if let Some(scope) = &app.app_providers.scope {
        if Some(&scope.context) == context && scope.host == app.apps_catalog.host {
            if let Some(snapshot) = &app.app_providers.snapshot {
                for row in &snapshot.catalog.rows {
                    let p = &row.provider;
                    rows.push(SelectionRow::new(
                        format!("供应商 {:?} · {}", p.app_id, p.name),
                        Choice::Provider(p.app_id, p.id.clone()),
                        true,
                    ));
                }
            }
        }
    }
    if let Some(Ok(catalog)) = crate::app::message::settings::mcp_management_list::visible(app) {
        for record in &catalog.records {
            rows.push(SelectionRow::new(format!("MCP · {}", record.name), Choice::Mcp(record.record.id.0.clone()), true));
        }
    }
    let prompts = &app.apps_catalog.prompts;
    if let Some(scope) = &prompts.scope {
        if scope.target_host_id == app.apps_catalog.host
            && Some(scope.project_id.as_ref()) == app.project_id.as_deref()
        {
            if let Some(catalog) = &prompts.catalog {
                for entry in &catalog.entries {
                    rows.push(SelectionRow::new(
                        format!("规则 {:?} · {}", scope.app_id, entry.name),
                        Choice::Prompt(scope.app_id, entry.id.clone()),
                        true,
                    ));
                }
            }
        }
    }
    let matches =
        |scope: &vw_api_app_skills::repos_api::SkillReposQuery| {
            scope.target_host_id == app.apps_catalog.host
                && Some(scope.project_id.as_ref()) == app.project_id.as_deref()
        };
    if let Some(inventory) =
        crate::app::message::settings::skills::install_view::sync_inventory(app)
    {
        if matches(&inventory.scope) {
            for skill in &inventory.skills {
                if skill.present {
                    rows.push(SelectionRow::new(format!("技能 · {}", skill.name), Choice::Skill(skill.id.clone()), true));
                }
            }
        }
    }
    if let Some(inventory) = crate::app::message::settings::skills::repo_modal::sync_inventory(app)
    {
        if matches(&inventory.scope) {
            for repo in &inventory.repos {
                rows.push(SelectionRow::new(
                    format!("技能仓库 · {}/{}", repo.owner, repo.name),
                    Choice::Repository(repo.owner.clone(), repo.name.clone()),
                    true,
                ));
            }
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    for group in ProjectProfileScope::ALL {
        for profile in &app.apps_catalog.project_profiles.groups.get(group).profiles {
            if !seen.insert(profile.id.0.clone()) {
                continue;
            }
            let has_skills = ProjectProfileScope::ALL
                .iter()
                .any(|g| profile.payload.skills.get(*g).as_ref().is_some_and(|v| !v.is_empty()));
            rows.push(SelectionRow::new(
                format!(
                    "组合 · {}{}",
                    profile.name,
                    if has_skills {
                        "（技能引用尚无授权目录解析，暂不可选）"
                    } else {
                        ""
                    }
                ),
                Choice::Profile(profile.clone()),
                !has_skills,
            ));
        }
    }
    rows
}
