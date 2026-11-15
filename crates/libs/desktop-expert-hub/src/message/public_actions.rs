use super::*;

/// 校验详情目标、项目范围和安装目标后开始一次安装请求。
pub fn begin_install(
    state: &mut PublicSkillMarketState,
    project_path: Option<&str>,
) -> Result<PublicRequest, SkillHubError> {
    let detail = state
        .detail
        .as_ref()
        .ok_or_else(|| SkillHubError::localized("请先打开技能或套件详情", vec![]))?;
    let project_missing = PublicSkillMarketState::install_availability(project_path)
        == PublicInstallAvailability::NoProject;
    match &detail.target {
        PublicDetailTarget::Skill(_) if detail.install_scope == SkillInstallScope::Project => {
            if project_missing {
                return Err(SkillHubError::localized("项目级安装需要先选择项目", vec![]));
            }
        }
        PublicDetailTarget::Suite { .. } if project_missing => {
            return Err(SkillHubError::localized("请先选择项目再安装技能", vec![]));
        }
        _ => {}
    }
    if matches!(detail.install, PublicInstallState::Installing { .. }) {
        return Err(SkillHubError::localized("安装正在进行中", vec![]));
    }
    if matches!(detail.install, PublicInstallState::Installed { .. }) {
        return Err(SkillHubError::localized("所选目标已完成安装，请调整选择后重试", vec![]));
    }
    let target = detail.target.clone();
    let install_target = match &target {
        PublicDetailTarget::Skill(locator) => {
            let LoadState::Loaded(skill_detail) = &detail.skill_detail else {
                return Err(SkillHubError::localized("技能详情尚未加载完成", vec![]));
            };
            if skill_detail.skill.coordinate.locator != *locator {
                return Err(SkillHubError::localized("技能详情坐标与当前请求不一致", vec![]));
            }
            let installations = detail
                .selected_install_targets
                .iter()
                .copied()
                .map(|target| install_selection(target, detail.install_scope))
                .collect::<Vec<_>>();
            if installations.is_empty() {
                return Err(SkillHubError::localized("请至少选择一个安装目标", vec![]));
            }
            Some((skill_detail.skill.coordinate.clone(), installations))
        }
        PublicDetailTarget::Suite { .. } => {
            if !matches!(detail.suite_detail, LoadState::Loaded(_)) {
                return Err(SkillHubError::localized("套件详情尚未加载完成", vec![]));
            }
            None
        }
    };
    let project_path =
        project_path.map(str::trim).filter(|path| !path.is_empty()).unwrap_or_default().to_string();

    state.install_generation = state.install_generation.wrapping_add(1);
    let generation = state.install_generation;
    state.detail.as_mut().expect("detail was validated").install =
        PublicInstallState::Installing { generation };
    match (target.clone(), install_target) {
        (PublicDetailTarget::Skill(_), Some((coordinate, installations))) => {
            Ok(PublicRequest::InstallSkill {
                project_path,
                coordinate,
                installations,
                target,
                generation,
            })
        }
        (PublicDetailTarget::Suite { source, slug }, None) => {
            Ok(PublicRequest::InstallSuite { project_path, source, slug, target, generation })
        }
        _ => Err(SkillHubError::localized("详情安装状态无效", vec![])),
    }
}

pub(super) fn install_selection(
    target: SkillInstallTarget,
    scope: SkillInstallScope,
) -> SkillMarketInstallSelectionDto {
    let target = match target {
        SkillInstallTarget::VibeWindow => SkillMarketInstallTargetDto::VibeWindow,
        SkillInstallTarget::AgentSkills => SkillMarketInstallTargetDto::AgentSkills,
        SkillInstallTarget::Codex => SkillMarketInstallTargetDto::Codex,
        SkillInstallTarget::WorkBuddy => SkillMarketInstallTargetDto::WorkBuddy,
        SkillInstallTarget::ClaudeCode => SkillMarketInstallTargetDto::ClaudeCode,
        SkillInstallTarget::OpenCode => SkillMarketInstallTargetDto::OpenCode,
        SkillInstallTarget::GeminiCli => SkillMarketInstallTargetDto::GeminiCli,
        SkillInstallTarget::GitHubCopilot => SkillMarketInstallTargetDto::GitHubCopilot,
        SkillInstallTarget::Cursor => SkillMarketInstallTargetDto::Cursor,
        SkillInstallTarget::Windsurf => SkillMarketInstallTargetDto::Windsurf,
    };
    let scope = match scope {
        SkillInstallScope::Global => SkillMarketInstallScopeDto::Global,
        SkillInstallScope::Project => SkillMarketInstallScopeDto::Project,
    };
    SkillMarketInstallSelectionDto { target, scope }
}

