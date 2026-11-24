use axum::routing::get;
use vw_api_types::cli_workbench::{CliWorkbenchDraft, CliWorkbenchDraftAction};

use super::{terminal::Authority, *};

pub(super) const MAX_PENDING_PER_PROJECT: usize = 32;

pub(super) struct Draft {
    pub(super) text: String,
    pub(super) updates: watch::Sender<CliWorkbenchMessage>,
}

pub(super) fn routes<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route(&format!("{SESSIONS}/drafts"), get(list))
        .route(&format!("{SESSIONS}/{{session}}/drafts/{{request}}"), post(resolve))
}

async fn list(
    Extension(tenant): Extension<TenantScope>,
    Path(project): Path<String>,
    authority: Authority,
) -> Result<Json<Vec<CliWorkbenchDraft>>, WorkbenchError> {
    authority.require()?;
    let scope = project_scope(tenant, project)?;
    let workbench = WORKBENCH.lock();
    let mut drafts: Vec<_> = workbench
        .deliveries
        .iter()
        .filter_map(|((owner, _), delivery)| {
            let draft = delivery.draft.as_ref()?;
            let message = delivery.updates.borrow();
            (owner == &scope)
                .then(|| CliWorkbenchDraft { id: message.id.clone(), text: draft.text.clone() })
        })
        .collect();
    drafts.sort_unstable_by(|a, b| a.id.request_id.cmp(&b.id.request_id));
    Ok(Json(drafts))
}

async fn resolve(
    Extension(tenant): Extension<TenantScope>,
    Path((project, session, request)): Path<(String, String, String)>,
    authority: Authority,
    Json(action): Json<CliWorkbenchDraftAction>,
) -> Result<Json<CliWorkbenchMessage>, WorkbenchError> {
    authority.require()?;
    let scope = project_scope(tenant, project)?;
    let mut workbench = WORKBENCH.lock();
    if workbench.session_mut(&scope, &session)?.info.capabilities.delivery
        != CliWorkbenchDeliveryCapability::Draft
    {
        return Err(not_found());
    }
    let delivery = workbench
        .deliveries
        .get_mut(&(scope, request))
        .filter(|entry| entry.updates.borrow().id.target_session_id == session)
        .ok_or_else(not_found)?;
    resolve_draft(delivery, action).map(Json)
}

fn resolve_draft(
    delivery: &mut Delivery,
    action: CliWorkbenchDraftAction,
) -> Result<CliWorkbenchMessage, WorkbenchError> {
    let (state, error_code) = match action {
        CliWorkbenchDraftAction::MarkSubmitted => (MessageState::Submitted, None),
        CliWorkbenchDraftAction::Cancel => (MessageState::Failed, Some(ErrorCode::Cancelled)),
    };
    if let Some(draft) = delivery.draft.take() {
        draft.updates.send_modify(|message| {
            message.revision += 1;
            message.state = state;
            message.error = error_code;
        });
    }
    let message = delivery.updates.borrow();
    if message.state != state || message.error != error_code {
        return Err(error(
            StatusCode::CONFLICT,
            ErrorCode::RequestConflict,
            "draft already resolved or unavailable",
        ));
    }
    Ok(message.clone())
}
