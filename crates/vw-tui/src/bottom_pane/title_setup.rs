//! 自定义终端窗口/标签标题的配置视图
//! Terminal title configuration view for customizing the terminal window/tab title.
//!
//! This module provides an interactive picker for selecting which items appear
//! in the terminal title. Users can:
//!
//! - Select items
//! - Reorder items
//! - Preview the rendered title

use itertools::Itertools;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Line;
use strum::IntoEnumIterator;
use strum_macros::Display;
use strum_macros::EnumIter;
use strum_macros::EnumString;

use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;
use crate::bottom_pane::ACTION_REQUIRED_PREVIEW_PREFIX;
use crate::bottom_pane::CancellationEvent;
use crate::bottom_pane::bottom_pane_view::BottomPaneView;
use crate::bottom_pane::build_action_required_title_text;
use crate::bottom_pane::multi_select_picker::MultiSelectItem;
use crate::bottom_pane::multi_select_picker::MultiSelectPicker;
use crate::bottom_pane::status_surface_preview::StatusSurfacePreviewData;
use crate::bottom_pane::status_surface_preview::StatusSurfacePreviewItem;
use crate::keymap::ListKeymap;
use crate::render::renderable::Renderable;

/// 通过strum将变体序列化并持久化到用户配置文件，因此更改或移除变体会导致配置变更。
/// Available items that can be displayed in the terminal title.
///
/// Variants serialize to kebab-case identifiers (e.g. `AppName` -> `"app-name"`)
/// via strum. These identifiers are persisted in user config files, so renaming
/// or removing a variant is a breaking config change.
#[derive(EnumIter, EnumString, Display, Debug, Clone, Copy, Eq, PartialEq, Hash)]
#[strum(serialize_all = "kebab-case")]
pub(crate) enum TerminalTitleItem {
    /// Codex 应用程序名称
    /// Codex app name.
    AppName,
    /// 项目根目录名称，或作为短路径的 cwd 降级。
    /// Project root name, or a compact cwd fallback.
    #[strum(to_string = "project-name", serialize = "project")]
    Project,
    /// 当前工作目录路径
    /// Current working directory path.
    CurrentDir,
    /// 活动指示器，用于显示终端标题状态
    /// Terminal-title activity indicator while active.
    #[strum(to_string = "activity", serialize = "spinner")]
    Spinner,
    /// 紧凑运行时状态文本
    /// Compact runtime run-state text.
    #[strum(to_string = "run-state", serialize = "status")]
    Status,
    /// 当前线程标题（如有）
    /// Current thread title (if available).
    #[strum(to_string = "thread-title", serialize = "thread")]
    Thread,
    /// 当前 Git 分支（如有）
    /// Current git branch (if available).
    GitBranch,
    /// 剩余上下文窗口百分比
    /// Percentage of context window remaining.
    ContextRemaining,
    /// 上下文窗口使用的百分比
    /// Percentage of context window used.
    #[strum(to_string = "context-used", serialize = "context-usage")]
    ContextUsed,
    /// 剩余的主速率限制使用量。
    /// Remaining usage on the primary rate limit.
    FiveHourLimit,
    /// 剩余次级速率限制的使用量。
    /// Remaining usage on the secondary rate limit.
    WeeklyLimit,
    /// Codex 应用程序版本
    /// Codex application version.
    CodexVersion,
    /// 当前会话中使用的总令牌数
    /// Total tokens used in the current session.
    UsedTokens,
    /// 总消耗输入令牌数
    /// Total input tokens consumed.
    TotalInputTokens,
    /// 生成的总输出令牌数
    /// Total output tokens generated.
    TotalOutputTokens,
    /// 当前企业线程直接分配的估算信用额
    /// Estimated credits attributed directly to the current enterprise thread.
    ThreadCredits,
    /// 当前企业线程直接产生的估算美元成本
    /// Estimated dollar cost attributed directly to the current enterprise thread.
    EstimatedThreadCost,
    /// 全线程 UUID
    /// Full thread UUID.
    #[strum(to_string = "thread-id", serialize = "session-id")]
    SessionId,
    /// 当前是否处于快速模式
    /// Whether Fast mode is currently active.
    FastMode,
    /// 当前模型名称
    /// Current model name.
    #[strum(to_string = "model", serialize = "model-name")]
    Model,
    /// 当前模型名称及其推理级别
    /// Current model name with reasoning level.
    ModelWithReasoning,
    /// 当前推理级别
    /// Current reasoning level.
    Reasoning,
    /// 从 `update_plan` 获取的最新检查任务进度
    /// Latest checklist task progress from `update_plan` (if available).
    TaskProgress,
}

