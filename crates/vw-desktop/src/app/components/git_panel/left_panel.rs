//! Git 面板左侧面板组件
//!
//! 本模块提供 Git 提交界面左侧面板的视图组件，用于构建用户友好的提交体验。
//! 支持约定式提交（Conventional Commits）规范，帮助团队保持一致的提交信息格式。
//!
//! # 主要功能
//!
//! - **提交类型选择**：支持下拉选择约定式提交类型（如 feat、fix、docs 等）
//! - **作用域输入**：可选的作用域字段，用于限定提交的影响范围
//! - **摘要编辑**：必填的简短提交摘要输入框
//! - **描述编辑器**：可选的详细提交描述，支持多行文本编辑
//! - **智能提交按钮**：根据选择状态和输入内容自动启用/禁用
//! - **帮助提示**：提供约定式提交规范的详细说明和示例
//!
//! # 约定式提交格式
//!
//! ```text
//! <类型>[可选作用域]: <摘要>
//!
//! [可选详细描述]
//! ```

use iced::widget::{button, column, container, pick_list, row, svg, text, text_editor, text_input};
use iced::{Alignment, Border, Color, Element, Length, Theme};

use crate::app::assets::Icon;
use crate::app::components::status_animation::spinner_frame;
use crate::app::components::system_settings_common::{
    icon_svg, settings_muted_text_style, settings_panel_style, settings_pick_list_menu_style,
    settings_pick_list_style, settings_text_editor_style, settings_text_input_style,
};
use crate::app::state::ConventionalCommitType;
use crate::app::{App, Message, message};

use super::ui::{
    disabled_square_content_button_tiny, disabled_square_icon_button_tiny, square_icon_button_tiny,
};

pub(super) const COMMIT_HELP_TITLE: &str = "约定式提交";

pub(super) const COMMIT_HELP_TEXT: &str = r#"约定式提交格式

<类型>[可选作用域]: <摘要>

[可选详细描述]

核心提交类型

feat: 新增功能（最小版本号增加）
例：feat(auth): 新增用户登录模块

fix: 修复 Bug（补丁版本号增加）
例：fix: 修复首页图片加载失败的 bug

docs: 仅文档更新
例：docs: 更新 API 接口文档

style: 代码风格调整（不影响逻辑）
例：style: 按 ESLint 规则格式化代码

refactor: 代码重构（非新增功能也非修复 bug）
例：refactor: 优化用户查询函数的结构

perf: 性能优化
例：perf: 使用缓存优化列表渲染速度

test: 增加或修改测试
例：test: 为登录模块添加单元测试

build: 构建系统或外部依赖变更
例：build: 升级 webpack 至 v5

ci: CI 配置或脚本变更
例：ci: 在 GitHub Actions 中增加 Node 版本矩阵

chore: 不修改源码或测试文件的杂项变更
例：chore: 更新 npm 依赖包版本

revert: 回退之前的提交
例：revert: 回滚提交 abc123

扩展与特殊类型

init: 项目初始化或脚手架
config: 配置文件修改
release: 发布版本
deploy: 部署相关
merge: 合并分支
wip: 进行中的工作（临时提交，慎用）
typo: 修复拼写错误
locale: 国际化/本地化相关"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CommitButtonState {
    InProgress,
    NoSelection,
    MissingSummary,
    MissingType,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CommitPanelState {
    pub(super) hint: &'static str,
    pub(super) button: CommitButtonState,
}

pub(super) fn has_selected_changes(app: &App) -> bool {
    !app.staged_files_selected.is_empty()
        || !app.staged_hunks_selected.is_empty()
        || !app.staged_lines_selected.is_empty()
        || !app.staged_old_lines_selected.is_empty()
}

pub(super) fn commit_panel_state(app: &App) -> CommitPanelState {
    let any_selected = has_selected_changes(app);
    let summary = app.git_commit_message.trim();
    let type_valid = !app.show_git_diff_summary || app.git_commit_type.is_some();

    if app.git_commit_in_progress {
        CommitPanelState { hint: "正在提交已选变更", button: CommitButtonState::InProgress }
    } else if !any_selected {
        CommitPanelState {
            hint: "请选择至少一项变更后再提交",
            button: CommitButtonState::NoSelection,
        }
    } else if summary.is_empty() {
        CommitPanelState {
            hint: "填写摘要后即可提交", button: CommitButtonState::MissingSummary
        }
    } else if !type_valid {
        CommitPanelState {
            hint: "选择提交类型后即可提交", button: CommitButtonState::MissingType
        }
    } else {
        CommitPanelState {
            hint: "已满足提交条件，可直接提交", button: CommitButtonState::Ready
        }
    }
}

pub(super) fn help_icon_style(theme: &Theme, _status: svg::Status) -> svg::Style {
    svg::Style { color: Some(theme.palette().text.scale_alpha(0.78)) }
}

pub(super) fn help_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let is_hovered = matches!(
        status,
        iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed
    );

    iced::widget::button::Style {
        background: is_hovered.then(|| palette.background.weak.color.scale_alpha(0.72).into()),
        text_color: theme.palette().text,
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 999.0.into() },
        ..Default::default()
    }
}

