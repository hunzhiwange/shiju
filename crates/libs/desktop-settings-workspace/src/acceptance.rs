//! Domain state for explicit native worktree rendering.
use crate::state::*;
use vw_api_worktree::WorktreeDeletionPreflight;
pub fn setup(state: &mut WorktreeSettingsState, case: usize) {
    *state = Default::default();
    state.saved = Some(Default::default());
    state.default_root = "/项目/中文路径/工作树".into();
    state.managed.groups = vec![ManagedWorktreeGroup {
            project: serde_json::from_value(serde_json::json!({
                "id":"fixture", "name":"中文项目与长标题", "directory":"/项目/中文路径",
                "display_path":"/项目/中文路径", "status":"ready", "created_at_ms":1,"updated_at_ms":2,
                "git":{"is_repo":true,"has_uncommitted_changes":false}
            })).unwrap(),
            items: vec![serde_json::from_value(serde_json::json!({
                "worktree":{"id":"checkout", "project_id":"fixture","name":"中文工作树", "branch":"feature",
                    "directory":"/工作树/很长的中文路径用于验证换行和字形整形/原目录", "status":"error", "created_at_ms":1,"updated_at_ms":2},
                "title":"中文聊天标题，用于检查很长的动态文本是否安全渲染", "sessions":[],"protection_reasons":[], "recovered_from":null
            })).unwrap()], collapsed:false, error:None,
        }];
    state.recovery.restored =
        Some(("fixture".into(), "unavailable".into(), "/项目/已恢复的中文路径".into()));

    match case % 5 {
        1 => {
            state.active_tab = WorktreesTab::Environment;
            state.environment.loaded = true;
            state.environment.selected = Some(serde_json::from_value(serde_json::json!({
                    "id":"fixture", "name":"中文项目与很长很长的环境初始化标题", "directory":"/项目/中文路径",
                    "display_path":"/项目/中文路径", "status":"ready", "created_at_ms":1,"updated_at_ms":2,
                    "git":{"is_repo":true,"has_uncommitted_changes":false}
                })).unwrap());
            state.environment.editor =
                iced::widget::text_editor::Content::with_text("echo 中文环境初始化\necho ready");
        }
        2 => {
            state.managed.preflight_target = Some(("中文项目".into(), "长标题工作树".repeat(8)));
            state.managed.preflight = Some(WorktreeDeletionPreflight {
                fingerprint: "fixture".into(),
                expires_at_ms: u64::MAX,
                confirmation_required: true,
                has_changes: true,
                has_ignored_files: true,
                protection_reasons: vec![],
            });
            state.recovery.error = Some("二进制修改将保存到恢复快照；忽略文件需要额外确认".into());
        }
        3 => {
            state.draft = Some(config_worktree::WorktreeSettings::default().into());
        }
        _ => {}
    }
}

/// Render the real domain page and overlays with native acceptance scroll bounds.
pub fn view(
    state: &WorktreeSettingsState,
    i18n: desktop_i18n::I18n,
    local: bool,
    case: usize,
) -> iced::Element<'_, crate::messages::WorktreeSettingsMessage> {
    use crate::system_settings_worktrees as worktrees;
    use iced::{Length, widget::container};
    let page =
        iced::widget::scrollable(container(worktrees::page(state, i18n, local)).padding([0, 7]))
            .id("worktree-window-page")
            .direction(iced::widget::scrollable::Direction::Vertical(
                iced::widget::scrollable::Scrollbar::new().width(4).scroller_width(4),
            ))
            .height(Length::Fill);
    let base = container(page).padding(14).center_x(Length::Fill).center_y(Length::Fill).into();
    let base = worktrees::editor_with(state, i18n, base, |event| event);
    let base = worktrees::environment::editor_with(&state.environment, i18n, base, |event| event);
    let base = worktrees::overlay_with(state, base, |event| event);
    iced::widget::keyed::Column::with_children([(case, base)])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