impl TerminalTitleItem {
    pub(crate) fn description(self) -> &'static str {
        match self {
            TerminalTitleItem::AppName => "Codex app name",
            TerminalTitleItem::Project => "Project name (falls back to current directory name)",
            TerminalTitleItem::CurrentDir => "Current working directory",
            TerminalTitleItem::Spinner => {
                "Spinner while working, action-required message while blocked."
            }
            TerminalTitleItem::Status => {
                "Compact session run-state text (Ready, Working, Thinking)"
            }
            TerminalTitleItem::Thread => "Current thread title, or thread identifier when unnamed",
            TerminalTitleItem::GitBranch => "Current Git branch (omitted when unavailable)",
            TerminalTitleItem::ContextRemaining => {
                "Percentage of context window remaining (omitted when unknown)"
            }
            TerminalTitleItem::ContextUsed => {
                "Percentage of context window used (omitted when unknown)"
            }
            TerminalTitleItem::FiveHourLimit => {
                "Remaining usage on the primary usage limit (omitted when unavailable)"
            }
            TerminalTitleItem::WeeklyLimit => {
                "Remaining usage on the secondary usage limit (omitted when unavailable)"
            }
            TerminalTitleItem::CodexVersion => "Codex application version",
            TerminalTitleItem::UsedTokens => "Total tokens used in session (omitted when zero)",
            TerminalTitleItem::TotalInputTokens => "Total input tokens used in session",
            TerminalTitleItem::TotalOutputTokens => "Total output tokens used in session",
            TerminalTitleItem::ThreadCredits => {
                "Estimated current-thread credits (Enterprise workspaces only; omitted when unavailable)"
            }
            TerminalTitleItem::EstimatedThreadCost => {
                "Estimated current-thread cost (Enterprise workspaces only; omitted when unavailable)"
            }
            TerminalTitleItem::SessionId => {
                "Current thread identifier (omitted until thread starts)"
            }
            TerminalTitleItem::FastMode => "Whether Fast mode is currently active",
            TerminalTitleItem::Model => "Current model name",
            TerminalTitleItem::ModelWithReasoning => "Current model name with reasoning level",
            TerminalTitleItem::Reasoning => "Current reasoning level",
            TerminalTitleItem::TaskProgress => {
                "Latest task progress from update_plan (omitted until available)"
            }
        }
    }

    pub(crate) fn preview_item(self) -> Option<StatusSurfacePreviewItem> {
        match self {
            TerminalTitleItem::AppName => Some(StatusSurfacePreviewItem::AppName),
            TerminalTitleItem::Project => Some(StatusSurfacePreviewItem::ProjectName),
            TerminalTitleItem::CurrentDir => Some(StatusSurfacePreviewItem::CurrentDir),
            TerminalTitleItem::Spinner => None,
            TerminalTitleItem::Status => Some(StatusSurfacePreviewItem::Status),
            TerminalTitleItem::Thread => Some(StatusSurfacePreviewItem::ThreadTitle),
            TerminalTitleItem::GitBranch => Some(StatusSurfacePreviewItem::GitBranch),
            TerminalTitleItem::ContextRemaining => Some(StatusSurfacePreviewItem::ContextRemaining),
            TerminalTitleItem::ContextUsed => Some(StatusSurfacePreviewItem::ContextUsed),
            TerminalTitleItem::FiveHourLimit => Some(StatusSurfacePreviewItem::FiveHourLimit),
            TerminalTitleItem::WeeklyLimit => Some(StatusSurfacePreviewItem::WeeklyLimit),
            TerminalTitleItem::CodexVersion => Some(StatusSurfacePreviewItem::CodexVersion),
            TerminalTitleItem::UsedTokens => Some(StatusSurfacePreviewItem::UsedTokens),
            TerminalTitleItem::TotalInputTokens => Some(StatusSurfacePreviewItem::TotalInputTokens),
            TerminalTitleItem::TotalOutputTokens => {
                Some(StatusSurfacePreviewItem::TotalOutputTokens)
            }
            TerminalTitleItem::ThreadCredits => Some(StatusSurfacePreviewItem::ThreadCredits),
            TerminalTitleItem::EstimatedThreadCost => {
                Some(StatusSurfacePreviewItem::EstimatedThreadCost)
            }
            TerminalTitleItem::SessionId => Some(StatusSurfacePreviewItem::SessionId),
            TerminalTitleItem::FastMode => Some(StatusSurfacePreviewItem::FastMode),
            TerminalTitleItem::Model => Some(StatusSurfacePreviewItem::Model),
            TerminalTitleItem::ModelWithReasoning => {
                Some(StatusSurfacePreviewItem::ModelWithReasoning)
            }
            TerminalTitleItem::Reasoning => Some(StatusSurfacePreviewItem::Reasoning),
            TerminalTitleItem::TaskProgress => Some(StatusSurfacePreviewItem::TaskProgress),
        }
    }

    /// 返回渲染标题前放置分隔符的分割值。
    /// Returns the separator to place before this item in a rendered title.
    ///
    /// The activity indicator gets a plain space on either side so it reads as
    /// `my-project <activity> Working` rather than `my-project | <activity> | Working`.
    /// All other adjacent items are joined with ` | `.
    pub(crate) fn separator_from_previous(self, previous: Option<Self>) -> &'static str {
        match previous {
            None => "",
            Some(previous)
                if previous == TerminalTitleItem::Spinner || self == TerminalTitleItem::Spinner =>
            {
                " "
            }
            Some(_) => " | ",
        }
    }
}