/// 为当前选中的已安装技能启动批量启停或删除操作。
pub fn begin_batch_action(
    state: &mut PublicSkillMarketState,
    project_path: Option<&str>,
    kind: PublicBatchActionKind,
) -> Result<Vec<PublicRequest>, SkillHubError> {
    let snapshot = state
        .start_batch_action(kind)
        .ok_or_else(|| SkillHubError::localized("请先选择可管理的已安装技能", vec![]))?;
    Ok(batch_requests(
        project_path.unwrap_or_default(),
        snapshot.generation,
        snapshot.kind,
        snapshot.skill_ids,
    ))
}

pub(super) fn begin_action_for_ids(
    state: &mut PublicSkillMarketState,
    project_path: Option<&str>,
    kind: PublicBatchActionKind,
    skill_ids: Vec<String>,
) -> Result<Vec<PublicRequest>, SkillHubError> {
    if state.batch_action.is_some() {
        return Err(SkillHubError::localized("已有批量操作正在进行", vec![]));
    }
    let manageable = match &state.installed {
        LoadState::Loaded(items) => skill_ids.iter().all(|skill_id| {
            items
                .iter()
                .any(|item| item.id == *skill_id && item.installed && item.source_path.is_some())
        }),
        _ => false,
    };
    if skill_ids.is_empty() || !manageable {
        return Err(SkillHubError::localized("只能操作 gateway 判定可管理的本地技能", vec![]));
    }
    state.batch_generation = state.batch_generation.wrapping_add(1);
    let generation = state.batch_generation;
    state.batch_action = Some(PublicBatchActionState {
        generation,
        kind,
        pending: skill_ids.iter().cloned().collect(),
        succeeded: 0,
        failures: std::collections::BTreeMap::new(),
        preserve_selection: true,
    });
    state.last_batch_summary = None;
    Ok(batch_requests(project_path.unwrap_or_default(), generation, kind, skill_ids))
}

pub(super) fn batch_requests(
    project_path: &str,
    generation: u64,
    kind: PublicBatchActionKind,
    skill_ids: Vec<String>,
) -> Vec<PublicRequest> {
    skill_ids
        .into_iter()
        .map(|skill_id| match kind {
            PublicBatchActionKind::Enable | PublicBatchActionKind::Disable => {
                PublicRequest::SetSkillEnabled {
                    project_path: project_path.to_string(),
                    skill_id,
                    enabled: kind == PublicBatchActionKind::Enable,
                    generation,
                }
            }
            PublicBatchActionKind::Delete => PublicRequest::DeleteSkill {
                project_path: project_path.to_string(),
                skill_id,
                generation,
            },
        })
        .collect()
}

pub(super) fn finish_batch_item(
    state: &mut PublicSkillMarketState,
    generation: u64,
    project_path: String,
    skill_id: String,
    result: Result<(), String>,
) -> Vec<PublicRequest> {
    let Some(action) = state.batch_action.as_mut() else {
        return Vec::new();
    };
    if action.generation != generation || !action.pending.remove(&skill_id) {
        return Vec::new();
    }
    match result {
        Ok(()) => action.succeeded = action.succeeded.saturating_add(1),
        Err(error) => {
            action.failures.insert(skill_id, error);
        }
    }
    if !action.pending.is_empty() {
        return Vec::new();
    }

    let summary = PublicBatchSummary { succeeded: action.succeeded, failed: action.failures.len() };
    let preserve_selection = action.preserve_selection;
    let failures = action.failures.keys().cloned().collect();
    state.last_batch_summary = Some(summary);
    state.batch_action = None;
    if !preserve_selection {
        state.installed_selection = failures;
    }
    state.installed_generation = state.installed_generation.wrapping_add(1);
    let generation = state.installed_generation;
    state.installed = LoadState::Loading { generation };
    vec![PublicRequest::RefreshInstalled { project_path, generation }]
}