/// 构建 Git 面板左侧面板视图
///
/// 该函数创建包含提交表单的完整左侧面板，根据应用状态动态调整界面布局：
/// - 当启用差异摘要模式时，显示完整的约定式提交表单（类型 + 作用域 + 摘要）
/// - 否则仅显示简化的摘要输入框
///
/// # 参数
///
/// * `app` - 应用状态引用，包含：
///   - `show_git_diff_summary`：是否显示完整的约定式提交表单
///   - `git_commit_type`：当前选中的提交类型
///   - `git_commit_scope`：当前的作用域文本
///   - `git_commit_message`：当前的提交摘要文本
///   - `git_commit_description_editor`：描述编辑器的内容
///   - `staged_files_selected` 等集合：已暂存的变更选择状态
///
/// # 返回值
///
/// 返回包含完整左侧面板的 `Element`，可嵌入到父容器中
///
/// # 界面行为
///
/// - **提交按钮状态**：
///   - 未选择任何变更时禁用，提示"选择变更后可提交"
///   - 摘要字段为空时禁用，提示"填写摘要后可提交"
///   - 未选择类型时禁用（仅在约定式提交模式下），提示"请选择类型"
///   - 所有条件满足时启用，显示"提交所选"
///
/// # 示例
///
/// ```ignore
/// let left_panel = view(&app);
/// // 将 left_panel 嵌入到 Git 面板的主布局中
/// ```
pub fn view(app: &App) -> Element<'_, Message> {
    let mode_description = if app.show_git_diff_summary {
        "按约定式提交规范组织标题、作用域与描述。"
    } else {
        "快速填写摘要并提交当前已选变更。"
    };

    // 构建摘要输入区域，根据模式决定显示完整表单还是简化输入框
    let summary_input: Option<Element<'_, Message>> = if app.show_git_diff_summary {
        // 约定式提交模式：显示类型选择器、作用域输入和摘要输入
        let commit_type_pick = pick_list(ConventionalCommitType::all(), app.git_commit_type, |t| {
            Message::Git(message::GitMessage::CommitTypeSelected(t))
        })
        .placeholder("类型（必选）")
        .text_size(13)
        .padding([10, 12])
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style)
        .width(Length::Fixed(110.0));
        Some(
            row![
                commit_type_pick,
                text_input("scope（可选）", &app.git_commit_scope)
                    .on_input(|v| Message::Git(message::GitMessage::CommitScopeChanged(v)))
                    .padding([10, 12])
                    .size(13)
                    .style(settings_text_input_style)
                    .width(Length::Fixed(120.0)),
                text_input("摘要（Summary）", &app.git_commit_message)
                    .on_input(|v| Message::Git(message::GitMessage::CommitMessageChanged(v)))
                    .padding([10, 12])
                    .size(13)
                    .style(settings_text_input_style)
                    .width(Length::Fill),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into(),
        )
    } else {
        // 简化模式：仅显示摘要输入框
        Some(
            text_input("摘要（Summary）", &app.git_commit_message)
                .on_input(|v| Message::Git(message::GitMessage::CommitMessageChanged(v)))
                .padding([10, 12])
                .size(13)
                .style(settings_text_input_style)
                .width(Length::Fill)
                .into(),
        )
    };
    // 构建描述编辑器，支持多行文本输入
    // 固定高度为 2 行（每行约 22 像素），提供简洁的描述输入体验
    let desc_input = text_editor(&app.git_commit_description_editor)
        .placeholder("描述（可选）")
        .on_action(|a| Message::Git(message::GitMessage::CommitDescriptionEditorAction(a)))
        .padding([10, 12])
        .size(13.0)
        .height(Length::Fixed(88.0))
        .style(settings_text_editor_style);

    let commit_state = commit_panel_state(app);

    // 根据条件构建提交按钮，动态显示不同的状态提示
    let commit_btn = match commit_state.button {
        CommitButtonState::InProgress => disabled_square_content_button_tiny(
            container(text(spinner_frame(app.file_manager_refresh_frame)))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center),
            "正在提交…".to_string(),
        ),
        CommitButtonState::NoSelection => {
            disabled_square_icon_button_tiny(Icon::Save, "选择变更后可提交".to_string())
        }
        CommitButtonState::MissingSummary => {
            disabled_square_icon_button_tiny(Icon::Save, "填写摘要后可提交".to_string())
        }
        CommitButtonState::MissingType => {
            disabled_square_icon_button_tiny(Icon::Save, "请选择类型".to_string())
        }
        CommitButtonState::Ready => square_icon_button_tiny(
            Icon::Save,
            "提交所选".to_string(),
            Message::Git(message::GitMessage::CommitSelected),
        ),
    };
    // 构建帮助按钮，点击后显示约定式提交规范弹窗。
    let help_btn = button(
        container(icon_svg(Icon::QuestionCircle, 14.0).style(help_icon_style))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .padding(0)
    .width(Length::Fixed(24.0))
    .height(Length::Fixed(24.0))
    .style(help_button_style)
    .on_press(Message::Git(message::GitMessage::CommitHelpOpen));

    // 构建主内容列，按顺序添加摘要输入、描述编辑器和底部操作区
    let header_block = container(
        row![
            container(
                column![
                    row![
                        text("提交所选变更").size(14),
                        container(help_btn).center_x(Length::Shrink).center_y(Length::Shrink),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                    text(mode_description).size(11).style(settings_muted_text_style),
                ]
                .spacing(4),
            )
            .width(Length::Fill)
            .center_y(Length::Shrink),
        ]
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .center_y(Length::Shrink);

    let mut form_col = column![header_block].spacing(12);
    if let Some(summary_input) = summary_input {
        form_col = form_col.push(summary_input);
    }
    form_col = form_col.push(
        column![text("详细描述").size(11).style(settings_muted_text_style), desc_input,].spacing(6),
    );

    let col = column![
        form_col,
        row![
            container(text(commit_state.hint).size(11).style(settings_muted_text_style))
                .width(Length::Fill)
                .center_y(Length::Shrink),
            container(commit_btn).center_x(Length::Shrink).center_y(Length::Shrink),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
    ]
    .spacing(12);

    // 包装为容器并应用最终样式，返回完整的面板元素
    container(col).width(Length::Fill).padding([14, 16]).style(settings_panel_style).into()
}
