//! 心跳配置设置界面组件
//!
//! 本模块提供了心跳（Heartbeat）功能的设置界面，允许用户配置周期性心跳任务的各项参数。
//! 心跳任务会按照配置的间隔时间定期触发，用于定时执行健康检查、状态汇报等任务。
//!
//! # 主要功能
//!
//! - **启用/禁用心跳**：通过复选框控制心跳任务是否激活
//! - **配置心跳间隔**：通过滑块设置心跳触发的时间间隔（1-1440分钟）
//! - **设置消息内容**：配置心跳任务的默认消息或兜底说明
//! - **指定目标通道**：配置心跳结果投递的目标通道（如 telegram）
//! - **设置接收者**：配置心跳结果的接收者标识（如 chat_id）
//! - **帮助模态框**：提供详细的使用说明和配置示例
//!
//! # 配置持久化
//!
//! 用户的配置会通过 Gateway 保存到共享数据库的 `heartbeat` 字段中。

use desktop_ui_controls::system_settings_common::{
    settings_checkbox_style, settings_field_label, settings_divider,
    settings_group_card,
    settings_help_button_with_locale as settings_help_button,
    settings_text_input, settings_value_badge,
};
use desktop_i18n::I18n;

use iced::widget::{checkbox, column, container, row, slider, text};
use iced::{Alignment, Element, Length};


#[derive(Debug, Clone)]
pub enum SettingsMessage { HeartbeatHelpOpen,HeartbeatHelpClose,HeartbeatEnabledToggled(bool),HeartbeatIntervalChanged(u32),HeartbeatMessageChanged(String),HeartbeatTargetChanged(String),HeartbeatToChanged(String) }
mod message { pub use super::SettingsMessage; }
#[derive(Debug, Clone)]
pub enum Message { None, CopyCode(String), OpenUrl(String), Settings(SettingsMessage) }
pub struct ViewData<'a> {
 pub heartbeat_settings: &'a crate::app_init::state::HeartbeatSettingsState,
 pub desktop_locale: config_ui::DesktopLocale,
 pub app_theme: &'a iced::Theme,
 pub last_copied_code_hash: Option<u64>,
 pub help_content: for<'b> fn(&'b str, std::borrow::Cow<'b,str>, Option<iced::Color>, I18n) -> Element<'b,Message>,
}

fn hover_help_label<'a>(app: &ViewData<'_>, title: &'a str, label: iced::widget::Text<'a>, description: impl Into<std::borrow::Cow<'a,str>>) -> iced::widget::Row<'a,Message> {
    desktop_ui_controls::system_settings_common::help::settings_hover_help_label(label, (app.help_content)(title, description.into(), Some(app.app_theme.palette().text), I18n::new(app.desktop_locale)))
}
fn settings_page_heading_with_help<'a>(app: &ViewData<'_>, title: &'a str, description: impl Into<std::borrow::Cow<'a,str>>) -> Element<'a,Message> {
    hover_help_label(app, title, text(title).size(28), description).wrap().into()
}
fn settings_group_heading_with_help<'a>(app: &ViewData<'_>, title: &'a str, description: impl Into<std::borrow::Cow<'a,str>>) -> Element<'a,Message> {
    hover_help_label(app, title, text(title).size(17), description).wrap().into()
}
fn settings_flexible_row_with_help<'a>(app: &ViewData<'_>, title: &'a str, description: impl Into<std::borrow::Cow<'a,str>>, width: f32, control: impl Into<Element<'a,Message>>) -> iced::widget::Container<'a,Message> {
    let label = hover_help_label(app, title, settings_field_label(title), description).width(Length::Fill).wrap();
    desktop_ui_controls::system_settings_common::settings_flexible_row_with_label(label,width,control)
}

fn field_row<'a>(
    app: &ViewData<'_>,
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    settings_flexible_row_with_help(app,
        i18n.__(label),
        i18n.__(description),
        320.0,
        control,
    )
    .into()
}

