//! Sequential aggregation; domain callbacks retain their native transaction owners.
use anyhow::Result;
use vw_api_types::project_profiles::{
    ProjectProfileApplyResult, ProjectProfileApplyStatus as State, ProjectProfileCategory as Category,
    ProjectProfileItemResult as Item, ProjectProfileItemStatus as Status, ProjectProfilePayload,
};
use super::{
    autosave::{AutosaveWarning, PreparedProfileSwitch},
    apply_prompt::PromptApplyFailure,
    apply_skills::{SkillApplyFailure, SkillsApplyResult},
    snapshot::SnapshotScope,
};

pub struct ApplyOutcome {
    pub result: ProjectProfileApplyResult,
    pub autosave_warning: Option<AutosaveWarning>,
}

/// Run inside the executor's outer operation lock, after prepare_switch and
/// permission/version checks. Each callback invokes its corresponding apply_*
/// entry on the same authorized scope. MCP's async entry must complete on the
/// executor runtime before returning; do not spawn a detached write job.
/// `record` atomically selects the target and records state, including clearing
/// any previous applied claim BEFORE domain writes. Initial Failed is conservative
/// if execution is interrupted. A failed final record is an error, never Applied.
/// Null slots are reported as skipped and never invoke their callbacks.
pub fn apply_prepared<P, M, S, R, W>(
    scope: &SnapshotScope<'_>,
    prepared: PreparedProfileSwitch,
    provider: P,
    mcp: M,
    skills: S,
    prompt: R,
    mut record: W,
) -> Result<ApplyOutcome>
where
    P: FnOnce(&SnapshotScope<'_>, &ProjectProfilePayload) -> Option<Item>,
    M: FnOnce(&SnapshotScope<'_>, &ProjectProfilePayload) -> Vec<Item>,
    S: FnOnce(&SnapshotScope<'_>, &ProjectProfilePayload) -> Result<SkillsApplyResult, SkillApplyFailure>,
    R: FnOnce(&SnapshotScope<'_>, &ProjectProfilePayload) -> Option<Result<(), PromptApplyFailure>>,
    W: FnMut(&ProjectProfileApplyResult) -> Result<()>,
{
    let payload = &prepared.target.payload;
    let captured = payload.scope_captured(scope.group);
    let mut result = ProjectProfileApplyResult {
        profile_id: prepared.target.id.clone(), group: scope.group, selected: true,
        status: if captured { State::Failed } else { State::Uncaptured }, items: Vec::new(),
    };
    record(&result)?;
    if captured {
        result.items.push(if payload.providers.get(scope.group).is_none() {
            skipped(Category::Provider)
        } else {
            provider(scope, payload).unwrap_or_else(|| item(Category::Provider, None, Status::Failed, Some("provider_result_missing")))
        });
        if payload.mcp.get(scope.group).is_none() {
            result.items.push(skipped(Category::Mcp));
        } else {
            result.items.extend(mcp(scope, payload));
        }
        if payload.skills.get(scope.group).is_none() {
            result.items.push(skipped(Category::Skill));
        } else {
            match skills(scope, payload) {
                Ok(observed) if observed.group == scope.group && observed.captured => {
                    for entry in observed.items {
                        let (status, reason) = skill_status(entry.result);
                        result.items.push(item(Category::Skill, Some(entry.installation_id), status, reason));
                    }
                }
                Ok(_) => result.items.push(item(Category::Skill, None, Status::Conflict, Some("skill_scope_mismatch"))),
                Err(error) => {
                    let (status, reason) = skill_status(Err(error));
                    result.items.push(item(Category::Skill, None, status, reason));
                }
            }
        }
        result.items.push(if payload.prompts.get(scope.group).is_none() {
            skipped(Category::Prompt)
        } else {
            let (status, reason) = match prompt(scope, payload) {
                Some(Ok(())) => (Status::Succeeded, None),
                Some(Err(error)) => {
                    let status = match error {
                        PromptApplyFailure::Missing => Status::Missing,
                        PromptApplyFailure::Denied => Status::Denied,
                        PromptApplyFailure::Unsupported => Status::Unsupported,
                        PromptApplyFailure::Conflict => Status::Conflict,
                        PromptApplyFailure::Failed => Status::Failed,
                    };
                    (status, Some("prompt_apply_failed"))
                }
                None => (Status::Failed, Some("prompt_result_missing")),
            };
            item(Category::Prompt, payload.prompts.get(scope.group).clone(), status, reason)
        });
        result.status = if result.items.iter().any(|item| item.status == Status::Cancelled) {
            State::Cancelled
        } else if result.items.iter().all(|item| item.status == Status::Succeeded) {
            State::Applied
        } else {
            State::Partial
        };
        record(&result)?;
    }
    Ok(ApplyOutcome { result, autosave_warning: prepared.warning })
}

fn skipped(category: Category) -> Item {
    // The wire DTO has no Skipped variant; reason_code distinguishes a no-op.
    item(category, None, Status::Succeeded, Some("uncaptured_slot_skipped"))
}
fn item(category: Category, reference_id: Option<String>, status: Status, reason: Option<&str>) -> Item {
    Item { category, reference_id, status, reason_code: reason.map(str::to_owned) }
}
fn skill_status(result: Result<(), SkillApplyFailure>) -> (Status, Option<&'static str>) {
    match result {
        Ok(()) => (Status::Succeeded, None),
        Err(error) => (match error {
            SkillApplyFailure::Missing => Status::Missing,
            SkillApplyFailure::Denied => Status::Denied,
            SkillApplyFailure::Unsupported => Status::Unsupported,
            SkillApplyFailure::Conflict => Status::Conflict,
            SkillApplyFailure::Failed => Status::Failed,
        }, Some("skill_apply_failed")),
    }
}
