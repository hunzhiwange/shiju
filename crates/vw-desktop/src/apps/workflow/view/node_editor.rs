//! 工作流节点编辑器视图模块，负责组装节点编辑弹窗、标签页、说明区和高级 DSL 区域。

use super::*;
use iced::widget::{column, row};

/// 构建 node editor modal 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_node_editor_modal(state: &WorkflowState) -> Element<'_, Message> {
    let Some(editor) = state.node_editor.as_ref() else {
        return container(Space::new().width(1).height(1)).into();
    };

    let has_visual_tab = editor.visual_draft.is_some();
    let active_tab = match (has_visual_tab, editor.active_tab) {
        (true, tab) => tab,
        (false, WorkflowNodeEditorTab::Visual) => WorkflowNodeEditorTab::Description,
        (false, tab) => tab,
    };

    let mut content = column![].spacing(12);

    match active_tab {
        WorkflowNodeEditorTab::Visual => {
            if let Some(visual_section) = build_node_visual_section(state, editor) {
                content = content.push(visual_section);
            } else {
                content = content.push(build_node_visual_placeholder());
            }
            content = content.push(build_node_connection_hint());
        }
        WorkflowNodeEditorTab::Description => {
            content = content.push(build_node_description_section(editor));
        }
        WorkflowNodeEditorTab::Basic => {
            content = content.push(build_node_next_step_section(state, editor));
            content = content.push(build_node_connection_hint());
        }
        WorkflowNodeEditorTab::AdvancedDsl => {
            content = content.push(build_node_advanced_dsl_section(editor));
        }
    }

    let scroll_content = container(content).width(Length::Fill).padding(iced::Padding {
        top: 0.0,
        right: 4.0,
        bottom: 0.0,
        left: 0.0,
    });

    let validation_summary = editor
        .validation
        .has_errors()
        .then(|| build_node_validation_summary(&editor.validation))
        .unwrap_or_else(|| container(Space::new().height(0)).into());

    settings_modal_overlay(
        None,
        Message::None,
        container(
            column![
                build_node_editor_header(editor),
                build_node_editor_tabs(has_visual_tab, active_tab),
                validation_summary,
                scrollable(scroll_content)
                    .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
                    .height(Length::Fill),
            ]
            .spacing(14)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([18, 20])
        .style(modal_card_style),
    )
}

/// 构建 embedded text editor 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_embedded_text_editor<'a, F>(
    content: &'a text_editor::Content,
    placeholder: &'static str,
    map_message: F,
    height: f32,
) -> Element<'a, Message>
where
    F: Fn(text_editor::Action) -> WorkflowMessage + 'static + Copy,
{
    text_editor(content)
        .placeholder(placeholder)
        .on_action(move |action| Message::WorkflowTool(map_message(action)))
        .padding([10, 12])
        .height(Length::Fixed(height))
        .style(editor_style)
        .into()
}

/// StartBuiltinVariableItem 数据结构承载该模块对外传递的 StartBuiltinVariableItem 状态。
///
/// 字段保持可序列化或可渲染形态，便于调用方直接组合 UI 或持久化数据。
#[derive(Clone, Copy)]
pub(super) struct StartBuiltinVariableItem {
    pub(super) name: &'static str,
    pub(super) value_type: &'static str,
    pub(super) description: &'static str,
    pub(super) legacy: bool,
}

/// StartVariableTypeOption 数据结构承载该模块对外传递的 StartVariableTypeOption 状态。
///
/// 字段保持可序列化或可渲染形态，便于调用方直接组合 UI 或持久化数据。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct StartVariableTypeOption {
    pub(super) input_type: &'static str,
    pub(super) label: &'static str,
    pub(super) value_type: &'static str,
}

impl std::fmt::Display for StartVariableTypeOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} · {}", self.label, self.value_type)
    }
}

