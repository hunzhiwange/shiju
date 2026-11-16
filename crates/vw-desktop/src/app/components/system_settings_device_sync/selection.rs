//! Project only inventories belonging to the current catalog scope.
use crate::app::App;
use desktop_settings_memory::system_settings_device_sync::selection::SelectionRow;
use vw_api_project_profiles::ProjectProfileScope;

pub(super) fn inventory(app: &App) -> Vec<SelectionRow> {
    let mut rows = Vec::new();
    let context = app.apps_catalog.context.as_ref();
    if let Some(scope) = &app.app_providers.scope {
        if Some(&scope.context) == context && scope.host == app.apps_catalog.host {
            if let Some(snapshot) = &app.app_providers.snapshot {
                for row in &snapshot.catalog.rows {
                    let p = &row.provider;
                    rows.push(SelectionRow::provider(p.app_id, p.id.clone(), &p.name));
                }
            }
        }
    }
    if let Some(Ok(catalog)) = crate::app::message::settings::mcp_management_list::visible(app) {
        for record in &catalog.records {
            rows.push(SelectionRow::mcp(record.record.id.0.clone(), &record.name));
        }
    }
    let prompts = &app.apps_catalog.prompts;
    if let Some(scope) = &prompts.scope {
        if scope.target_host_id == app.apps_catalog.host
            && Some(scope.project_id.as_ref()) == app.project_id.as_deref()
        {
            if let Some(catalog) = &prompts.catalog {
                for entry in &catalog.entries {
                    rows.push(SelectionRow::prompt(scope.app_id, entry.id.clone(), &entry.name));
                }
            }
        }
    }
    let matches = |scope: &vw_api_app_skills::repos_api::SkillReposQuery| {
        scope.target_host_id == app.apps_catalog.host
            && Some(scope.project_id.as_ref()) == app.project_id.as_deref()
    };
    if let Some(inventory) =
        crate::app::message::settings::skills::install_view::sync_inventory(app)
    {
        if matches(&inventory.scope) {
            for skill in &inventory.skills {
                if skill.present {
                    rows.push(SelectionRow::skill(skill.id.clone(), &skill.name));
                }
            }
        }
    }
    if let Some(inventory) = crate::app::message::settings::skills::repo_modal::sync_inventory(app)
    {
        if matches(&inventory.scope) {
            for repo in &inventory.repos {
                rows.push(SelectionRow::repository(repo.owner.clone(), repo.name.clone()));
            }
        }
    }
    desktop_settings_memory::system_settings_device_sync::selection::append_profiles(
        &mut rows,
        ProjectProfileScope::ALL
            .into_iter()
            .flat_map(|group| &app.apps_catalog.project_profiles.groups.get(group).profiles),
    );
    rows
}
