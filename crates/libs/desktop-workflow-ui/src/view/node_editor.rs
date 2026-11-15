//! 工作流节点编辑器视图模块，负责组装节点编辑弹窗、标签页、说明区和高级 DSL 区域。

use super::*;
use desktop_ui_controls::text_editor_context_menu::{
    TextEditorContextMenuMessages, TextEditorContextMenuState, wrap_with_context_menu,
};
use iced::widget::{column, mouse_area, opaque, row};

const NODE_EDITOR_CANVAS_PORTION: u16 = 3;
const NODE_EDITOR_DRAWER_PORTION: u16 = 2;

/// 构建 node editor modal 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub fn build_node_editor_modal<'a, M: Clone + 'static>(
    i18n: desktop_i18n::I18n,
    state: &'a WorkflowState,
    visual_section: Option<Element<'a, M>>,
    map_message: fn(Message) -> M,
) -> Element<'a, M> {
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
            if let Some(visual_section) = visual_section {
                content = content.push(visual_section);
            } else {
                content = content.push(build_node_visual_placeholder(i18n).map(map_message));
            }
            content = content.push(build_node_connection_hint(i18n).map(map_message));
        }
        WorkflowNodeEditorTab::Description => {
            content = content.push(build_node_description_section(i18n, editor).map(map_message));
        }
        WorkflowNodeEditorTab::Basic => {
            content = content.push(build_node_next_step_section(i18n, state, editor).map(map_message));
            content = content.push(build_node_connection_hint(i18n).map(map_message));
        }
        WorkflowNodeEditorTab::AdvancedDsl => {
            content = content.push(build_node_advanced_dsl_section(i18n, state, editor).map(map_message));
        }
    }

    let mut header = column![
        build_node_editor_header(i18n, editor).map(map_message),
        build_node_editor_tabs(i18n, has_visual_tab, active_tab).map(map_message),
    ]
    .spacing(12);
    if editor.validation.has_errors() {
        header = header.push(build_node_validation_summary(i18n, &editor.validation).map(map_message));
    }
    let drawer = opaque(
        mouse_area(
            settings_modal_layout(header, content, None)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding([18, 20])
                .style(node_editor_drawer_style),
        )
        .on_scroll(move |_| map_message(WorkflowMessage::NodeEditorScrollCaptured)),
    );

    row![
        Space::new().width(Length::FillPortion(NODE_EDITOR_CANVAS_PORTION)),
        container(drawer).width(Length::FillPortion(NODE_EDITOR_DRAWER_PORTION)),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// 构建 embedded text editor 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub fn build_embedded_text_editor<'a, F>(
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
        .on_action(move |action| map_message(action))
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .height(Length::Fixed(height))
        .style(editor_style)
        .into()
}

/// StartBuiltinVariableItem 数据结构承载该模块对外传递的 StartBuiltinVariableItem 状态。
///
/// 字段保持可序列化或可渲染形态，便于调用方直接组合 UI 或持久化数据。
#[derive(Clone, Copy)]
pub struct StartBuiltinVariableItem {
    pub name: &'static str,
    pub value_type: &'static str,
    pub description: &'static str,
    pub legacy: bool,
}

/// StartVariableTypeOption 数据结构承载该模块对外传递的 StartVariableTypeOption 状态。
///
/// 字段保持可序列化或可渲染形态，便于调用方直接组合 UI 或持久化数据。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartVariableTypeOption {
    pub input_type: &'static str,
    pub label: &'static str,
    pub value_type: &'static str,
}

impl std::fmt::Display for StartVariableTypeOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} · {}", self.label, self.value_type)
    }
}