fn text_row<'a>(
    app: &ViewData<'_>,
    label: &'static str,
    description: &'static str,
    placeholder: &'static str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    field_row(
        app,
        label,
        description,
        settings_text_input(placeholder, value).on_input(on_input).width(Length::Fill),
    )
}

/// 渲染心跳配置设置界面
///
/// 该函数创建一个完整的心跳配置界面，包括：
/// - 标题和配置文件路径提示
/// - 各项配置输入控件（启用开关、间隔滑块、文本输入框等）
/// - 帮助按钮和帮助模态框
/// - 错误信息显示（如果配置保存失败）
///
/// # 参数
///
/// * `app` - 应用状态引用，包含当前的心跳配置设置
///
/// # 返回值
///
/// 返回一个 ICED Element，可嵌入到更大的界面中显示
///
/// # 示例
///
/// ```ignore
/// let settings_view = view(&app);
/// // 将 settings_view 添加到主界面的某个容器中
/// ```
///
/// # 界面布局
///
/// 1. 标题栏：显示"心跳配置"文本和帮助按钮
/// 2. 配置文件路径提示
/// 3. 启用开关行：控制心跳是否激活
/// 4. 间隔配置行：通过滑块设置心跳间隔（1-1440分钟）
/// 5. 消息输入行：设置心跳消息内容
/// 6. 目标通道输入行：设置投递目标通道
/// 7. 接收者输入行：设置接收者标识
/// 8. 错误提示（如果有）
/// 9. 帮助模态框（如果启用）
pub fn view<'a>(app: &ViewData<'a>) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let s = &app.heartbeat_settings;
    let help_btn = settings_help_button(
        app.desktop_locale,
        Message::Settings(message::SettingsMessage::HeartbeatHelpOpen),
    );

    let enabled_row = field_row(
        app,
        "启用",
        "控制是否开启周期心跳。",
        checkbox(s.enabled)
            .label(i18n.__("开启周期心跳"))
            .on_toggle(|v| Message::Settings(message::SettingsMessage::HeartbeatEnabledToggled(v)))
            .style(settings_checkbox_style),
    );

    let interval_slider = slider(1.0..=1440.0, s.interval_minutes as f32, |v| {
        Message::Settings(message::SettingsMessage::HeartbeatIntervalChanged(v.round() as u32))
    })
    .width(Length::Fill);

    let interval_row = field_row(
        app,
        "间隔分钟",
        "周期性触发心跳的时间间隔。",
        row![
            interval_slider,
            container(settings_value_badge(
                desktop_i18n::__!(i18n, "%u 分钟", s.interval_minutes)
                    .expect("valid heartbeat interval template")
            ))
            .align_right(104.0),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    );

    let message_row = text_row(
        app,
        "消息",
        "没有 HEARTBEAT.md 任务时的兜底说明。",
        i18n.__("可选"),
        &s.message_input,
        |v| Message::Settings(message::SettingsMessage::HeartbeatMessageChanged(v)),
    );

    let target_row = text_row(
        app,
        "目标通道",
        "可选的心跳结果投递通道。",
        i18n.__("可选，例如 telegram"),
        &s.target_input,
        |v| Message::Settings(message::SettingsMessage::HeartbeatTargetChanged(v)),
    );

    let to_row = text_row(
        app,
        "接收者",
        "目标通道对应的接收者标识。",
        i18n.__("可选"),
        &s.to_input,
        |v| Message::Settings(message::SettingsMessage::HeartbeatToChanged(v)),
    );

    let col = column![
        row![
            container(settings_page_heading_with_help(app,
                i18n.__("心跳配置"),
                i18n.__("配置心跳任务的间隔、兜底消息与投递目标。")
            ))
            .width(Length::Fill),
            help_btn
        ]
        .align_y(Alignment::Start),
        settings_group_heading_with_help(app,
            i18n.__("基础行为"),
            i18n.__("控制心跳开关与执行间隔。")
        ),
        settings_group_card(column![enabled_row, settings_divider(), interval_row].spacing(0)),
        settings_group_heading_with_help(app,
            i18n.__("投递内容"),
            i18n.__("配置兜底消息、目标通道和接收者。")
        ),
        settings_group_card(
            column![message_row, settings_divider(), target_row, settings_divider(), to_row]
                .spacing(0),
        ),
    ]
    .spacing(16)
    .width(Length::Fill);

    col.into()
}