pub(crate) fn preview_line_for_title_items(
    items: &[TerminalTitleItem],
    preview_data: &StatusSurfacePreviewData,
) -> Option<Line<'static>> {
    if items.contains(&TerminalTitleItem::Spinner) {
        let preview = build_action_required_title_text(
            ACTION_REQUIRED_PREVIEW_PREFIX,
            items.iter().copied(),
            &[],
            |item| {
                item.preview_item()
                    .and_then(|preview_item| preview_data.value_for(preview_item))
                    .map(str::to_owned)
            },
        );
        return Some(Line::from(preview));
    }

    let mut previous = None;
    let preview = items
        .iter()
        .copied()
        .fold(String::new(), |mut preview, item| {
            let Some(value) = item
                .preview_item()
                .and_then(|preview_item| preview_data.value_for(preview_item))
            else {
                return preview;
            };
            preview.push_str(item.separator_from_previous(previous));
            preview.push_str(value);
            previous = Some(item);
            preview
        });
    if preview.is_empty() {
        None
    } else {
        Some(Line::from(preview))
    }
}

fn parse_terminal_title_items<T>(ids: impl Iterator<Item = T>) -> Option<Vec<TerminalTitleItem>>
where
    T: AsRef<str>,
{
    // 将解析视为全或全无，因此预览/确认回调不会发出部分解释的排序。在构建选择器时，无效 ID 会被忽略；但在用户与选择器交互时，我们只希望持久化或预览一个完全有效的选择。
    // Treat parsing as all-or-nothing so preview/confirm callbacks never emit
    // a partially interpreted ordering. Invalid ids are ignored when building
    // the picker, but once the user is interacting with the picker we only want
    // to persist or preview a fully valid selection.
    ids.map(|id| id.as_ref().parse::<TerminalTitleItem>())
        .collect::<Result<Vec<_>, _>>()
        .ok()
}

/// 交互式终端标题项配置视图
/// Interactive view for configuring terminal-title items.
pub(crate) struct TerminalTitleSetupView {
    picker: MultiSelectPicker,
}

