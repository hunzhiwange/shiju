//! 处理公共技能市场的检索、详情、安装与批量操作消息。

use gateway_skill_market::{
    SkillMarketCategoryDto,
    SkillMarketCoordinateDto,
    SkillMarketInstallScopeDto,
    SkillMarketInstallSelectionDto,
    SkillMarketInstallTargetDto,
    SkillMarketSkillDetailDto,
    SkillMarketSkillDto,
    SkillMarketSkillFileDto,
    SkillMarketSkillLocatorDto,
    SkillMarketSkillPageDto,
    SkillMarketSkillQuery,
    SkillMarketSkillSetDetailDto,
    SkillMarketSkillSetPageDto,
    SkillMarketSort,
    SkillMarketSourceDto,
};
use desktop_skill_hub::skill_hub::SkillHubError;
use iced::Task;
use gateway_desktop_settings::{DesktopSkillCatalogEntryDto};


use super::public_gateway::*;
use desktop_skill_hub::skill_hub::{
    LoadState, PublicBatchActionKind, PublicBatchActionState, PublicBatchSummary, PublicDetailMode,
    PublicDetailTarget, PublicInstallAvailability, PublicInstallState, PublicSkillMarketState,
    PublicSkillTab, SkillInstallScope, SkillInstallTarget, parse_public_markdown,
};
use super::{SkillUpdate, Message, ToastKind, Effect};

use super::skill_hub::SkillHubMessage;

#[path = "public_reduce.rs"]
mod reducer;
#[path = "public_actions.rs"]
mod actions;
#[path = "public_dispatch.rs"]
mod dispatcher;
pub use reducer::reduce;
pub use actions::{begin_install, begin_batch_action};
use reducer::install_response_is_current;
use actions::{begin_action_for_ids, finish_batch_item};
use dispatcher::dispatch;

const MARKET_PAGE_SIZE: u32 = 24;

