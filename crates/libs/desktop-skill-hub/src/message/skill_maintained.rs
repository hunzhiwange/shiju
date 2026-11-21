use super::*;

fn row<'a>(app: &'a SkillUpdate<'_>, id: u64) -> Option<&'a TenantSkill> {
    let LoadState::Loaded(page) = &app.expert_hub.mine else {
        return None;
    };
    page.items.iter().find(|skill| skill.id == id)
}

pub(super) fn edit(app: &mut SkillUpdate<'_>, id: u64) -> Task<Message> {
    if app.screen != HubPage::ShijuSkills
        || app.expert_hub.skill_tab != SkillTab::Mine
        || row(app, id).is_none()
    {
        return Task::none();
    }
    let url = format!("https://ai.shiju.cloud/skill-hub/my-skills/{id}/edit");
    if !crate::message::open_url(&url) {
        return app.feedback(
            ToastKind::Error,
            SkillHubError::localized("无法打开技能编辑页面", vec![]).message(),
        );
    }
    Task::none()
}

pub(super) fn open(
    app: &mut SkillUpdate<'_>,
    id: u64,
    kind: MaintainedSkillAction,
) -> Task<Message> {
    if app.screen != HubPage::ShijuSkills
        || app.expert_hub.skill_tab != SkillTab::Mine
        || app.expert_hub.maintained_action.is_some()
    {
        return Task::none();
    }
    let Some(skill) = row(app, id) else {
        return Task::none();
    };
    if matches!(kind, MaintainedSkillAction::Unpublish) && skill.status != "published"
        || matches!(kind, MaintainedSkillAction::Delete) && skill.status != "draft"
    {
        return Task::none();
    }
    let visibility = skill.tenant_visibility.as_ref();
    let scope = visibility.and_then(|value| value.scope_row.as_ref());
    let ids = |values: Option<&Vec<String>>| values.map(|ids| ids.join(", ")).unwrap_or_default();
    app.expert_hub.maintained_action = Some(MaintainedSkillActionState {
        skill_id: id,
        name: skill.name.clone(),
        kind,
        all: visibility.is_some_and(|value| value.effective_mode == "tenant_open"),
        department_ids: ids(scope.and_then(|scope| scope.department_ids.as_ref())),
        user_ids: ids(scope.and_then(|scope| scope.user_ids.as_ref())),
        pending: None,
        error: None,
    });
    Task::none()
}

pub(super) fn close(app: &mut SkillUpdate<'_>) -> Task<Message> {
    if app.expert_hub.maintained_action.as_ref().is_some_and(|state| state.pending.is_none()) {
        app.expert_hub.maintained_action = None;
    }
    Task::none()
}

pub(super) fn change(
    app: &mut SkillUpdate<'_>,
    apply: impl FnOnce(&mut MaintainedSkillActionState),
) -> Task<Message> {
    if let Some(state) = app.expert_hub.maintained_action.as_mut()
        && state.kind == MaintainedSkillAction::Visibility
        && state.pending.is_none()
    {
        apply(state);
    }
    Task::none()
}

pub(super) fn submit(app: &mut SkillUpdate<'_>) -> Task<Message> {
    let Some(account) = app.active_account else {
        return Task::none();
    };
    let account_id = account.id().to_string();
    if app.expert_hub.account_id.as_deref() != Some(account_id.as_str()) {
        return Task::none();
    }
    let Some(state) =
        app.expert_hub.maintained_action.as_mut().filter(|state| state.pending.is_none())
    else {
        return Task::none();
    };
    let credential = account.skill_hub_credential();
    let gateway = app.gateway;
    let skill_id = state.skill_id;
    let kind = state.kind;
    let all = state.all;
    let parse_ids = |input: &str| -> Vec<String> {
        input
            .split([',', '，'])
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(String::from)
            .collect()
    };
    let department_ids = parse_ids(&state.department_ids);
    let mut user_ids: Vec<String> = parse_ids(&state.user_ids);
    app.expert_hub.action_generation = app.expert_hub.action_generation.wrapping_add(1);
    let generation = app.expert_hub.action_generation;
    state.pending = Some(generation);
    state.error = None;
    Task::perform(
        async move {
            if kind == MaintainedSkillAction::Visibility && !all {
                let account_client = gateway_shiju_account::GatewayShijuAccountClient::new(
                    gateway().map_err(SkillHubError::from)?,
                );
                let user_id = account_client
                    .shiju_account_me(
                        &credential
                            .gateway_credential()
                            .map_err(|error| SkillHubError::from(error.to_string()))?,
                    )
                    .await
                    .map_err(SkillHubError::from)?
                    .user_id;
                if user_id.trim().is_empty() {
                    return Err(SkillHubError::localized("无法确认当前员工身份", vec![]));
                }
                if !user_ids.contains(&user_id) {
                    user_ids.push(user_id);
                }
            }
            SkillHubClient::new(credential)?
                .mutate_maintained_skill(skill_id, kind, all, department_ids, user_ids)
                .await
        },
        move |result| {
            Message::SkillHub(SkillHubMessage::Catalog(
                SkillCatalogMessage::MaintainedSkillActionFinished {
                    generation,
                    account_id,
                    skill_id,
                    result,
                },
            ))
        },
    )
}

pub(super) fn finish(
    app: &mut SkillUpdate<'_>,
    generation: u64,
    account_id: &str,
    skill_id: u64,
    result: Result<(), SkillHubError>,
) -> Task<Message> {
    if app.expert_hub.account_id.as_deref() != Some(account_id) {
        return Task::none();
    }
    let Some(state) = app
        .expert_hub
        .maintained_action
        .as_mut()
        .filter(|state| state.skill_id == skill_id && state.pending == Some(generation))
    else {
        return Task::none();
    };
    if let Err(error) = result {
        state.pending = None;
        state.error = Some(error);
        return Task::none();
    }
    let deleted = state.kind == MaintainedSkillAction::Delete;
    app.expert_hub.maintained_action = None;
    if app.expert_hub.skill_tab == SkillTab::Mine {
        if deleted
            && matches!(&app.expert_hub.mine,
            LoadState::Loaded(page) if page.page > 1 && page.items.len() == 1)
        {
            app.expert_hub.mine_page -= 1;
        }
        load_current_tab(app)
    } else {
        app.expert_hub.mine = LoadState::Idle;
        Task::none()
    }
}
