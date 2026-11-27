use crate::app::components::system_settings_common::{
    icon_svg, primary_action_btn_style, rounded_action_btn_style, settings_divider,
    settings_muted_text_style, settings_panel_style, settings_select,
};
use crate::app::i18n::I18n;
use crate::app::message::TaskBoardMessage;
use crate::app::{App, Message};
use desktop_assets::assets::Icon;
use desktop_task_board::message::AdvancedWorkFilterAction as Action;
use desktop_task_board::work_views::{
    WorkAdvancedFilterDraft, WorkFilterCondition, WorkFilterField, WorkFilterGroup, WorkFilterJoin,
    WorkFilterNode,
};
use desktop_ui_controls::{
    overlays::BelowOverlay, single_line_label::SingleLineLabel,
    system_settings_common::icons::icon_btn_svg_style,
};
use iced::widget::{Space, button, column, container, row, text, tooltip};
use iced::{Alignment, Element, Font, Length};

use super::work_filter_picker::{advanced_values, field_label, fields, scroll_options};

#[derive(Clone, PartialEq)]
struct Choice<T> {
    value: T,
    label: String,
}

impl<T> std::fmt::Display for Choice<T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.label)
    }
}

fn message(action: Action) -> Message {
    TaskBoardMessage::AdvancedWorkFilter(action).into()
}

pub(super) fn view(app: &App, i18n: I18n) -> Element<'_, Message> {
    let Some(draft) = app.task_board.work_advanced_filter.as_ref() else {
        return Space::new().into();
    };
    let field_choices = fields(app)
        .into_iter()
        .map(|(field, _)| Choice { label: field_label(app, i18n, &field), value: field })
        .collect::<Vec<_>>();
    let content = group(app, i18n, draft, &draft.group, &[], &field_choices);
    let footer = row![
        secondary(i18n.__("切回简单筛选"), Action::SwitchToSimple),
        Space::new().width(Length::Fill),
        secondary(i18n.__("清除全部"), Action::Clear),
        primary(i18n.__("应用筛选"), Action::Apply),
    ]
    .spacing(7)
    .align_y(Alignment::Center)
    .wrap()
    .vertical_spacing(5);
    container(column![scroll_options(content, 480), settings_divider(), footer].spacing(10))
        .padding(12)
        .width(690)
        .style(settings_panel_style)
        .into()
}

fn group<'a>(
    app: &'a App,
    i18n: I18n,
    draft: &'a WorkAdvancedFilterDraft,
    group: &'a WorkFilterGroup,
    path: &[usize],
    fields: &[Choice<WorkFilterField>],
) -> Element<'a, Message> {
    let joins = [
        Choice { value: WorkFilterJoin::All, label: i18n.__("全部").to_owned() },
        Choice { value: WorkFilterJoin::Any, label: i18n.__("任一").to_owned() },
    ];
    let selected = joins.iter().find(|choice| choice.value == group.join).cloned();
    let group_path = path.to_vec();
    let join = settings_select(joins, selected, move |choice| {
        message(Action::SetJoin(group_path.clone(), choice.value))
    })
    .width(92);
    let is_root = path.is_empty();
    let mut header = row![].spacing(8).align_y(Alignment::Center);
    if is_root {
        header = header.push(icon_svg(Icon::Sliders, 15.0).style(icon_btn_svg_style)).push(
            text(i18n.__("高级筛选"))
                .size(14)
                .font(Font { weight: iced::font::Weight::Semibold, ..Font::DEFAULT }),
        );
    }
    header = header
        .push(
            text(i18n.__(if is_root { "满足" } else { "满足组内" }))
                .size(12)
                .style(settings_muted_text_style),
        )
        .push(join)
        .push(text(i18n.__("条件")).size(12).style(settings_muted_text_style));
    if !is_root {
        header = header.push(Space::new().width(Length::Fill)).push(node_menu(
            i18n,
            path,
            true,
            draft.menu.as_deref(),
        ));
    }
    let mut content = column![header].spacing(8).width(Length::Fill);
    for (index, child) in group.children.iter().enumerate() {
        let mut child_path = path.to_vec();
        child_path.push(index);
        let body: Element<'_, Message> = match child {
            WorkFilterNode::Condition(condition) => {
                condition_row(app, i18n, draft, condition, &child_path, fields)
            }
            WorkFilterNode::Group(nested) => {
                container(self::group(app, i18n, draft, nested, &child_path, fields))
                    .padding(10)
                    .width(Length::Fill)
                    .style(super::style::filter_group_surface)
                    .into()
            }
        };
        let connector = if index == 0 {
            "当"
        } else if group.join == WorkFilterJoin::All {
            "且"
        } else {
            "或"
        };
        content = content.push(
            row![
                container(text(i18n.__(connector)).size(12).style(settings_muted_text_style))
                    .width(39)
                    .height(34)
                    .align_y(Alignment::Center),
                body,
            ]
            .spacing(6)
            .align_y(Alignment::Start),
        );
    }
    let mut actions =
        row![add_button(i18n.__("添加条件"), Action::AddCondition(path.to_vec()))].spacing(7);
    if is_root {
        actions = actions.push(add_button(i18n.__("添加条件组"), Action::AddGroup(Vec::new())));
    }
    content.push(actions.wrap()).into()
}

