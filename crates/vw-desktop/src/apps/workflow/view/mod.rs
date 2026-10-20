//! # Workflow 视图模块
//!
//! 该模块构建 workflow 整体界面，包含头部工具区、画布区、各类弹层编辑器与交互控件。

use super::canvas::WorkflowCanvas;
use super::message::WorkflowMessage;
use super::model::{
    WorkflowEdge, WorkflowNode, WorkflowNodeIconDescriptor, WorkflowNodeTypeDescriptor,
    pretty_block_type, supported_node_types, workflow_node_accent_color, workflow_node_icon,
    workflow_system_variables,
};
pub(super) use super::state;
use super::state::{
    WorkflowAppEditorMode, WorkflowCanvasContextMenuTarget, WorkflowCodeOutputDraft,
    WorkflowCodeVariableDraft, WorkflowDataNodeEdit, WorkflowHttpRequestEdit,
    WorkflowNodeEditorMode, WorkflowNodeEditorTab, WorkflowNodeRetryDraft, WorkflowNodeVisualDraft,
    WorkflowState, WorkflowVariablePanelKind,
};
use crate::app::Message;
use crate::app::assets::{self, Icon};
use crate::app::components::overlays::PointBelowOverlay;
use crate::app::components::system_settings_common::{
    danger_action_btn_style, primary_action_btn_style, round_icon_btn_style,
    rounded_action_btn_style, settings_close_button, settings_modal_backdrop_style,
    settings_modal_card_style, settings_modal_overlay, settings_muted_text_style,
    settings_panel_style, settings_pick_list_menu_style, settings_pick_list_style,
    settings_section_card, settings_text_editor_style, settings_text_input_style,
    settings_value_badge,
};
use iced::widget::{
    Space, button, checkbox, container, image, mouse_area, pick_list, scrollable,
    scrollable::{Direction, Scrollbar},
    slider, stack, svg, text, text_editor, text_input, toggler,
    tooltip::{Position as TooltipPosition, Tooltip},
};
use iced::{Alignment, Background, Border, Color, Element, Length, Point, Shadow, Theme, Vector};

mod app_editor;
mod app_list;
mod canvas_shell;
mod data_nodes;
mod header;
mod http_request;
mod if_else;
mod menu;
mod next_step;
mod node_editor;
#[cfg_attr(not(test), allow(dead_code))]
mod node_editor_fields;
mod node_visual;
mod node_visual_integrations;
mod run_preview;
mod start_inputs;
mod start_meta;
mod start_panel;
mod styles;
mod toolbar;
mod variables;

#[cfg(test)]
#[path = "node_editor_fields_tests.rs"]
mod node_editor_fields_tests;

use app_editor::*;
use app_list::*;
use canvas_shell::*;
use data_nodes::*;
use header::*;
use http_request::*;
use if_else::*;
use menu::*;
use next_step::*;
use node_editor::*;
use node_visual::*;
use node_visual_integrations::*;
use run_preview::*;
use start_inputs::*;
use start_meta::*;
use start_panel::*;
use styles::*;
use toolbar::*;
use variables::*;

const FLOATING_MARGIN: f32 = 14.0;
const ACTION_BAR_BUTTON_SIZE: f32 = 30.0;
const ACTION_BAR_PADDING: f32 = 6.0;
const ACTION_BAR_HEIGHT: f32 = ACTION_BAR_PADDING * 2.0 + ACTION_BAR_BUTTON_SIZE;
const ACTION_MENU_GAP: f32 = 8.0;
const TOOLBAR_WIDTH: f32 = 248.0;
const TOOLBAR_HEIGHT: f32 = 40.0;
const QUICK_INSERT_GAP: f32 = 8.0;
const ZOOM_CONTROL_WIDTH: f32 = 156.0;
const ZOOM_CONTROL_HEIGHT: f32 = 32.0;
const ZOOM_MENU_GAP: f32 = 8.0;

fn editor_style(theme: &Theme, status: text_editor::Status) -> text_editor::Style {
    settings_text_editor_style(theme, status)
}

fn is_dark_theme(theme: &Theme) -> bool {
    let background = theme.palette().background;
    background.r + background.g + background.b < 1.5
}

fn workflow_text_input<'a>(
    placeholder: &'a str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'static,
) -> Element<'a, Message> {
    text_input(placeholder, value)
        .on_input(on_input)
        .padding([10, 12])
        .size(13)
        .style(settings_text_input_style)
        .width(Length::Fill)
        .into()
}