impl TerminalTitleSetupView {
    /// 创建终端标题选择器，并保留配置项的原始顺序。
    /// Creates the terminal-title picker, preserving the configured item order first.
    ///
    /// Unknown configured ids are skipped here instead of surfaced inline. The
    /// main TUI still warns about them when rendering the actual title, but the
    /// picker itself only exposes the selectable items it can meaningfully
    /// preview and persist.
    pub(crate) fn new(
        title_items: Option<&[String]>,
        preview_data: StatusSurfacePreviewData,
        app_event_tx: AppEventSender,
        list_keymap: ListKeymap,
    ) -> Self {
        let selected_items = title_items
            .into_iter()
            .flatten()
            .filter_map(|id| id.parse::<TerminalTitleItem>().ok())
            .unique()
            .collect_vec();
        let selected_set = selected_items
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>();
        let items = selected_items
            .into_iter()
            .map(|item| Self::title_select_item(item, /*enabled*/ true, &preview_data))
            .chain(
                TerminalTitleItem::iter()
                    .filter(|item| !selected_set.contains(item))
                    .map(|item| {
                        Self::title_select_item(item, /*enabled*/ false, &preview_data)
                    }),
            )
            .collect();

        Self {
            picker: MultiSelectPicker::builder(
                "Configure Terminal Title".to_string(),
                Some("Select which items to display in the terminal title.".to_string()),
                app_event_tx,
            )
            .list_keymap(list_keymap)
            .items(items)
            .enable_ordering()
            .on_preview(move |items| {
                let items = parse_terminal_title_items(
                    items
                        .iter()
                        .filter(|item| item.enabled)
                        .map(|item| item.id.as_str()),
                )?;
                preview_line_for_title_items(&items, &preview_data)
            })
            .on_change(|items, app_event| {
                let Some(items) = parse_terminal_title_items(
                    items
                        .iter()
                        .filter(|item| item.enabled)
                        .map(|item| item.id.as_str()),
                ) else {
                    return;
                };
                app_event.send(AppEvent::TerminalTitleSetupPreview { items });
            })
            .on_confirm(|ids, app_event| {
                let Some(items) = parse_terminal_title_items(ids.iter().map(String::as_str)) else {
                    return;
                };
                app_event.send(AppEvent::TerminalTitleSetup { items });
            })
            .on_cancel(|app_event| {
                app_event.send(AppEvent::TerminalTitleSetupCancelled);
            })
            .build(),
        }
    }

    fn title_select_item(
        item: TerminalTitleItem,
        enabled: bool,
        preview_data: &StatusSurfacePreviewData,
    ) -> MultiSelectItem {
        let default_name = item.to_string();
        let default_description = item.description();
        let (name, description) = match item.preview_item() {
            Some(
                preview_item @ (StatusSurfacePreviewItem::FiveHourLimit
                | StatusSurfacePreviewItem::WeeklyLimit),
            ) => (
                preview_data.rate_limit_item_name(preview_item, &default_name),
                preview_data.rate_limit_item_description(preview_item, default_description),
            ),
            _ => (default_name, default_description.to_string()),
        };

        MultiSelectItem {
            id: item.to_string(),
            name,
            description: Some(description),
            enabled,
            orderable: true,
            section_break_after: false,
        }
    }
}

impl BottomPaneView for TerminalTitleSetupView {
    fn keymap_contexts(&self) -> crate::keymap::KeymapContextSet {
        crate::keymap::KeymapContextSet::new(crate::keymap::KeymapContext::List)
    }

    fn handle_key_event(&mut self, key_event: crossterm::event::KeyEvent) {
        self.picker.handle_key_event(key_event);
    }

    fn is_complete(&self) -> bool {
        self.picker.complete
    }

    fn on_ctrl_c(&mut self) -> CancellationEvent {
        self.picker.close();
        CancellationEvent::Handled
    }
}

