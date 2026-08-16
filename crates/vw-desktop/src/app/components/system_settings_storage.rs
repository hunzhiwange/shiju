//! 系统设置中 storage 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, settings_checkbox_style, settings_control_with_hint, settings_divider,
    settings_muted_text_style, settings_panel, settings_pick_list_menu_style,
    settings_pick_list_style, settings_section_card, settings_text_input_style,
};
use crate::app::message::settings::StorageMessage;
use crate::app::{App, Message, message};
use iced::widget::{checkbox, column, container, pick_list, row, text, text_input};
use iced::{Alignment, Element, Length};

const STORAGE_PROVIDER_OPTIONS: [&str; 3] = ["postgres", "mariadb", "sqlite"];

fn field_row<'a>(
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(
        row![
            column![
                text(label).size(13),
                text(description).size(11).style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
            container(control.into()).width(Length::Fill),
        ]
        .spacing(22)
        .align_y(Alignment::Center),
    )
    .padding([14, 0])
    .width(Length::Fill)
    .into()
}

fn text_row<'a>(
    label: &'static str,
    description: &'static str,
    placeholder: &'static str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    field_row(
        label,
        description,
        text_input(placeholder, value)
            .on_input(on_input)
            .padding([10, 12])
            .size(13)
            .style(settings_text_input_style)
            .width(Length::Fill),
    )
}

/// 构建或处理 `view` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回可交给 Iced 渲染树使用的 `Element`，其中已绑定必要的消息回调。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn embedded_view(app: &App) -> Element<'_, Message> {
    let s = &app.storage_settings;

    let provider_pick = pick_list(
        STORAGE_PROVIDER_OPTIONS,
        STORAGE_PROVIDER_OPTIONS.into_iter().find(|option| *option == s.provider.as_str()),
        |value| {
            Message::Settings(message::SettingsMessage::Storage(StorageMessage::ProviderChanged(
                value.to_string(),
            )))
        },
    )
    .padding([10, 14])
    .text_size(13)
    .style(settings_pick_list_style)
    .menu_style(settings_pick_list_menu_style)
    .width(Length::Fixed(280.0));

    let provider_row = field_row("存储类型", "选择后端数据库类型。", provider_pick);

    let db_url_row = field_row(
        "数据库地址",
        "",
        settings_control_with_hint(
            text_input("postgres://user:pass@host:5432/db", &s.db_url_input)
                .on_input(|value| {
                    Message::Settings(message::SettingsMessage::Storage(
                        StorageMessage::DbUrlChanged(value),
                    ))
                })
                .padding([10, 12])
                .size(13)
                .style(settings_text_input_style)
                .width(Length::Fill),
            "远程 SQL 存储使用连接串；留空时不写入数据库地址。",
        ),
    );

    let schema_row =
        text_row("Schema 名称", "数据库 schema 名称。", "public", &s.schema, |value| {
            Message::Settings(message::SettingsMessage::Storage(StorageMessage::SchemaChanged(
                value,
            )))
        });

    let table_row = text_row(
        "数据表",
        "保存记录所使用的数据表名。",
        "memories",
        &s.table,
        |value| {
            Message::Settings(message::SettingsMessage::Storage(StorageMessage::TableChanged(
                value,
            )))
        },
    );

    let timeout_row = field_row(
        "连接超时",
        "",
        settings_control_with_hint(
            text_input("秒；留空表示使用默认", &s.connect_timeout_secs_input)
                .on_input(|value| {
                    Message::Settings(message::SettingsMessage::Storage(
                        StorageMessage::ConnectTimeoutSecsChanged(value),
                    ))
                })
                .padding([10, 12])
                .size(13)
                .style(settings_text_input_style)
                .width(Length::Fill),
            "仅接受整数秒；留空会写入 None。",
        ),
    );

    let tls_row = field_row(
        "TLS 加密",
        "控制远程存储连接是否启用 TLS。",
        checkbox(s.tls)
            .label("启用存储连接 TLS")
            .on_toggle(|value| {
                Message::Settings(message::SettingsMessage::Storage(StorageMessage::TlsToggled(
                    value,
                )))
            })
            .style(settings_checkbox_style),
    );

    let content = column![
        settings_section_card("连接参数", "选择数据库类型并配置基础连接信息。"),
        settings_panel(column![provider_row, settings_divider(), db_url_row].spacing(0)),
        settings_section_card("命名与超时", "配置 schema、表名和连接超时。"),
        settings_panel(
            column![schema_row, settings_divider(), table_row, settings_divider(), timeout_row]
                .spacing(0),
        ),
        settings_section_card("安全", "配置与远程存储的加密连接策略。"),
        settings_panel(column![tls_row].spacing(0)),
    ]
    .spacing(16)
    .width(Length::Fill);

    content.into()
}
#[cfg(test)]
#[path = "system_settings_storage_tests.rs"]
mod system_settings_storage_tests;
