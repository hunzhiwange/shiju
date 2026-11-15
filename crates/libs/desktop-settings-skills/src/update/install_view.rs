//! One explicit host/source request per confirmation; credentials never enter events.
use super::Input;
use vw_api_app_skills::{install_api::*, repos_api::SkillReposQuery};

pub use crate::install_view::{State, Event};

pub enum Effect { None, Error(String), Submitted(SkillReposQuery, vw_api_transport::id::TaskId), Refresh, Request { g: u64, scope: SkillReposQuery, request: Option<InstallSkillRequest> } }
pub fn update(state: &mut State, input: &Input, event: Event) -> Effect {
    if matches!(event, Event::Install) && input.job_active { return Effect::Error("请先确认当前技能任务的最终状态".into()); }
    let s = state;
    if matches!(event, Event::Close) {
        // Keep a submitted operation visible until its terminal response.
        if !s.busy { *s = State { generation: s.generation.wrapping_add(1), ..Default::default() }; }
        return Effect::None;
    }
    let Some(context) = input.context.clone() else { return Effect::None; };
    if matches!(event, Event::Open) && !s.open {
        let Some(project) = input.project.clone() else { return Effect::Error(("请先打开已登记的项目").into()); };
        *s = State { open: true, context: Some(context.clone()), catalog: input.catalog,
            scope: Some(SkillReposQuery { project_id: project.into(), target_host_id: input.host.clone() }),
            generation: s.generation.wrapping_add(1), ..Default::default() };
    }
    if !s.open { return Effect::None; }
    if s.context.as_ref() != Some(&context) || s.catalog != input.catalog
        || s.scope.as_ref().is_none_or(|scope| Some(scope.project_id.as_ref()) != input.project.as_deref()
            || scope.target_host_id != input.host) {
        *s = State { generation: s.generation.wrapping_add(1), ..Default::default() };
        return Effect::None;
    }
    match event {
        Event::Submitted(g, result) if g == s.generation && s.busy => {
            s.busy = false;
            match result {
                Ok(response) if Some(&response.scope) == s.scope.as_ref() => {
                    s.inventory = None;
                    return Effect::Submitted(response.scope, response.task_id);
                }
                Ok(_) => s.error = Some("安装响应目标不匹配".into()),
                Err(e) => { s.error = Some(e.clone()); return Effect::Error(e); }
            }
            return Effect::None;
        }
        Event::Submitted(..) => return Effect::None,
        Event::Loaded(g, result) if g == s.generation && s.busy => {
            s.busy = false;
            match result {
                Ok(data) if Some(&data.scope) == s.scope.as_ref() => { s.inventory = Some(data); s.error = None; }
                Ok(_) => s.error = Some("技能响应目标不匹配".into()),
                Err(e) => { s.error = Some(e.clone()); return Effect::Error(e); }
            }
            return Effect::None;
        }
        Event::Installed(g, result) if g == s.generation && s.busy => {
            s.busy = false;
            if let Err(e) = result { s.error = Some(e.clone()); return Effect::Error(e); }
            s.inventory = None;
            return Effect::Refresh;
        }
        Event::Loaded(..) | Event::Installed(..) => return Effect::None,
        _ if s.busy => return Effect::None,
        Event::Select(id) => { s.selected = Some(id); return Effect::None; }
        Event::Repo(v) => { s.repo = v; return Effect::None; }
        Event::Selection(v) => { s.selection = v; return Effect::None; }
        Event::Directory(v) => { s.directory = v; return Effect::None; }
        _ => {}
    }
    if !input.authorized {
        s.error = Some("请先在应用设置中连接并授权目标主机".into()); return Effect::None;
    }
    let scope = s.scope.clone().expect("open scope");
    let request = if matches!(event, Event::Install) {
        let Some(data) = &s.inventory else { return Effect::None; };
        if s.repo.trim().is_empty() || s.selection.trim().is_empty() || s.directory.trim().is_empty() {
            s.error = Some("请选择仓库、来源目录与安装目录".into()); return Effect::None;
        }
        Some(InstallSkillRequest { scope: scope.clone(), expected_version: data.version,
            source: SkillInstallSource::Git { repo_id: s.repo.trim().into(),
                selection: s.selection.trim().into(), directory: s.directory.trim().into() } })
    } else { None };
    s.inventory = None;
    s.busy = true; s.error = None; s.generation = s.generation.wrapping_add(1);
    let g = s.generation;
    Effect::Request { g, scope, request }
}

#[cfg(test)]
#[path = "install_view_tests.rs"]
mod tests;

pub fn sync_inventory<'a>(state: &'a State, context: &Option<desktop_provider_apps::AgentAppsContext>, catalog: u64) -> Option<&'a vw_api_app_skills::inventory_api::SkillInventoryResponse> {
    (state.context.as_ref() == context.as_ref() && state.catalog == catalog).then_some(state.inventory.as_ref()).flatten()
}