/// 构建 node editor header 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_node_editor_header(
    editor: &super::state::WorkflowNodeEditorDraft,
) -> Element<'_, Message> {
    let accent = workflow_node_accent_color(&editor.block_type);

    container(
        column![
            row![
                workflow_node_icon_badge(workflow_node_icon(&editor.block_type), accent, 16.0),
                text_input("输入节点名称", &editor.title)
                    .on_input(|value| {
                        Message::WorkflowTool(WorkflowMessage::NodeEditorTitleChanged(value))
                    })
                    .padding([8, 10])
                    .size(20)
                    .style(node_editor_title_input_style)
                    .width(Length::Fill),
                Space::new().width(Length::Fill),
                button(text("保存").size(12))
                    .style(primary_action_btn_style)
                    .padding([7, 11])
                    .on_press(Message::WorkflowTool(WorkflowMessage::SubmitNodeEditor)),
                settings_close_button(Message::WorkflowTool(WorkflowMessage::CloseNodeEditor)),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        ]
        .spacing(10),
    )
    .padding([2, 2])
    .into()
}

fn build_node_description_section(
    editor: &super::state::WorkflowNodeEditorDraft,
) -> Element<'_, Message> {
    text_editor(&editor.description_editor)
        .placeholder("补充这个节点在工作流中的作用与约束")
        .on_action(|action| {
            Message::WorkflowTool(WorkflowMessage::NodeEditorDescriptionAction(action))
        })
        .padding([10, 12])
        .height(Length::Fixed(78.0))
        .style(node_editor_description_style)
        .into()
}

fn build_node_connection_hint() -> Element<'static, Message> {
    container(
        text("连线提示：保存节点后，拖拽节点右侧或分支句柄到其他节点左侧句柄，即可添加连线。")
            .size(12)
            .style(settings_muted_text_style),
    )
    .padding([2, 0])
    .into()
}

/// 构建 node editor tabs 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_node_editor_tabs(
    has_visual_tab: bool,
    active_tab: WorkflowNodeEditorTab,
) -> Element<'static, Message> {
    let mut tabs = row![].spacing(8);

    if has_visual_tab {
        tabs = tabs.push(build_node_editor_tab_button(
            "配置",
            WorkflowNodeEditorTab::Visual,
            active_tab,
        ));
    }

    tabs = tabs
        .push(build_node_editor_tab_button("描述", WorkflowNodeEditorTab::Description, active_tab))
        .push(build_node_editor_tab_button("下一步", WorkflowNodeEditorTab::Basic, active_tab))
        .push(build_node_editor_tab_button(
            "高级 DSL",
            WorkflowNodeEditorTab::AdvancedDsl,
            active_tab,
        ));

    container(tabs).padding([0, 2]).into()
}

/// 构建 node editor tab button 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_node_editor_tab_button(
    label: &'static str,
    tab: WorkflowNodeEditorTab,
    active_tab: WorkflowNodeEditorTab,
) -> Element<'static, Message> {
    let active = tab == active_tab;

    button(text(label).size(12))
        .style(if active { primary_action_btn_style } else { rounded_action_btn_style })
        .padding([8, 12])
        .on_press(Message::WorkflowTool(WorkflowMessage::NodeEditorTabSelected(tab)))
        .into()
}

/// 构建 node advanced dsl section 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_node_advanced_dsl_section(
    editor: &super::state::WorkflowNodeEditorDraft,
) -> Element<'_, Message> {
    container(
        text_editor(&editor.raw_data_editor)
            .placeholder("输入节点 data 的 YAML，支持各类型特有字段")
            .on_action(|action| {
                Message::WorkflowTool(WorkflowMessage::NodeEditorDataAction(action))
            })
            .padding([10, 12])
            .height(Length::Fixed(360.0))
            .style(editor_style),
    )
    .padding(0)
    .style(value_card_style)
    .into()
}

/// 构建 node visual placeholder 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_node_visual_placeholder() -> Element<'static, Message> {
    text("当前节点暂未配置可视化表单，请切换到“高级 DSL”直接编辑节点 data YAML。")
        .size(12)
        .style(settings_muted_text_style)
        .into()
}

#[cfg(test)]
#[path = "node_editor_tests.rs"]
mod node_editor_tests;
