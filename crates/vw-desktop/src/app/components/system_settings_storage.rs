//! 系统设置中 storage 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use super::system_settings_memory::{field_row, text_row};
use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_TEXT_SIZE, settings_checkbox_style, settings_divider, settings_group_card,
    settings_group_heading, settings_select,
};
use crate::app::i18n::settings_runtime::text as runtime_text;
use crate::app::message::settings::StorageMessage;
use crate::app::{App, Message, message};
use iced::widget::{checkbox, column};
use iced::{Element, Length};

const STORAGE_PROVIDER_OPTIONS: [&str; 3] = ["postgres", "mariadb", "sqlite"];

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
    let locale = app.desktop_locale;
    let s = &app.storage_settings;

    let provider_pick = settings_select(
        STORAGE_PROVIDER_OPTIONS,
        STORAGE_PROVIDER_OPTIONS.into_iter().find(|option| *option == s.provider.as_str()),
        |value| {
            Message::Settings(message::SettingsMessage::Storage(StorageMessage::ProviderChanged(
                value.to_string(),
            )))
        },
    )
    .width(Length::Fill);

    let provider_row = field_row(
        runtime_text(locale, "存储类型"),
        runtime_text(locale, "选择后端数据库类型。"),
        provider_pick,
    );

    let db_url_row = text_row(
        runtime_text(locale, "数据库地址"),
        runtime_text(locale, "远程 SQL 存储使用连接串；留空时不写入数据库地址。"),
        "postgres://user:pass@host:5432/db",
        &s.db_url_input,
        |value| {
            Message::Settings(message::SettingsMessage::Storage(StorageMessage::DbUrlChanged(
                value,
            )))
        },
    );

    let schema_row = text_row(
        runtime_text(locale, "Schema 名称"),
        runtime_text(locale, "数据库 schema 名称。"),
        "public",
        &s.schema,
        |value| {
            Message::Settings(message::SettingsMessage::Storage(StorageMessage::SchemaChanged(
                value,
            )))
        },
    );

    let table_row = text_row(
        runtime_text(locale, "数据表"),
        runtime_text(locale, "保存记录所使用的数据表名。"),
        "memories",
        &s.table,
        |value| {
            Message::Settings(message::SettingsMessage::Storage(StorageMessage::TableChanged(
                value,
            )))
        },
    );

    let timeout_row = text_row(
        runtime_text(locale, "连接超时"),
        runtime_text(locale, "仅接受整数秒；留空会写入 None。"),
        runtime_text(locale, "秒；留空表示使用默认"),
        &s.connect_timeout_secs_input,
        |value| {
            Message::Settings(message::SettingsMessage::Storage(
                StorageMessage::ConnectTimeoutSecsChanged(value),
            ))
        },
    );

    let tls_row = field_row(
        runtime_text(locale, "TLS 加密"),
        runtime_text(locale, "控制远程存储连接是否启用 TLS。"),
        checkbox(s.tls)
            .label(runtime_text(locale, "启用存储连接 TLS"))
            .text_size(SETTINGS_CONTROL_TEXT_SIZE)
            .on_toggle(|value| {
                Message::Settings(message::SettingsMessage::Storage(StorageMessage::TlsToggled(
                    value,
                )))
            })
            .style(settings_checkbox_style),
    );

    let content = column![
        settings_group_heading(
            runtime_text(locale, "连接参数"),
            runtime_text(locale, "选择数据库类型并配置基础连接信息。")
        ),
        settings_group_card(column![provider_row, settings_divider(), db_url_row].spacing(0)),
        settings_group_heading(
            runtime_text(locale, "命名与超时"),
            runtime_text(locale, "配置 schema、表名和连接超时。")
        ),
        settings_group_card(
            column![schema_row, settings_divider(), table_row, settings_divider(), timeout_row]
                .spacing(0),
        ),
        settings_group_heading(
            runtime_text(locale, "安全"),
            runtime_text(locale, "配置与远程存储的加密连接策略。")
        ),
        settings_group_card(column![tls_row].spacing(0)),
    ]
    .spacing(16)
    .width(Length::Fill);

    content.into()
}
#[cfg(test)]
#[path = "system_settings_storage_tests.rs"]
mod system_settings_storage_tests;
