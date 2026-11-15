//! Storage drafts never replace the acknowledged execution-host root.
use super::Input;
use vw_api_app_skills::{bindings_api::*, repos_api::SkillReposQuery, storage_api::*};

pub use crate::storage_modal::{State, Event, Readback};
pub enum Change { OpenDirectory(String), Read, Mode(SkillStoragePatch), Move(SkillStorageMoveRequest), Resync(SkillResyncRequest) }


pub enum Effect { None, Error(String),  Request { generation: u64, scope: SkillReposQuery, change: Change } }
pub fn update(state: &mut State, input: &Input, event: Event) -> Effect {
    let s = state;
    if matches!(event, Event::Close) {
        if !s.busy { *s = State { generation: s.generation.wrapping_add(1), ..Default::default() }; }
        return Effect::None;
    }
    let Some(context) = input.context.clone() else { return Effect::Error(("无法连接存储服务").into()); };
    if matches!(event, Event::Open) {
        if s.open { return Effect::None; }
        let Some(project) = input.project.clone() else { return Effect::Error(("请先打开已登记的项目").into()); };
        *s = State { open: true, context: Some(context.clone()), catalog: input.catalog,
            scope: Some(SkillReposQuery { project_id: project.into(), target_host_id: input.host.clone() }),
            generation: s.generation.wrapping_add(1), ..Default::default() };
    }
    if !s.open { return Effect::None; }
    if s.context.as_ref() != Some(&context) || s.catalog != input.catalog || s.scope.as_ref().is_none_or(|scope|
        Some(scope.project_id.as_ref()) != input.project.as_deref() || scope.target_host_id != input.host) {
        *s = State { generation: s.generation.wrapping_add(1), ..Default::default() };
        return Effect::None;
    }
    if let Event::Completed(generation, result) = event {
        if generation != s.generation || !s.busy { return Effect::None; }
        s.busy = false;
        if let Err(error) = result.and_then(|value| s.accept(value)) {
            s.error = Some(error.clone()); return Effect::Error(error);
        }
        return Effect::None;
    }
    if s.busy { return Effect::None; }
    if let Event::Select(location) = event { s.pending = Some(location); return Effect::None; }
    if !input.authorized {
        s.error = Some("请先在应用设置中连接并授权目标主机".into()); return Effect::None;
    }
    let scope = s.scope.clone().expect("open scope");
    let change = match event {
        Event::Open | Event::Refresh => Change::Read,
        Event::Mode(mode) => { let Some(current) = &s.current else { return Effect::None; };
            Change::Mode(SkillStoragePatch { scope: scope.clone(), expected_version: current.version, mode }) }
        Event::Move => { let (Some(current), Some(location)) = (&s.current, s.pending) else { return Effect::None; };
            Change::Move(SkillStorageMoveRequest { scope: scope.clone(), expected_version: current.version, location, targets: Vec::new() }) }
        Event::OpenDirectory => {
            if !input.loopback || scope.target_host_id != local_app_auth::HOST_ID {
                return Effect::Error(("远程目标目录不可在本机打开").into());
            }
            let Some(current) = &s.current else { return Effect::None; };
            Change::OpenDirectory(current.root.clone())
        }
        Event::Resync => Change::Resync(SkillResyncRequest { scope: scope.clone(), targets: Vec::new() }),
        _ => return Effect::None,
    };
    s.busy = true;
    s.error = None;
    s.results.clear();
    s.generation = s.generation.wrapping_add(1);
    let generation = s.generation;
    Effect::Request { generation, scope, change }
}
#[cfg(target_os = "macos")]
pub fn open_local(home: &str, path: &str) -> Result<(), ()> {
    use std::path::Path;
    let home = Path::new(home);
    let path = Path::new(path);
    let relative = path.strip_prefix(home).map_err(|_| ())?;
    if relative.as_os_str().is_empty()
        || !relative.components().all(|part| matches!(part, std::path::Component::Normal(_)))
    {
        return Err(());
    }
    let actual = path.canonicalize().map_err(|_| ())?;
    if actual != home.canonicalize().map_err(|_| ())?.join(relative) || !actual.is_dir() {
        return Err(());
    }
    let status = std::process::Command::new("/usr/bin/open")
        .args(["-a", "Finder"])
        .arg(actual)
        .status()
        .map_err(|_| ())?;
    status.success().then_some(()).ok_or(())
}

#[cfg(not(target_os = "macos"))]
pub fn open_local(_: &str, _: &str) -> Result<(), ()> {
    Err(())
}