impl Renderable for TerminalTitleSetupView {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.picker.render(area, buf);
    }

    fn desired_height(&self, width: u16) -> u16 {
        self.picker.desired_height(width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use pretty_assertions::assert_eq;
    use tokio::sync::mpsc::unbounded_channel;

    fn render_lines(view: &TerminalTitleSetupView, width: u16) -> String {
        let height = view.desired_height(width);
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        view.render(area, &mut buf);

        let lines: Vec<String> = (0..area.height)
            .map(|row| {
                let mut line = String::new();
                for col in 0..area.width {
                    let symbol = buf[(area.x + col, area.y + row)].symbol();
                    if symbol.is_empty() {
                        line.push(' ');
                    } else {
                        line.push_str(symbol);
                    }
                }
                line
            })
            .collect();
        lines.join("\n")
    }

    // 验证 `renders_title_setup_popup` 场景的触发条件与预期结果。
    #[test]
    fn renders_title_setup_popup() {
        let (tx_raw, _rx) = unbounded_channel::<AppEvent>();
        let tx = AppEventSender::new(tx_raw);
        let selected = [
            "project-name".to_string(),
            "activity".to_string(),
            "run-state".to_string(),
            "thread-title".to_string(),
        ];
        let view = TerminalTitleSetupView::new(
            Some(&selected),
            StatusSurfacePreviewData::default(),
            tx,
            crate::keymap::RuntimeKeymap::defaults().list,
        );
        assert_snapshot!(
            "terminal_title_setup_basic",
            render_lines(&view, /*width*/ 84)
        );
    }

    // 验证 `parse_terminal_title_items_preserves_order` 场景的触发条件与预期结果。
    #[test]
    fn parse_terminal_title_items_preserves_order() {
        let items = parse_terminal_title_items(
            ["project-name", "activity", "run-state", "thread-title"].into_iter(),
        );
        assert_eq!(
            items,
            Some(vec![
                TerminalTitleItem::Project,
                TerminalTitleItem::Spinner,
                TerminalTitleItem::Status,
                TerminalTitleItem::Thread,
            ])
        );
    }

    // 验证 `parse_terminal_title_items_rejects_invalid_ids` 场景的触发条件与预期结果。
    #[test]
    fn parse_terminal_title_items_rejects_invalid_ids() {
        let items = parse_terminal_title_items(["project", "not-a-title-item"].into_iter());
        assert_eq!(items, None);
    }

    // 验证 `activity_is_canonical_and_accepts_spinner_legacy_id` 场景的触发条件与预期结果。
    #[test]
    fn activity_is_canonical_and_accepts_spinner_legacy_id() {
        assert_eq!(TerminalTitleItem::Spinner.to_string(), "activity");
        assert_eq!(
            "activity".parse::<TerminalTitleItem>(),
            Ok(TerminalTitleItem::Spinner)
        );
        assert_eq!(
            "spinner".parse::<TerminalTitleItem>(),
            Ok(TerminalTitleItem::Spinner)
        );
    }

    // 验证 `project_name_is_canonical_and_accepts_project_legacy_id` 场景的触发条件与预期结果。
    #[test]
    fn project_name_is_canonical_and_accepts_project_legacy_id() {
        assert_eq!(TerminalTitleItem::Project.to_string(), "project-name");
        assert_eq!(
            "project-name".parse::<TerminalTitleItem>(),
            Ok(TerminalTitleItem::Project)
        );
        assert_eq!(
            "project".parse::<TerminalTitleItem>(),
            Ok(TerminalTitleItem::Project)
        );
    }

    // 验证 `thread_title_is_canonical_and_accepts_thread_legacy_id` 场景的触发条件与预期结果。
    #[test]
    fn thread_title_is_canonical_and_accepts_thread_legacy_id() {
        assert_eq!(TerminalTitleItem::Thread.to_string(), "thread-title");
        assert_eq!(
            "thread-title".parse::<TerminalTitleItem>(),
            Ok(TerminalTitleItem::Thread)
        );
        assert_eq!(
            "thread".parse::<TerminalTitleItem>(),
            Ok(TerminalTitleItem::Thread)
        );
    }

    // 验证 `model_is_canonical_and_accepts_model_name_legacy_id` 场景的触发条件与预期结果。
    #[test]
    fn model_is_canonical_and_accepts_model_name_legacy_id() {
        assert_eq!(TerminalTitleItem::Model.to_string(), "model");
        assert_eq!(
            "model".parse::<TerminalTitleItem>(),
            Ok(TerminalTitleItem::Model)
        );
        assert_eq!(
            "model-name".parse::<TerminalTitleItem>(),
            Ok(TerminalTitleItem::Model)
        );
    }

    // 验证 `run_state_is_canonical_and_accepts_status_legacy_id` 场景的触发条件与预期结果。
    #[test]
    fn run_state_is_canonical_and_accepts_status_legacy_id() {
        assert_eq!(TerminalTitleItem::Status.to_string(), "run-state");
        assert_eq!(
            "run-state".parse::<TerminalTitleItem>(),
            Ok(TerminalTitleItem::Status)
        );
        assert_eq!(
            "status".parse::<TerminalTitleItem>(),
            Ok(TerminalTitleItem::Status)
        );
    }

    // 验证 `model_with_reasoning_has_distinct_id` 场景的触发条件与预期结果。
    #[test]
    fn model_with_reasoning_has_distinct_id() {
        assert_eq!(
            TerminalTitleItem::ModelWithReasoning.to_string(),
            "model-with-reasoning"
        );
        assert_eq!(
            "model-with-reasoning".parse::<TerminalTitleItem>(),
            Ok(TerminalTitleItem::ModelWithReasoning)
        );
    }

    // 验证 `reasoning_is_selectable_id` 场景的触发条件与预期结果。
    #[test]
    fn reasoning_is_selectable_id() {
        assert_eq!(TerminalTitleItem::Reasoning.to_string(), "reasoning");
        assert_eq!(
            "reasoning".parse::<TerminalTitleItem>(),
            Ok(TerminalTitleItem::Reasoning)
        );
    }

    // 验证 `thread_usage_items_are_independently_selectable` 场景的触发条件与预期结果。
    #[test]
    fn thread_usage_items_are_independently_selectable() {
        assert_eq!(
            parse_terminal_title_items(["thread-credits", "estimated-thread-cost"].into_iter()),
            Some(vec![
                TerminalTitleItem::ThreadCredits,
                TerminalTitleItem::EstimatedThreadCost,
            ])
        );
    }

    // 验证 `parse_terminal_title_items_accepts_kebab_case_variants` 场景的触发条件与预期结果。
    #[test]
    fn parse_terminal_title_items_accepts_kebab_case_variants() {
        let items = parse_terminal_title_items(
            [
                "app-name",
                "context-remaining",
                "context-used",
                "five-hour-limit",
                "git-branch",
                "activity",
                "current-dir",
                "project-name",
                "model",
                "model-with-reasoning",
                "reasoning",
                "weekly-limit",
                "codex-version",
                "used-tokens",
                "total-input-tokens",
                "total-output-tokens",
                "thread-credits",
                "estimated-thread-cost",
                "session-id",
                "fast-mode",
            ]
            .into_iter(),
        );
        assert_eq!(
            items,
            Some(vec![
                TerminalTitleItem::AppName,
                TerminalTitleItem::ContextRemaining,
                TerminalTitleItem::ContextUsed,
                TerminalTitleItem::FiveHourLimit,
                TerminalTitleItem::GitBranch,
                TerminalTitleItem::Spinner,
                TerminalTitleItem::CurrentDir,
                TerminalTitleItem::Project,
                TerminalTitleItem::Model,
                TerminalTitleItem::ModelWithReasoning,
                TerminalTitleItem::Reasoning,
                TerminalTitleItem::WeeklyLimit,
                TerminalTitleItem::CodexVersion,
                TerminalTitleItem::UsedTokens,
                TerminalTitleItem::TotalInputTokens,
                TerminalTitleItem::TotalOutputTokens,
                TerminalTitleItem::ThreadCredits,
                TerminalTitleItem::EstimatedThreadCost,
                TerminalTitleItem::SessionId,
                TerminalTitleItem::FastMode,
            ])
        );
    }
}