fn node_editor_title_input_style(
    theme: &Theme,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    let palette = theme.palette();
    let extended = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    let is_focused = matches!(status, iced::widget::text_input::Status::Focused { .. });
    let is_hovered = matches!(status, iced::widget::text_input::Status::Hovered)
        || matches!(status, iced::widget::text_input::Status::Focused { is_hovered: true });

    iced::widget::text_input::Style {
        background: Background::Color(if is_focused {
            palette.primary.scale_alpha(if is_dark { 0.10 } else { 0.08 })
        } else if is_hovered {
            extended.background.weak.color.scale_alpha(if is_dark { 0.54 } else { 0.72 })
        } else {
            Color::TRANSPARENT
        }),
        border: Border {
            width: 1.0,
            color: if is_focused {
                palette.primary.scale_alpha(0.88)
            } else if is_hovered {
                extended.background.strong.color.scale_alpha(if is_dark { 0.72 } else { 0.28 })
            } else {
                Color::TRANSPARENT
            },
            radius: 12.0.into(),
        },
        icon: palette.text.scale_alpha(0.65),
        placeholder: palette.text.scale_alpha(0.42),
        value: palette.text,
        selection: palette.primary.scale_alpha(0.20),
    }
}

fn node_editor_description_style(theme: &Theme, status: text_editor::Status) -> text_editor::Style {
    let palette = theme.palette();
    let extended = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    let is_focused = matches!(status, text_editor::Status::Focused { .. });
    let is_hovered = matches!(status, text_editor::Status::Hovered)
        || matches!(status, text_editor::Status::Focused { is_hovered: true });

    text_editor::Style {
        background: Background::Color(if is_focused {
            palette.primary.scale_alpha(if is_dark { 0.08 } else { 0.06 })
        } else if is_hovered {
            extended.background.weak.color.scale_alpha(if is_dark { 0.50 } else { 0.68 })
        } else {
            Color::TRANSPARENT
        }),
        border: Border {
            width: 1.0,
            color: if is_focused {
                palette.primary.scale_alpha(0.84)
            } else if is_hovered {
                extended.background.strong.color.scale_alpha(if is_dark { 0.70 } else { 0.24 })
            } else {
                Color::TRANSPARENT
            },
            radius: 12.0.into(),
        },
        placeholder: palette.text.scale_alpha(0.42),
        value: palette.text,
        selection: palette.primary.scale_alpha(0.18),
    }
}

fn next_step_jump_card_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let extended = theme.extended_palette();
    let is_dark = is_dark_theme(theme);

    let background = match status {
        iced::widget::button::Status::Pressed => {
            Some(Background::Color(extended.background.strong.color.scale_alpha(0.92)))
        }
        iced::widget::button::Status::Hovered => {
            Some(Background::Color(extended.background.weak.color.scale_alpha(if is_dark {
                0.92
            } else {
                0.96
            })))
        }
        _ => Some(Background::Color(if is_dark {
            extended.background.base.color.scale_alpha(0.86)
        } else {
            Color::WHITE.scale_alpha(0.82)
        })),
    };

    iced::widget::button::Style {
        background,
        border: Border {
            width: 1.0,
            color: match status {
                iced::widget::button::Status::Pressed => theme.palette().primary.scale_alpha(0.72),
                iced::widget::button::Status::Hovered => theme.palette().primary.scale_alpha(0.34),
                _ => {
                    extended.background.strong.color.scale_alpha(if is_dark { 0.56 } else { 0.14 })
                }
            },
            radius: 14.0.into(),
        },
        text_color: theme.palette().text,
        ..Default::default()
    }
}