/// 构建 node editor header 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub fn build_node_editor_header(
    i18n: desktop_i18n::I18n,
    editor: &super::state::WorkflowNodeEditorDraft,
) -> Element<'_, Message> {
    let accent = workflow_node_accent_color(&editor.block_type);

    settings_modal_header(
        row![
            workflow_node_icon_badge(workflow_node_icon(&editor.block_type), accent, 16.0),
            workflow_text_input(i18n.__("输入节点名称"), &editor.title, |value| {
                WorkflowMessage::NodeEditorTitleChanged(value)
            }),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        row![
            button(text(i18n.__("保存")).size(SETTINGS_CONTROL_TEXT_SIZE))
                .style(primary_action_btn_style)
                .padding([10, 12])
                .on_press(WorkflowMessage::SubmitNodeEditor),
            settings_close_button(WorkflowMessage::CloseNodeEditor),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
}

fn build_node_description_section(
    i18n: desktop_i18n::I18n,
    editor: &super::state::WorkflowNodeEditorDraft,
) -> Element<'_, Message> {
    text_editor(&editor.description_editor)
        .placeholder(i18n.__("补充这个节点在工作流中的作用与约束"))
        .on_action(|action| {
            WorkflowMessage::NodeEditorDescriptionAction(action)
        })
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .height(Length::Fixed(78.0))
        .style(settings_text_editor_style)
        .into()
}

fn build_node_connection_hint(i18n: desktop_i18n::I18n) -> Element<'static, Message> {
    container(
        text(
            i18n.__(
                "连线提示：保存节点后，拖拽节点右侧或分支句柄到其他节点左侧句柄，即可添加连线。",
            ),
        )
        .size(12)
        .style(settings_muted_text_style),
    )
    .padding([2, 0])
    .into()
}

/// 构建 node editor tabs 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub fn build_node_editor_tabs(
    i18n: desktop_i18n::I18n,
    has_visual_tab: bool,
    active_tab: WorkflowNodeEditorTab,
) -> Element<'static, Message> {
    let mut tabs = row![].spacing(8);

    if has_visual_tab {
        tabs = tabs.push(build_node_editor_tab_button(
            i18n.__("配置"),
            WorkflowNodeEditorTab::Visual,
            active_tab,
        ));
    }

    tabs = tabs
        .push(build_node_editor_tab_button(
            i18n.__("描述"),
            WorkflowNodeEditorTab::Description,
            active_tab,
        ))
        .push(build_node_editor_tab_button(
            i18n.__("下一步"),
            WorkflowNodeEditorTab::Basic,
            active_tab,
        ))
        .push(build_node_editor_tab_button(
            i18n.__("高级 DSL"),
            WorkflowNodeEditorTab::AdvancedDsl,
            active_tab,
        ));

    tabs.wrap().into()
}

/// 构建 node editor tab button 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub fn build_node_editor_tab_button(
    label: &'static str,
    tab: WorkflowNodeEditorTab,
    active_tab: WorkflowNodeEditorTab,
) -> Element<'static, Message> {
    let active = tab == active_tab;

    button(text(label).size(12))
        .style(move |theme, status| {
            let mut style = settings_segment_button_style(theme, status, active);
            if active {
                style.text_color = theme.palette().text;
            }
            style
        })
        .padding([8, 8])
        .on_press(WorkflowMessage::NodeEditorTabSelected(tab))
        .into()
}

/// 构建 node advanced dsl section 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub fn build_node_advanced_dsl_section<'a>(
    i18n: desktop_i18n::I18n,
    state: &'a WorkflowState,
    editor: &'a super::state::WorkflowNodeEditorDraft,
) -> Element<'a, Message> {
    let editor = text_editor(&editor.raw_data_editor)
        .id(iced::widget::Id::new(NODE_EDITOR_DSL_EDITOR_ID))
        .placeholder(i18n.__("输入节点 data 的 YAML，支持各类型特有字段"))
        .on_action(|action| WorkflowMessage::NodeEditorDataAction(action))
        .key_binding(node_editor_dsl_key_binding)
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .height(Length::Fixed(360.0))
        .style(editor_style);
    wrap_with_context_menu(
        i18n,
        editor,
        TextEditorContextMenuState {
            open: state.node_editor_dsl_context_menu_open,
            position: state.node_editor_dsl_context_menu_pos,
        },
        |point| {
            WorkflowMessage::NodeEditorDslOpenContextMenu {
                x: point.x,
                y: point.y,
            }
        },
        TextEditorContextMenuMessages {
            close: WorkflowMessage::NodeEditorDslCloseContextMenu,
            undo: Some(WorkflowMessage::NodeEditorDslUndo),
            redo: Some(WorkflowMessage::NodeEditorDslRedo),
            copy: WorkflowMessage::NodeEditorDslContextMenuCopy,
            cut: WorkflowMessage::NodeEditorDslContextMenuCut,
            paste: WorkflowMessage::NodeEditorDslContextMenuPaste,
            delete: WorkflowMessage::NodeEditorDslContextMenuDelete,
        },
    )
}

