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

use iced::advanced::widget::operation::{focusable, scope};
use iced::advanced::widget::{Id, operate};
use iced::widget::{column, container, row, text};
use iced::{Alignment, Element, Length};

use crate::app::assets::Icon;
use crate::app::components::status_animation::spinner_frame;
use crate::app::components::system_settings_common::{
    icon_btn, settings_hover_help_label, settings_muted_text_style, settings_panel_style,
    settings_select, settings_text_editor, settings_text_input,
};
use crate::app::i18n::I18n;
use crate::app::state::ConventionalCommitType;
use crate::app::{App, Message, message};

use super::ui::{
    disabled_square_content_button_tiny, disabled_square_icon_button_tiny, square_icon_button_tiny,
};

const COMMIT_FOCUS_SCOPE: &str = "git-commit-form";

pub(crate) fn traverse_commit_focus(reverse: bool) -> iced::Task<Message> {
    let id = Id::from(COMMIT_FOCUS_SCOPE);
    if reverse {
        operate(scope(id, focusable::focus_previous::<()>())).discard()
    } else {
        operate(scope(id, focusable::focus_next::<()>())).discard()
    }
}

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

    let i18n = I18n::new(app.desktop_locale);
    let type_and_scope = app.show_git_diff_summary.then(|| {
        let commit_type_pick =
            settings_select(ConventionalCommitType::all(), app.git_commit_type, |commit_type| {
                Message::Git(message::GitMessage::CommitTypeSelected(commit_type))
            })
            .placeholder(i18n.__("类型（必选）"))
            .width(Length::Fixed(148.0));
        let scope = settings_text_input(i18n.__("scope（可选）"), &app.git_commit_scope)
            .on_input(|value| Message::Git(message::GitMessage::CommitScopeChanged(value)))
            .width(Length::Fill);
        row![commit_type_pick, scope].spacing(8).align_y(Alignment::Center)
    });

    let summary_input = settings_text_input(i18n.__("摘要（Summary）"), &app.git_commit_message)
        .on_input(|value| Message::Git(message::GitMessage::CommitMessageChanged(value)))
        .width(Length::Fill);
    let desc_input = settings_text_editor(&app.git_commit_description_editor)
        .placeholder(i18n.__("描述（可选）"))
        .on_action(|action| {
            Message::Git(message::GitMessage::CommitDescriptionEditorAction(action))
        })
        .height(Length::Fixed(88.0));

    let commit_state = commit_panel_state(app);

    // 根据条件构建提交按钮，动态显示不同的状态提示
    let commit_btn = match commit_state.button {
        CommitButtonState::InProgress => disabled_square_content_button_tiny(
            container(text(spinner_frame(app.file_manager_refresh_frame)))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center),
            crate::app::i18n::I18n::new(app.desktop_locale).__("正在提交…").to_owned(),
        ),
        CommitButtonState::NoSelection => disabled_square_icon_button_tiny(
            Icon::Save,
            crate::app::i18n::I18n::new(app.desktop_locale).__("选择变更后可提交").to_owned(),
        ),
        CommitButtonState::MissingSummary => disabled_square_icon_button_tiny(
            Icon::Save,
            crate::app::i18n::I18n::new(app.desktop_locale).__("填写摘要后可提交").to_owned(),
        ),
        CommitButtonState::MissingType => disabled_square_icon_button_tiny(
            Icon::Save,
            crate::app::i18n::I18n::new(app.desktop_locale).__("请选择类型").to_owned(),
        ),
        CommitButtonState::Ready => square_icon_button_tiny(
            Icon::Save,
            crate::app::i18n::I18n::new(app.desktop_locale).__("提交所选").to_owned(),
            Message::Git(message::GitMessage::CommitSelected),
        ),
    };
    let title = i18n.__("提交所选变更");
    let header_block = row![
        settings_hover_help_label(
            i18n,
            None,
            title,
            text(title).size(14),
            i18n.__(mode_description)
        ),
        icon_btn(
            Icon::FileText,
            i18n.__(COMMIT_HELP_TITLE),
            Some(Message::Git(message::GitMessage::CommitHelpOpen)),
        ),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let mut form_col = column![header_block].spacing(12);
    if let Some(type_and_scope) = type_and_scope {
        form_col = form_col.push(type_and_scope);
    }
    form_col = form_col.push(summary_input).push(
        column![
            text(crate::app::i18n::I18n::new(app.desktop_locale).__("详细描述"))
                .size(11)
                .style(settings_muted_text_style),
            desc_input,
        ]
        .spacing(6),
    );

    let col = column![
        form_col,
        row![
            container(
                text(crate::app::i18n::I18n::new(app.desktop_locale).__(commit_state.hint))
                    .size(11)
                    .style(settings_muted_text_style)
            )
            .width(Length::Fill)
            .center_y(Length::Shrink),
            container(commit_btn).center_x(Length::Shrink).center_y(Length::Shrink),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
    ]
    .spacing(12);

    // 包装为容器并应用最终样式，返回完整的面板元素
    container(col)
        .id(COMMIT_FOCUS_SCOPE)
        .width(Length::Fill)
        .padding([14, 16])
        .style(settings_panel_style)
        .into()
}