fn section_card<'a>(title: &'a str, description: &'a str) -> Element<'a, Message> {
    settings_section_card(title, description).into()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkflowAppSwitchOption {
    id: String,
    label: String,
    source: WorkflowAppSwitchSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum WorkflowAppSwitchSource {
    SavedUuid(String),
    OpenApp(String),
}

impl std::fmt::Display for WorkflowAppSwitchOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

fn workflow_app_display_name(name: &str) -> String {
    if name.trim().is_empty() { "未命名应用".to_string() } else { name.to_string() }
}

fn workflow_app_switch_options(state: &WorkflowState) -> Vec<WorkflowAppSwitchOption> {
    let mut options = state
        .saved_apps
        .iter()
        .map(|app| {
            let is_active = state.local_uuid.as_deref() == Some(app.uuid.as_str());
            let dirty = is_active && state.active_is_dirty;
            let label = if dirty {
                format!("🤖 {} *", workflow_app_display_name(&app.name))
            } else {
                format!("🤖 {}", workflow_app_display_name(&app.name))
            };

            WorkflowAppSwitchOption {
                id: app.uuid.clone(),
                label,
                source: WorkflowAppSwitchSource::SavedUuid(app.uuid.clone()),
            }
        })
        .collect::<Vec<_>>();

    for app in state.apps.iter().filter(|app| {
        app.local_uuid
            .as_deref()
            .is_none_or(|uuid| !state.saved_apps.iter().any(|saved| saved.uuid == uuid))
    }) {
        let is_active = state.active_app_id.as_deref() == Some(app.id.as_str());
        let name = if is_active {
            workflow_app_display_name(&state.source_name)
        } else {
            workflow_app_display_name(&app.meta.name)
        };
        let dirty = if is_active { state.active_is_dirty } else { app.is_dirty };
        let label = if dirty {
            format!("{} {} *", app.meta.icon, name)
        } else {
            format!("{} {}", app.meta.icon, name)
        };

        options.push(WorkflowAppSwitchOption {
            id: app.id.clone(),
            label,
            source: WorkflowAppSwitchSource::OpenApp(app.id.clone()),
        });
    }

    options
}

fn build_app_switcher(state: &WorkflowState) -> Element<'static, Message> {
    let options = workflow_app_switch_options(state);
    let selected = state
        .local_uuid
        .as_ref()
        .and_then(|uuid| {
            options.iter().find(|option| {
                matches!(&option.source, WorkflowAppSwitchSource::SavedUuid(saved_uuid) if saved_uuid == uuid)
            })
        })
        .or_else(|| {
            state.active_app_id.as_ref().and_then(|active_id| {
                options.iter().find(|option| {
                    matches!(&option.source, WorkflowAppSwitchSource::OpenApp(app_id) if app_id == active_id)
                })
            })
        })
        .cloned();

    if options.is_empty() {
        container(Space::new().width(Length::Shrink).height(Length::Fixed(0.0)))
            .width(Length::Fill)
            .into()
    } else {
        pick_list(options, selected, |option| match option.source {
            WorkflowAppSwitchSource::SavedUuid(uuid) => {
                Message::WorkflowTool(WorkflowMessage::OpenSavedApp(uuid))
            }
            WorkflowAppSwitchSource::OpenApp(id) => {
                Message::WorkflowTool(WorkflowMessage::SelectApp(id))
            }
        })
        .padding([10, 14])
        .text_size(13)
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style)
        .width(Length::Fill)
        .into()
    }
}

pub fn view(state: &WorkflowState) -> Element<'_, Message> {
    if state.active_app_id.is_none() {
        let mut layers: Vec<Element<'_, Message>> = vec![
            container(build_saved_apps_view(state)).width(Length::Fill).height(Length::Fill).into(),
        ];

        if let Some(error) = &state.error_message {
            layers.push(
                container(error_banner(error))
                    .padding(iced::Padding {
                        top: FLOATING_MARGIN,
                        right: FLOATING_MARGIN,
                        bottom: 0.0,
                        left: FLOATING_MARGIN,
                    })
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Top)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into(),
            );
        }

        if state.app_editor.is_some() {
            layers.push(build_app_editor_modal(state));
        }
        if let Some(confirm_dialog) = build_saved_app_delete_confirm_dialog(state) {
            layers.push(confirm_dialog);
        }
        if let Some(run_prompt) = build_saved_app_run_prompt_dialog(state) {
            layers.push(run_prompt);
        }
        if let Some(preview) = &state.saved_app_run_preview {
            layers.push(build_saved_app_run_preview_layer(preview, state.run_error_copied));
        }
        if let Some(detail) = &state.saved_app_run_detail {
            layers.push(build_saved_app_run_detail_layer(detail));
        }

        return stack(layers).width(Length::Fill).height(Length::Fill).into();
    }

    let canvas_base: Element<'_, Message> = if !state.has_apps() || state.document.nodes.is_empty()
    {
        container(Space::new().width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(canvas_panel_style)
            .into()
    } else {
        container(
            iced::widget::canvas(WorkflowCanvas {
                document: &state.document,
                pan: state.pan,
                zoom: state.zoom,
                selected_node_id: state.selected_node_id.as_deref(),
                selected_edge_id: state.selected_edge_id.as_deref(),
                connection_draft: state.connection_draft.as_ref(),
            })
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(canvas_panel_style)
        .into()
    };

    let canvas_area = build_canvas_area(state, canvas_base);

    let mut layers: Vec<Element<'_, Message>> = vec![
        container(canvas_area).width(Length::Fill).height(Length::Fill).style(root_style).into(),
    ];

    if let Some(error) = &state.error_message {
        layers.push(
            container(error_banner(error))
                .padding(iced::Padding {
                    top: FLOATING_MARGIN,
                    right: FLOATING_MARGIN,
                    bottom: 0.0,
                    left: FLOATING_MARGIN,
                })
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Top)
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        );
    }

    if state.app_editor.is_some() {
        layers.push(build_app_editor_modal(state));
    }
    if state.node_editor.is_some() {
        layers.push(build_node_editor_modal(state));
    }
    if state.variable_panel.is_some() {
        layers.push(build_variable_panel_modal(state));
    }
    if state.variable_editor.is_some() {
        layers.push(build_variable_editor_modal(state));
    }
    if state.node_editor.as_ref().and_then(|editor| editor.start_variable_editor.as_ref()).is_some()
    {
        layers.push(build_start_variable_editor_modal(state));
    }

    stack(layers).width(Length::Fill).height(Length::Fill).into()
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