#[derive(Debug, Clone)]
#[allow(dead_code)]
/// 公共技能市场的用户输入、加载结果与安装操作消息。
pub enum PublicSkillMessage {
    LoadSources,
    SourcesLoaded {
        generation: u64,
        result: Result<Vec<SkillMarketSourceDto>, String>,
    },
    SelectSource(String),
    SelectTab(PublicSkillTab),
    RotateFeatured,
    QueryDraftChanged(String),
    SubmitQuery,
    SelectCategory(Option<String>),
    SelectSuiteCategory(Option<String>),
    SelectSort(SkillMarketSort),
    PreviousPage,
    NextPage,
    Retry,
    OpenInstalled,
    OpenSkillDetail(SkillMarketSkillLocatorDto),
    OpenSuiteDetail {
        source: String,
        slug: String,
    },
    CloseDetail,
    SetDetailMode(PublicDetailMode),
    SelectDetailInstallScope(SkillInstallScope),
    SelectDetailInstallProject(String),
    ToggleDetailInstallTarget(SkillInstallTarget),
    OpenExpertInstall,
    CancelExpertInstall,
    InstallCurrent,
    SelectExpertForChat,
    SkillDetailLoaded {
        locator: SkillMarketSkillLocatorDto,
        generation: u64,
        result: Result<SkillMarketSkillDetailDto, String>,
    },
    SkillFileLoaded {
        locator: SkillMarketSkillLocatorDto,
        generation: u64,
        result: Result<SkillMarketSkillFileDto, String>,
    },
    SuiteDetailLoaded {
        source: String,
        slug: String,
        generation: u64,
        result: Result<SkillMarketSkillSetDetailDto, String>,
    },
    InstallFinished {
        target: PublicDetailTarget,
        generation: u64,
        project_path: String,
        result: Result<Vec<String>, String>,
    },
    InstalledRefreshed {
        generation: u64,
        result: Result<Vec<DesktopSkillCatalogEntryDto>, String>,
    },
    CloseInstalled,
    SetInstalledSelected {
        skill_id: String,
        selected: bool,
    },
    SelectAllInstalled,
    ClearInstalledSelection,
    StartBatchEnabled(bool),
    ToggleInstalled {
        skill_id: String,
        enabled: bool,
    },
    RequestBatchDelete,
    CancelBatchDelete,
    ConfirmBatchDelete,
    BatchItemFinished {
        generation: u64,
        project_path: String,
        skill_id: String,
        result: Result<(), String>,
    },
    RecommendedLoaded {
        source: String,
        generation: u64,
        result: Result<Vec<SkillMarketSkillDto>, String>,
    },
    CategoriesLoaded {
        source: String,
        generation: u64,
        result: Result<Vec<SkillMarketCategoryDto>, String>,
    },
    MarketLoaded {
        source: String,
        generation: u64,
        result: Result<SkillMarketSkillPageDto, String>,
    },
    SuitesLoaded {
        source: String,
        generation: u64,
        result: Result<SkillMarketSkillSetPageDto, String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 归约器产生的网关请求描述；实际异步执行与状态变更保持分离。
pub enum PublicRequest {
    Sources {
        generation: u64,
    },
    Recommended {
        source: String,
        generation: u64,
    },
    Categories {
        source: String,
        generation: u64,
    },
    Market {
        source: String,
        generation: u64,
        query: SkillMarketSkillQuery,
    },
    Suites {
        source: String,
        generation: u64,
        query: SkillMarketSkillQuery,
    },
    SkillDetail {
        locator: SkillMarketSkillLocatorDto,
        generation: u64,
    },
    SkillFile {
        locator: SkillMarketSkillLocatorDto,
        generation: u64,
    },
    SuiteDetail {
        source: String,
        slug: String,
        generation: u64,
    },
    InstallSkill {
        project_path: String,
        coordinate: SkillMarketCoordinateDto,
        installations: Vec<SkillMarketInstallSelectionDto>,
        target: PublicDetailTarget,
        generation: u64,
    },
    InstallSuite {
        project_path: String,
        source: String,
        slug: String,
        target: PublicDetailTarget,
        generation: u64,
    },
    RefreshInstalled {
        project_path: String,
        generation: u64,
    },
    SetSkillEnabled {
        project_path: String,
        skill_id: String,
        enabled: bool,
        generation: u64,
    },
    DeleteSkill {
        project_path: String,
        skill_id: String,
        generation: u64,
    },
}

/// 归约公共市场消息，并批量派发其产生的网关请求。
pub fn update(app: &mut SkillUpdate<'_>, message: PublicSkillMessage) -> Task<Message> {
    if matches!(message, PublicSkillMessage::SelectExpertForChat) {
        let skill_id = match selectable_expert_skill(&app.expert_hub.public_market, app.project_path.as_deref(), app.has_project_chat) {
            Ok(id) => id,
            Err(reason) => {
                return app.show_localized_toast(
                    ToastKind::Warning,
                    reason,
                    vec![],
                );
            }
        };
        app.effects.push(Effect::SelectExpertForChat(skill_id));
        return Task::none();
    }
    if let PublicSkillMessage::SelectDetailInstallProject(project_path) = &message {
        let project_path = project_path.trim();
        if project_path.is_empty() || !known_install_project(app, project_path) {
            return app.show_localized_toast(
                ToastKind::Warning,
                "请选择当前或最近使用的项目",
                vec![],
            );
        }
        let Some(detail) = app.expert_hub.public_market.detail.as_mut() else {
            return Task::none();
        };
        if detail.install_is_busy() {
            return app.show_localized_toast(
                ToastKind::Warning,
                "安装进行中，暂时无法切换项目",
                vec![],
            );
        }
        if !detail.select_install_project_path(project_path.to_string()) {
            return Task::none();
        }
        app.expert_hub.public_market.installed_generation =
            app.expert_hub.public_market.installed_generation.wrapping_add(1);
        let generation = app.expert_hub.public_market.installed_generation;
        app.expert_hub.public_market.installed = LoadState::Loading { generation };
        return dispatch(app.gateway, PublicRequest::RefreshInstalled {
            project_path: project_path.to_string(),
            generation,
        });
    }
    if matches!(message, PublicSkillMessage::InstallCurrent) {
        if app.expert_hub.section == desktop_skill_hub::skill_hub::ExpertHubSection::Experts {
            let valid = app.expert_hub.public_market.detail.as_ref().is_some_and(|detail| {
                matches!(detail.target, PublicDetailTarget::Suite { .. })
                    && detail.install_confirmation_open
                    && detail.selected_install_project_path.as_deref().is_some_and(|path| {
                        known_install_project(app, path) && !path.trim().is_empty()
                    })
            });
            if !valid {
                return app.show_localized_toast(
                    ToastKind::Warning,
                    "请在安装弹窗选择当前或最近使用的项目",
                    vec![],
                );
            }
        }
        let project_path = app
            .expert_hub
            .public_market
            .detail
            .as_ref()
            .and_then(|detail| detail.selected_install_project_path.clone());
        return match begin_install(&mut app.expert_hub.public_market, project_path.as_deref()) {
            Ok(request) => {
                if let Some(detail) = app.expert_hub.public_market.detail.as_mut() {
                    detail.install_confirmation_open = false;
                }
                dispatch(app.gateway, request)
            }
            Err(error) => {
                super::skill_hub::show_skill_error(app, ToastKind::Warning, None, error)
            }
        };
    }
    if let PublicSkillMessage::StartBatchEnabled(enabled) = &message {
        return match begin_batch_action(
            &mut app.expert_hub.public_market,
            app.project_path.as_deref(),
            if *enabled { PublicBatchActionKind::Enable } else { PublicBatchActionKind::Disable },
        ) {
            Ok(requests) => Task::batch(requests.into_iter().map(|request| dispatch(app.gateway, request))),
            Err(error) => {
                super::skill_hub::show_skill_error(app, ToastKind::Warning, None, error)
            }
        };
    }
    if let PublicSkillMessage::ToggleInstalled { skill_id, enabled } = &message {
        return match begin_action_for_ids(
            &mut app.expert_hub.public_market,
            app.project_path.as_deref(),
            if *enabled { PublicBatchActionKind::Enable } else { PublicBatchActionKind::Disable },
            vec![skill_id.clone()],
        ) {
            Ok(requests) => Task::batch(requests.into_iter().map(|request| dispatch(app.gateway, request))),
            Err(error) => {
                super::skill_hub::show_skill_error(app, ToastKind::Warning, None, error)
            }
        };
    }
    if matches!(message, PublicSkillMessage::ConfirmBatchDelete) {
        if !app.expert_hub.public_market.delete_confirmation_open {
            return Task::none();
        }
        app.expert_hub.public_market.delete_confirmation_open = false;
        return match begin_batch_action(
            &mut app.expert_hub.public_market,
            app.project_path.as_deref(),
            PublicBatchActionKind::Delete,
        ) {
            Ok(requests) => Task::batch(requests.into_iter().map(|request| dispatch(app.gateway, request))),
            Err(error) => {
                super::skill_hub::show_skill_error(app, ToastKind::Warning, None, error)
            }
        };
    }
    let install_feedback = match &message {
        PublicSkillMessage::InstallFinished { target, generation, result, .. }
            if install_response_is_current(&app.expert_hub.public_market, target, *generation) =>
        {
            Some((
                matches!(target, PublicDetailTarget::Suite { .. }),
                result.as_ref().map(|_| ()).map_err(Clone::clone),
            ))
        }
        _ => None,
    };
    let refresh_error = match &message {
        PublicSkillMessage::InstalledRefreshed { generation, result }
            if *generation == app.expert_hub.public_market.installed_generation =>
        {
            result.as_ref().err().cloned()
        }
        _ => None,
    };
    let batch_feedback = match &message {
        PublicSkillMessage::BatchItemFinished { generation, result, .. } => app
            .expert_hub
            .public_market
            .batch_action
            .as_ref()
            .filter(|action| action.generation == *generation && action.pending.len() == 1)
            .map(|action| {
                let succeeded = action.succeeded + usize::from(result.is_ok());
                let failed = action.failures.len() + usize::from(result.is_err());
                PublicBatchSummary { succeeded, failed }
            }),
        _ => None,
    };
    let open_installed = matches!(message, PublicSkillMessage::OpenInstalled);
    let opens_detail = matches!(
        &message,
        PublicSkillMessage::OpenSkillDetail(_) | PublicSkillMessage::OpenSuiteDetail { .. }
    );
    let default_project_path = default_install_project_path(app);
    let mut requests = reduce(&mut app.expert_hub.public_market, message);
    let detail_opened = opens_detail
        && requests.iter().any(|request| {
            matches!(request, PublicRequest::SkillDetail { .. } | PublicRequest::SuiteDetail { .. })
        });
    if detail_opened {
        if let Some(detail) = app.expert_hub.public_market.detail.as_mut() {
            detail.selected_install_project_path = default_project_path.clone();
        }
    }
    if detail_opened && let Some(project_path) = default_project_path {
        app.expert_hub.public_market.installed_generation =
            app.expert_hub.public_market.installed_generation.wrapping_add(1);
        let generation = app.expert_hub.public_market.installed_generation;
        app.expert_hub.public_market.installed = LoadState::Loading { generation };
        requests.push(PublicRequest::RefreshInstalled { project_path, generation });
    }
    if open_installed {
        app.expert_hub.public_market.installed_generation =
            app.expert_hub.public_market.installed_generation.wrapping_add(1);
        let generation = app.expert_hub.public_market.installed_generation;
        app.expert_hub.public_market.installed = LoadState::Loading { generation };
        requests.push(PublicRequest::RefreshInstalled {
            project_path: app.project_path.clone().unwrap_or_default(),
            generation,
        });
    }
    let mut tasks = requests.into_iter().map(|request| dispatch(app.gateway, request)).collect::<Vec<_>>();
    if let Some((expert, result)) = install_feedback {
        tasks.push(match result {
            Ok(()) if expert => app.show_localized_toast(
                ToastKind::Success,
                "专家包安装成功",
                vec![],
            ),
            Ok(()) => app.show_localized_toast(
                ToastKind::Success,
                "技能安装成功",
                vec![],
            ),
            Err(error) if expert => app.show_localized_toast(
                ToastKind::Error,
                "专家包安装失败：%s",
                vec![error.to_string().into()],
            ),
            Err(error) => app.show_localized_toast(
                ToastKind::Error,
                "技能安装失败：%s",
                vec![error.to_string().into()],
            ),
        });
    }
    if let Some(error) = refresh_error {
        tasks.push(app.show_localized_toast(
            ToastKind::Warning,
            "技能已安装，但刷新本地目录失败：%s",
            vec![error.to_string().into()],
        ));
    }
    if let Some(summary) = batch_feedback {
        tasks.push(if summary.failed == 0 {
            app.show_localized_toast(
                ToastKind::Success,
                "批量操作完成：成功 %u 项",
                vec![summary.succeeded.into()],
            )
        } else {
            app.show_localized_toast(
                ToastKind::Warning,
                "批量操作完成：成功 %u 项，失败 %u 项",
                vec![summary.succeeded.into(), summary.failed.into()],
            )
        });
    }
    Task::batch(tasks)
}

pub fn selectable_expert_skill(state: &PublicSkillMarketState, project_path: Option<&str>, has_project_chat: bool) -> Result<String, &'static str> {
    let detail = state.detail.as_ref().ok_or("请先打开专家详情")?;
    let desktop_skill_hub::skill_hub::PublicDetailTarget::Suite { source, slug } = &detail.target else {
        return Err("当前详情不是专家包");
    };
    if !matches!(detail.suite_detail, LoadState::Loaded(_)) {
        return Err("专家包详情尚未加载完成");
    }
    let project_path = project_path.ok_or("请先打开项目聊天")?;
    if !has_project_chat {
        return Err("请先打开当前项目的聊天标签");
    }
    if detail.selected_install_project_path.as_deref() != Some(project_path) {
        return Err("专家包安装项目与当前聊天项目不一致");
    }
    let expected = format!("{source}--skillset--{slug}");
    match &state.installed {
        LoadState::Loaded(items) => items
            .iter()
            .find(|item| {
                item.id == expected
                    && item.installed
                    && item.enabled
                    && item.source == *source
                    && item.namespace.as_deref() == Some("skillset")
                    && item.package_type.as_deref() == Some("meta-skill")
                    && item.source_pack.as_deref() == Some(slug.as_str())
            })
            .map(|item| item.id.clone())
            .ok_or("请先在当前项目安装并启用该专家包"),
        _ => Err("当前项目的安装状态尚未确认"),
    }
}

/// 在公共市场尚未初始化时加载来源与当前页，已加载时保持状态不变。
pub fn ensure_loaded(app: &mut SkillUpdate<'_>) -> Task<Message> {
    if matches!(app.expert_hub.public_market.sources, LoadState::Idle | LoadState::Error(_)) {
        update(app, PublicSkillMessage::LoadSources)
    } else {
        Task::none()
    }
}

fn default_install_project_path(app: &SkillUpdate<'_>) -> Option<String> {
    app.project_path
        .iter()
        .chain(app.recent_projects.iter())
        .map(|path| path.trim())
        .find(|path| !path.is_empty())
        .map(str::to_string)
}

fn known_install_project(app: &SkillUpdate<'_>, project_path: &str) -> bool {
    app.project_path
        .iter()
        .chain(app.recent_projects.iter())
        .any(|known| known.trim() == project_path)
}

#[cfg(test)]
#[path = "public_tests.rs"]
mod tests;
