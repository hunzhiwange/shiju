//! 节点编辑器共享字段，集中提供变量、列表、模型与重试配置的稳定编辑事件。

use super::state::{
    WorkflowKeyValueDraft, WorkflowModelDraft, WorkflowNamedValueDraft, WorkflowNodeFieldEdit,
    WorkflowVariableSelectorDraft,
};
use super::*;
use iced::widget::{column, row};

pub(super) fn build_variable_selector_field<'a, F>(
    label: &'a str,
    selector: &'a WorkflowVariableSelectorDraft,
    map_edit: F,
) -> Element<'a, Message>
where
    F: Fn(WorkflowNodeFieldEdit) -> Message + 'static,
{
    let input = selector.to_input();
    container(
        column![
            text(label).size(12),
            text_input("节点.变量", &input)
                .on_input(move |value| {
                    map_edit(WorkflowNodeFieldEdit::VariableSelectorChanged(value))
                })
                .padding([10, 12])
                .size(13)
                .style(settings_text_input_style)
                .width(Length::Fill),
        ]
        .spacing(6),
    )
    .padding(10)
    .style(node_editor_field_card_style)
    .into()
}

pub(super) fn build_named_value_list<'a, F>(
    values: &'a [WorkflowNamedValueDraft],
    map_edit: F,
) -> Element<'a, Message>
where
    F: Fn(WorkflowNodeFieldEdit) -> Message + Clone + 'static,
{
    let mut list = column![].spacing(8);
    for (index, value) in values.iter().enumerate() {
        let name_mapper = map_edit.clone();
        let type_mapper = map_edit.clone();
        let selector_mapper = map_edit.clone();
        let remove_mapper = map_edit.clone();
        let selector_input = value.selector.to_input();
        list = list.push(
            container(
                column![
                    row![
                        text_input("名称", &value.name)
                            .on_input(move |input| name_mapper(
                                WorkflowNodeFieldEdit::NamedValueNameChanged(index, input)
                            ))
                            .padding([8, 10])
                            .style(settings_text_input_style),
                        text_input("类型", &value.value_type)
                            .on_input(move |input| type_mapper(
                                WorkflowNodeFieldEdit::NamedValueTypeChanged(index, input)
                            ))
                            .padding([8, 10])
                            .style(settings_text_input_style),
                        button(text("删除").size(12)).style(danger_action_btn_style).on_press(
                            remove_mapper(WorkflowNodeFieldEdit::NamedValueRemoved(index))
                        ),
                    ]
                    .spacing(8),
                    text_input("节点.变量", &selector_input)
                        .on_input(move |input| selector_mapper(
                            WorkflowNodeFieldEdit::NamedValueSelectorChanged(index, input)
                        ))
                        .padding([8, 10])
                        .style(settings_text_input_style),
                ]
                .spacing(8),
            )
            .padding(10)
            .style(node_editor_field_card_style),
        );
    }
    list.push(
        button(text("新增命名项").size(12))
            .style(rounded_action_btn_style)
            .on_press(map_edit(WorkflowNodeFieldEdit::NamedValueAdded)),
    )
    .into()
}

pub(super) fn build_key_value_list<'a, F>(
    values: &'a [WorkflowKeyValueDraft],
    map_edit: F,
) -> Element<'a, Message>
where
    F: Fn(WorkflowNodeFieldEdit) -> Message + Clone + 'static,
{
    let mut list = column![].spacing(8);
    for (index, value) in values.iter().enumerate() {
        let key_mapper = map_edit.clone();
        let value_mapper = map_edit.clone();
        let remove_mapper = map_edit.clone();
        list = list.push(
            row![
                text_input("键", &value.key)
                    .on_input(move |input| key_mapper(WorkflowNodeFieldEdit::KeyValueKeyChanged(
                        index, input
                    )))
                    .padding([8, 10])
                    .style(settings_text_input_style),
                text_input("值", &value.value)
                    .on_input(move |input| value_mapper(
                        WorkflowNodeFieldEdit::KeyValueValueChanged(index, input)
                    ))
                    .padding([8, 10])
                    .style(settings_text_input_style),
                button(text("删除").size(12))
                    .style(danger_action_btn_style)
                    .on_press(remove_mapper(WorkflowNodeFieldEdit::KeyValueRemoved(index))),
            ]
            .spacing(8),
        );
    }
    list.push(
        button(text("新增键值").size(12))
            .style(rounded_action_btn_style)
            .on_press(map_edit(WorkflowNodeFieldEdit::KeyValueAdded)),
    )
    .into()
}

pub(super) fn build_model_fields<'a, F>(
    model: &'a WorkflowModelDraft,
    map_edit: F,
) -> Element<'a, Message>
where
    F: Fn(WorkflowNodeFieldEdit) -> Message + Clone + 'static,
{
    let provider_mapper = map_edit.clone();
    let name_mapper = map_edit.clone();
    column![
        text_input("模型供应商", &model.provider)
            .on_input(move |value| provider_mapper(WorkflowNodeFieldEdit::ModelProviderChanged(
                value
            )))
            .padding([10, 12])
            .style(settings_text_input_style),
        text_input("模型名称", &model.name)
            .on_input(move |value| name_mapper(WorkflowNodeFieldEdit::ModelNameChanged(value)))
            .padding([10, 12])
            .style(settings_text_input_style),
        text_input("模型模式", &model.mode)
            .on_input(move |value| map_edit(WorkflowNodeFieldEdit::ModelModeChanged(value)))
            .padding([10, 12])
            .style(settings_text_input_style),
    ]
    .spacing(8)
    .into()
}

pub(super) fn build_retry_fields<'a, F>(
    retry: WorkflowNodeRetryDraft,
    map_edit: F,
) -> Element<'a, Message>
where
    F: Fn(WorkflowNodeFieldEdit) -> Message + Clone + 'static,
{
    let enabled_mapper = map_edit.clone();
    let retries_mapper = map_edit.clone();
    column![
        row![
            text("启用失败重试").size(12),
            Space::new().width(Length::Fill),
            toggler(retry.enabled).on_toggle(move |enabled| enabled_mapper(
                WorkflowNodeFieldEdit::RetryEnabledChanged(enabled)
            )),
        ]
        .align_y(Alignment::Center),
        row![
            text("最大次数").size(12).width(Length::Fixed(72.0)),
            slider(1.0..=10.0, retry.max_retries.max(1) as f32, move |value: f32| {
                retries_mapper(WorkflowNodeFieldEdit::RetryMaxRetriesChanged(
                    (value.round() as u8).clamp(1, 10),
                ))
            })
            .step(1.0_f32),
            text(retry.max_retries.to_string()).size(12),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        row![
            text("间隔毫秒").size(12).width(Length::Fixed(72.0)),
            slider(100.0..=5000.0, retry.retry_interval.max(100) as f32, move |value: f32| {
                map_edit(WorkflowNodeFieldEdit::RetryIntervalChanged(
                    ((value.round() as u16) / 100).max(1) * 100,
                ))
            })
            .step(100.0_f32),
            text(retry.retry_interval.to_string()).size(12),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .spacing(10)
    .into()
}

pub(super) fn node_editor_field_card_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.palette();
    let extended = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(
            extended.background.weak.color.scale_alpha(if is_dark_theme(theme) {
                0.58
            } else {
                0.72
            }),
        )),
        border: Border {
            width: 1.0,
            color: extended.background.strong.color.scale_alpha(if is_dark_theme(theme) {
                0.62
            } else {
                0.18
            }),
            radius: 10.0.into(),
        },
        text_color: Some(palette.text),
        ..Default::default()
    }
}