fn condition_row<'a>(
    app: &'a App,
    i18n: I18n,
    draft: &WorkAdvancedFilterDraft,
    condition: &'a WorkFilterCondition,
    path: &[usize],
    fields: &[Choice<WorkFilterField>],
) -> Element<'a, Message> {
    let editing = draft.editing.as_deref() == Some(path);
    let selected_field =
        Choice { label: field_label(app, i18n, &condition.field), value: condition.field.clone() };
    let field_path = path.to_vec();
    let field = settings_select(fields.to_vec(), Some(selected_field), move |choice| {
        message(Action::SetField(field_path.clone(), choice.value))
    })
    .width(Length::FillPortion(4));
    let labels = if condition.field == WorkFilterField::Label {
        ["包含任一", "不包含任一"]
    } else {
        ["是", "不是"]
    };
    let operators = [
        Choice { value: false, label: i18n.__(labels[0]).to_owned() },
        Choice { value: true, label: i18n.__(labels[1]).to_owned() },
    ];
    let selected_operator =
        operators.iter().find(|choice| choice.value == condition.excluded).cloned();
    let operator_path = path.to_vec();
    let operator = settings_select(operators, selected_operator, move |choice| {
        message(Action::SetExcluded(operator_path.clone(), choice.value))
    })
    .width(Length::FillPortion(3));
    let summary =
        super::work_filter_bar::value_summary(app, i18n, &condition.field, &condition.rule)
            .unwrap_or_else(|| i18n.__("选择筛选值").to_owned());
    let value = button(
        row![
            SingleLineLabel::new(summary, 13.0),
            icon_svg(Icon::ChevronDown, 12.0).style(icon_btn_svg_style),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([7, 9])
    .width(Length::FillPortion(5))
    .style(move |theme, status| {
        if editing {
            super::style::navigation_button(theme, status, true)
        } else {
            rounded_action_btn_style(theme, status)
        }
    })
    .on_press(message(Action::EditValue(path.to_vec())));
    let row = row![field, operator, value, node_menu(i18n, path, false, draft.menu.as_deref())]
        .spacing(7)
        .align_y(Alignment::Center);
    if !editing {
        return row.into();
    }
    let pick = container(
        column![
            advanced_values(app, i18n, &condition.field),
            row![
                secondary(i18n.__("取消"), Action::CancelValue),
                primary(i18n.__("确定"), Action::ConfirmValue),
            ]
            .spacing(6),
        ]
        .spacing(8),
    )
    .width(Length::Fill)
    .max_width(320)
    .padding(8)
    .style(super::style::filter_chip_surface);
    column![row, pick].spacing(6).into()
}

fn add_button(label: &str, action: Action) -> Element<'static, Message> {
    button(
        row![icon_svg(Icon::Plus, 13.0).style(icon_btn_svg_style), text(label.to_owned()).size(12)]
            .spacing(6)
            .align_y(Alignment::Center),
    )
    .padding([6, 8])
    .style(rounded_action_btn_style)
    .on_press(message(action))
    .into()
}

fn node_menu(
    i18n: I18n,
    path: &[usize],
    group: bool,
    active: Option<&[usize]>,
) -> Element<'static, Message> {
    let open = active == Some(path);
    let trigger = button(icon_svg(Icon::DotsThreeHorizontal, 15.0).style(icon_btn_svg_style))
        .padding(6)
        .style(move |theme, status| super::style::navigation_button(theme, status, open))
        .on_press(message(Action::ShowMenu((!open).then(|| path.to_vec()))));
    if !open {
        return tooltip(
            trigger,
            container(text(i18n.__("操作")).size(12))
                .padding([4, 7])
                .style(desktop_ui_controls::tooltip::tooltip_dark_style),
            tooltip::Position::Top,
        )
        .into();
    }
    let mut items = column![].spacing(2);
    if !group {
        if path.len() == 1 {
            items = items.push(menu_action(
                i18n.__("转为条件组"),
                Action::ConvertToGroup(path.to_vec()),
                false,
            ));
        }
        items = items.push(menu_action(
            i18n.__("复制"),
            Action::DuplicateCondition(path.to_vec()),
            false,
        ));
    }
    items = items.push(menu_action(
        i18n.__(if group { "删除条件组" } else { "删除" }),
        Action::Remove(path.to_vec()),
        true,
    ));
    BelowOverlay::new(trigger, container(items).width(152).padding(5).style(settings_panel_style))
        .show(true)
        .within_window()
        .align_right(true)
        .gap(4.0)
        .dismiss_on_escape()
        .on_close(message(Action::ShowMenu(None)))
        .into()
}

fn menu_action(label: &str, action: Action, danger: bool) -> Element<'static, Message> {
    button(text(label.to_owned()).size(13))
        .padding([7, 9])
        .width(Length::Fill)
        .style(move |theme, status| {
            let mut style = super::style::navigation_button(theme, status, false);
            if danger {
                style.text_color = theme.extended_palette().danger.base.color;
            }
            style
        })
        .on_press(message(action))
        .into()
}

fn secondary(label: &str, action: Action) -> Element<'static, Message> {
    button(text(label.to_owned()).size(12))
        .padding([6, 8])
        .style(rounded_action_btn_style)
        .on_press(message(action))
        .into()
}

fn primary(label: &str, action: Action) -> Element<'static, Message> {
    button(text(label.to_owned()).size(12))
        .padding([6, 10])
        .style(primary_action_btn_style)
        .on_press(message(action))
        .into()
}
