//! Reuse scoped domain inventories; never turn an empty choice into select-all.
use crate::app::components::system_settings_common::settings_checkbox_style;
use crate::app::{App, Message};
use iced::{
    Element,
    widget::{checkbox, column, text},
};
use vw_gateway_client::{
    DeviceSyncSelection,
    vw_api_types::{
        agent_apps::AppId,
        project_profiles::{ProjectProfile, ProjectProfileScope},
    },
};

#[derive(Clone)]
enum Choice {
    Preferences,
    Provider(AppId, String),
    Mcp(String),
    Prompt(AppId, String),
    Skill(String),
    Repository(String, String),
    Profile(ProjectProfile),
}
impl Choice {
    fn checked(&self, s: &DeviceSyncSelection) -> bool {
        match self {
            Self::Preferences => s.preferences,
            Self::Provider(a, id) => s.providers.contains(&(*a, id.clone())),
            Self::Mcp(id) => s.mcp.contains(id),
            Self::Prompt(a, id) => s.prompts.contains(&(*a, id.clone())),
            Self::Skill(id) => s.skills.contains(id),
            Self::Repository(a, b) => s.repositories.contains(&(a.clone(), b.clone())),
            Self::Profile(p) => s.profiles.contains(&p.id.0),
        }
    }
    fn apply(&self, s: &mut DeviceSyncSelection, selected: bool) {
        fn toggle<T: PartialEq>(v: &mut Vec<T>, item: T, on: bool) {
            v.retain(|x| x != &item);
            if on {
                v.push(item);
            }
        }
        match self {
            Self::Preferences => s.preferences = selected,
            Self::Provider(a, id) => toggle(&mut s.providers, (*a, id.clone()), selected),
            Self::Mcp(id) => toggle(&mut s.mcp, id.clone(), selected),
            Self::Prompt(a, id) => toggle(&mut s.prompts, (*a, id.clone()), selected),
            Self::Skill(id) => toggle(&mut s.skills, id.clone(), selected),
            Self::Repository(a, b) => toggle(&mut s.repositories, (a.clone(), b.clone()), selected),
            Self::Profile(p) => {
                toggle(&mut s.profiles, p.id.0.clone(), selected);
                if selected {
                    for group in ProjectProfileScope::ALL {
                        let app = group.app_id();
                        if let Some(id) = p.payload.providers.get(group) {
                            toggle(&mut s.providers, (app, id.clone()), true);
                        }
                        if let Some(ids) = p.payload.mcp.get(group) {
                            for id in ids {
                                toggle(&mut s.mcp, id.clone(), true);
                            }
                        }
                        if let Some(id) = p.payload.prompts.get(group) {
                            if !id.is_empty() {
                                toggle(&mut s.prompts, (app, id.clone()), true);
                            }
                        }
                    }
                }
            }
        }
    }
}
pub(super) fn view(
    app: &App,
    selection: &DeviceSyncSelection,
    enabled: bool,
    message: fn(DeviceSyncSelection) -> Message,
) -> Element<'static, Message> {
    let mut rows = column![text("同步范围（明确选择）"), text("库存来自对应设置页的已加载列表；缺少项目请先在对应页面刷新。组合会包含其供应商、MCP 和规则引用，取消组合后可逐项取消依赖。").size(12)].spacing(8);
    let mut add = |label: String, choice: Choice, available: bool| {
        let mut control =
            checkbox(choice.checked(selection)).label(label).style(settings_checkbox_style);
        if enabled && available {
            let current = selection.clone();
            control = control.on_toggle(move |on| {
                let mut next = current.clone();
                choice.apply(&mut next, on);
                message(next)
            });
        }
        rows = std::mem::replace(&mut rows, column![]).push(control);
    };
    add("偏好设置".into(), Choice::Preferences, true);
    let context = app.apps_catalog.context.as_ref();
    if let Some(scope) = &app.app_providers.scope {
        if Some(&scope.context) == context && scope.host == app.apps_catalog.host {
            if let Some(snapshot) = &app.app_providers.snapshot {
                for row in &snapshot.catalog.rows {
                    let p = &row.provider;
                    add(
                        format!("供应商 {:?} · {}", p.app_id, p.name),
                        Choice::Provider(p.app_id, p.id.clone()),
                        true,
                    );
                }
            }
        }
    }
    if let Some(Ok(catalog)) = crate::app::message::settings::mcp_management_list::visible(app) {
        for record in &catalog.records {
            add(format!("MCP · {}", record.name), Choice::Mcp(record.record.id.0.clone()), true);
        }
    }
    let prompts = &app.apps_catalog.prompts;
    if let Some(scope) = &prompts.scope {
        if scope.target_host_id == app.apps_catalog.host
            && Some(scope.project_id.as_ref()) == app.project_id.as_deref()
        {
            if let Some(catalog) = &prompts.catalog {
                for entry in &catalog.entries {
                    add(
                        format!("规则 {:?} · {}", scope.app_id, entry.name),
                        Choice::Prompt(scope.app_id, entry.id.clone()),
                        true,
                    );
                }
            }
        }
    }
    let matches =
        |scope: &vw_gateway_client::vw_api_types::agent_apps_skills::repos_api::SkillReposQuery| {
            scope.target_host_id == app.apps_catalog.host
                && Some(scope.project_id.as_ref()) == app.project_id.as_deref()
        };
    if let Some(inventory) =
        crate::app::message::settings::skills::install_view::sync_inventory(app)
    {
        if matches(&inventory.scope) {
            for skill in &inventory.skills {
                if skill.present {
                    add(format!("技能 · {}", skill.name), Choice::Skill(skill.id.clone()), true);
                }
            }
        }
    }
    if let Some(inventory) = crate::app::message::settings::skills::repo_modal::sync_inventory(app)
    {
        if matches(&inventory.scope) {
            for repo in &inventory.repos {
                add(
                    format!("技能仓库 · {}/{}", repo.owner, repo.name),
                    Choice::Repository(repo.owner.clone(), repo.name.clone()),
                    true,
                );
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
            add(
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
            );
        }
    }
    rows.into()
}
