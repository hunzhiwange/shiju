//! Observe durable jobs independently of the lifetime of their dialog.
use super::Input;
use {
    vw_api_app_skills::jobs_api::*,
    vw_api_app_skills::repos_api::SkillReposQuery,
    vw_api_transport::id::TaskId,
};

pub use crate::jobs_view::{State, Event, terminal};

pub enum Effect { None, Error(String), Finished(bool), Poll(u64), Request { g: u64, scope: SkillReposQuery, id: TaskId, cancel: bool } }
pub fn update(state: &mut State, input: &Input, event: Event) -> Effect {
    let s = state;
    if matches!(event, Event::Close) { s.open = false; return Effect::None; }
    let Some(context) = input.context.clone() else { return Effect::None; };
    if s.context.as_ref() != Some(&context) || s.scope.as_ref().is_none_or(|scope|
        scope.project_id.as_ref() != vw_api_transport::local_settings_project(input.project.as_deref()) || scope.target_host_id != input.host) {
        *s = State { generation: s.generation.wrapping_add(1), ..Default::default() };
        return Effect::None;
    }
    match event {
        Event::Loaded(g, result) if g == s.generation && s.busy => {
            s.busy = false;
            match result {
                Ok(job) if Some(&job.scope) == s.scope.as_ref() && Some(&job.task_id) == s.task_id.as_ref() => {
                    let done = terminal(job.phase);
                    let success = job.phase == SkillJobPhase::Completed && !job.requires_reconciliation;
                    s.snapshot = Some(job); s.error = None;
                    if done && !s.notified {
                        s.notified = true;
                        return Effect::Finished(success);
                    }
                    if !done {
                        return Effect::Poll(g);
                    }
                }
                Ok(_) => s.error = Some("任务响应目标不匹配".into()),
                Err(e) => { s.error = Some(e.clone()); return Effect::Error(e); }
            }
            return Effect::None;
        }
        Event::Loaded(..) => return Effect::None,
        Event::Poll(g) if g != s.generation => return Effect::None,
        Event::Open => s.open = true,
        _ => {}
    }
    if s.busy { return Effect::None; }
    let cancel = matches!(event, Event::Cancel);
    if cancel && s.snapshot.as_ref().is_none_or(|job| terminal(job.phase) || job.phase == SkillJobPhase::Committing) {
        return Effect::None;
    }
    let (Some(scope), Some(id)) = (s.scope.clone(), s.task_id.clone()) else { return Effect::None; };
    s.busy = true; s.generation = s.generation.wrapping_add(1);
    let g = s.generation;
    Effect::Request { g, scope, id, cancel }
}

pub fn submitted(state: &mut State, input: &Input, scope: SkillReposQuery, id: TaskId) -> Effect {
    let Some(context) = input.context.clone() else { return Effect::None; };
    *state = State { open: true, task_id: Some(id), scope: Some(scope), context: Some(context),
        generation: state.generation.wrapping_add(1), ..Default::default() };
    update(state, input, Event::Refresh)
}