pub fn view_overlays<'a, M: Clone + 'static>(app: &ViewData<'a>, dialog: Element<'a, M>, map: fn(Message) -> M) -> Element<'a, M> {
    let s = &app.heartbeat_settings;
    if !s.show_help_modal {
        return dialog;
    }

    let help_text = r#"心跳配置说明

一、作用
- heartbeat 是一个“定时心跳任务”配置。
- 运行时会按 interval_minutes 周期触发一次心跳。
- 每次心跳会优先读取工作区下的 HEARTBEAT.md 任务；若没有任务，可使用 message 作为兜底说明。

二、字段含义
1) enabled
- 类型：布尔（true / false）
- 含义：是否启用心跳机制。
- false：关闭，不会周期触发。
- true：开启，按 interval_minutes 触发。

2) interval_minutes
- 类型：整数（分钟）
- 含义：心跳触发间隔。
- 例：30 表示每 30 分钟触发一次。
- 建议：测试环境可用 5-10，生产环境常用 15-60。

3) message
- 类型：字符串或 null
- 含义：当 HEARTBEAT.md 没有可执行任务时的兜底文本。
- 为空（null）表示不提供兜底文本。
- 示例："巡检服务状态，汇总错误日志与告警"。

4) target
- 类型：字符串或 null
- 含义：结果投递目标通道（例如 telegram）。
- 为空（null）表示不指定额外投递通道。

5) to
- 类型：字符串或 null
- 含义：投递目标接收者（chat_id / user_id / room_id 等）。
- 通常在 target 不为空时一起配置。

三、典型示例

示例 A：只在本地定时跑，不投递
{
  "heartbeat": {
    "enabled": true,
    "interval_minutes": 30,
    "message": "检查服务健康与待办",
    "target": null,
    "to": null
  }
}

示例 B：投递到 telegram
{
  "heartbeat": {
    "enabled": true,
    "interval_minutes": 15,
    "message": "定时汇报系统状态",
    "target": "telegram",
    "to": "123456789"
  }
}

示例 C：按当前默认结构
{
  "heartbeat": {
    "enabled": false,
    "interval_minutes": 30,
    "message": null,
    "target": null,
    "to": null
  }
}

四、排查建议（如果你觉得没生效）
1) 先确认 enabled=true。
2) interval_minutes 不要太大，先设 1-5 分钟验证。
3) 若配置 target，请同时填写 to。
4) 确认 Gateway 可用且系统设置保存成功。
5) 修改后重启应用，观察运行日志是否出现 heartbeat tick。
"#;

    with_help_modal(
        app,
        dialog,
        "Heartbeat 配置帮助",
        help_text,
        Message::Settings(message::SettingsMessage::HeartbeatHelpClose),
        map,
    )
}
#[cfg(test)]
#[path = "system_settings_heartbeat_tests.rs"]
mod system_settings_heartbeat_tests;

fn with_help_modal<'a, M: Clone + 'static>(app: &ViewData<'_>, base: Element<'a,M>, title: &str, body: &str, close: Message, map: fn(Message) -> M) -> Element<'a,M> {
 let i18n=I18n::new(app.desktop_locale);
 let body=i18n.__(body).to_string();
 let (icon,tip)=desktop_ui_controls::system_settings_common::settings_help_copy_state(app.last_copied_code_hash,&body);
 let tip=desktop_i18n::settings_runtime::text(app.desktop_locale,tip);
 let copy=Message::CopyCode(body.clone());
 desktop_ui_controls::system_settings_common::help::with_settings_help_modal(base,i18n.__(title).to_string(),body,icon,tip,map(copy),map(close))
}