fn node_editor_dsl_key_binding(
    key_press: text_editor::KeyPress,
) -> Option<text_editor::Binding<Message>> {
    let command = key_press.modifiers.command();
    let shift = key_press.modifiers.shift();
    let custom = match key_press.key.as_ref() {
        iced::keyboard::Key::Character(value)
            if command && value.eq_ignore_ascii_case("z") && shift =>
        {
            Some(WorkflowMessage::NodeEditorDslRedo)
        }
        iced::keyboard::Key::Character(value) if command && value.eq_ignore_ascii_case("z") => {
            Some(WorkflowMessage::NodeEditorDslUndo)
        }
        iced::keyboard::Key::Character(value) if command && value.eq_ignore_ascii_case("y") => {
            Some(WorkflowMessage::NodeEditorDslRedo)
        }
        _ => None,
    };

    custom
        .map(|message| text_editor::Binding::Custom(message))
        .or_else(|| text_editor::Binding::from_key_press(key_press))
}

/// 构建 node visual placeholder 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub fn build_node_visual_placeholder(
    i18n: desktop_i18n::I18n,
) -> Element<'static, Message> {
    text(i18n.__("当前节点暂未配置可视化表单，请切换到“高级 DSL”直接编辑节点 data YAML。"))
        .size(12)
        .style(settings_muted_text_style)
        .into()
}

#[cfg(test)]
#[path = "node_editor_tests.rs"]
mod node_editor_tests;

pub const NODE_EDITOR_DSL_EDITOR_ID: &str = "workflow-node-advanced-dsl-editor";

pub fn node_editor_drawer_style(theme: &Theme) -> iced::widget::container::Style {
    let mut style = settings_modal_card_style(theme);
    if let Some(Background::Color(background)) = &mut style.background {
        background.a = 1.0;
    }
    style.border.radius = 0.0.into();
    style.shadow = Shadow {
        color: Color::BLACK.scale_alpha(if is_dark_theme(theme) { 0.28 } else { 0.16 }),
        offset: Vector::new(-8.0, 0.0),
        blur_radius: 24.0,
    };
    style
}

pub fn validation_summary_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();

    iced::widget::container::Style {
        background: Some(Background::Color(palette.danger.weak.color.scale_alpha(0.20))),
        text_color: Some(if is_dark_theme(theme) {
            palette.background.base.text
        } else {
            palette.danger.base.color
        }),
        border: Border {
            color: palette.danger.base.color.scale_alpha(0.45),
            width: 1.0,
            radius: 16.0.into(),
        },
        ..Default::default()
    }
}

pub fn build_node_validation_summary(
    i18n: desktop_i18n::I18n,
    validation: &super::state::WorkflowNodeEditorValidation,
) -> Element<'_, Message> {
    let mut items = column![
        text(i18n.__("当前节点还有未修正的字段")).size(14),
        text(i18n.__("请先处理下面这些错误，再保存节点。")).size(12),
    ]
    .spacing(6);

    for error in validation.field_errors.iter().take(8) {
        items = items.push(text(format!("- {}", error.message.render(i18n))).size(12));
    }

    if validation.field_errors.len() > 8 {
        items = items.push(
            text(
                i18n.format(
                    "还有 %s 项未展示",
                    &[(validation.field_errors.len() - 8).to_string().into()],
                )
                .expect("valid workflow display template"),
            )
            .size(12),
        );
    }

    container(items).padding([12, 14]).style(validation_summary_style).into()
}
