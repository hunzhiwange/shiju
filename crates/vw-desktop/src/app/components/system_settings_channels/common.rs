//! 系统设置中渠道配置页面的分组界面与配置项渲染。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::number_input::NumberInput;
use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, rounded_action_btn_style, settings_checkbox_style,
    settings_control_with_hint, settings_divider, settings_muted_text_style, settings_panel,
    settings_pick_list_menu_style, settings_pick_list_style, settings_text_editor_style,
    settings_text_input_style, settings_value_badge,
};
use crate::app::message::settings::{ChannelsMessage, SettingsMessage};
use crate::app::{App, Message};
use iced::widget::{
    button, checkbox, column, container, pick_list, row, text, text_editor, text_input,
};
use iced::{Alignment, Element, Length};
use vw_config_types::channels::{GroupReplyConfig, LarkReceiveMode, QQReceiveMode};

/// `LABEL_WIDTH` 常量，用于表达本模块对该领域对象的建模。
///
/// 该定义保持在当前模块职责内，调用方应通过显式字段、变体或别名理解其语义。
pub(super) const LABEL_WIDTH: f32 = SETTINGS_LABEL_WIDTH;
const MULTILINE_EDITOR_HEIGHT: f32 = 92.0;

#[derive(Clone, Copy, PartialEq, Eq)]
/// `LabeledOption` 结构体，用于表达本模块对该领域对象的建模。
///
/// 该定义保持在当前模块职责内，调用方应通过显式字段、变体或别名理解其语义。
pub(super) struct LabeledOption {
    pub(super) value: &'static str,
    label: &'static str,
}

impl std::fmt::Display for LabeledOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

/// `GROUP_REPLY_MODE_OPTIONS` 常量，用于表达本模块对该领域对象的建模。
///
/// 该定义保持在当前模块职责内，调用方应通过显式字段、变体或别名理解其语义。
pub(super) const GROUP_REPLY_MODE_OPTIONS: [LabeledOption; 2] = [
    LabeledOption { value: "all_messages", label: "全部消息" },
    LabeledOption { value: "mention_only", label: "仅被提及" },
];

/// `LARK_RECEIVE_MODE_OPTIONS` 常量，用于表达本模块对该领域对象的建模。
///
/// 该定义保持在当前模块职责内，调用方应通过显式字段、变体或别名理解其语义。
pub(super) const LARK_RECEIVE_MODE_OPTIONS: [LabeledOption; 2] = [
    LabeledOption { value: "websocket", label: "WebSocket" },
    LabeledOption { value: "webhook", label: "Webhook" },
];

/// `QQ_RECEIVE_MODE_OPTIONS` 常量，用于表达本模块对该领域对象的建模。
///
/// 该定义保持在当前模块职责内，调用方应通过显式字段、变体或别名理解其语义。
pub(super) const QQ_RECEIVE_MODE_OPTIONS: [LabeledOption; 2] = [
    LabeledOption { value: "webhook", label: "Webhook" },
    LabeledOption { value: "websocket", label: "WebSocket" },
];

fn input_value<'a>(app: &'a App, key: &str, fallback: &'a str) -> &'a str {
    app.channels_settings.text_inputs.get(key).map(String::as_str).unwrap_or(fallback)
}

fn named_instance_field<'a>(key: &'a str, prefix: &str) -> Option<&'a str> {
    let rest = key.strip_prefix(prefix)?.strip_prefix('.')?;
    let (_, field) = rest.split_once('.')?;
    Some(field)
}

/// 构建或处理 `group_reply_mode_value` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub(super) fn group_reply_mode_value(group_reply: &Option<GroupReplyConfig>) -> &'static str {
    match group_reply.as_ref().and_then(|cfg| cfg.mode) {
        Some(vw_config_types::channels::GroupReplyMode::MentionOnly) => "mention_only",
        _ => "all_messages",
    }
}

/// 构建或处理 `lark_receive_mode_value` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub(super) fn lark_receive_mode_value(mode: LarkReceiveMode) -> &'static str {
    match mode {
        LarkReceiveMode::Webhook => "webhook",
        LarkReceiveMode::Websocket => "websocket",
    }
}

/// 构建或处理 `qq_receive_mode_value` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub(super) fn qq_receive_mode_value(mode: QQReceiveMode) -> &'static str {
    match mode {
        QQReceiveMode::Webhook => "webhook",
        QQReceiveMode::Websocket => "websocket",
    }
}

fn editor_style(theme: &iced::Theme, status: text_editor::Status) -> text_editor::Style {
    settings_text_editor_style(theme, status)
}

pub(super) fn is_multiline_list_key(key: &str) -> bool {
    matches!(
        key,
        "telegram.allowed_users"
            | "telegram.group_reply.allowed_sender_ids"
            | "discord.allowed_users"
            | "discord.group_reply.allowed_sender_ids"
            | "slack.allowed_users"
            | "slack.group_reply.allowed_sender_ids"
            | "mattermost.allowed_users"
            | "mattermost.group_reply.allowed_sender_ids"
            | "imessage.allowed_contacts"
            | "signal.allowed_from"
            | "whatsapp.allowed_numbers"
            | "linq.allowed_senders"
            | "wati.allowed_numbers"
            | "nextcloud_talk.allowed_users"
            | "email.allowed_senders"
            | "irc.channels"
            | "irc.allowed_users"
            | "lark.allowed_users"
            | "lark.group_reply.allowed_sender_ids"
            | "feishu.allowed_users"
            | "feishu.group_reply.allowed_sender_ids"
            | "dingtalk.allowed_users"
            | "qq.allowed_users"
            | "nostr.relays"
            | "nostr.allowed_pubkeys"
            | "clawdtalk.allowed_destinations"
    ) || matches!(
        named_instance_field(key, "feishu_instances"),
        Some("allowed_users" | "group_reply.allowed_sender_ids")
    ) || matches!(named_instance_field(key, "dingtalk_instances"), Some("allowed_users"))
}

pub(super) fn localized_label(label: &'static str) -> &'static str {
    match label {
        "bot_token" => "机器人令牌",
        "instance_id" => "实例 ID",
        "allowed_users" => "允许用户",
        "group_reply.mode" => "群聊回复模式",
        "group_reply.allowed_sender_ids" => "允许发送者 ID",
        "mention_only" => "仅提及时回复",
        "homeserver" => "Homeserver 地址",
        "access_token" => "访问令牌",
        "user_id" => "用户 ID",
        "device_id" => "设备 ID",
        "room_id" => "房间 ID",
        "http_url" => "HTTP 地址",
        "account" => "账号",
        "group_id" => "群组 ID",
        "ignore_attachments" => "忽略附件",
        "ignore_stories" => "忽略动态",
        "phone_number_id" => "电话号码 ID",
        "verify_token" => "验证令牌",
        "app_secret" => "应用密钥",
        "session_path" => "会话路径",
        "pair_phone" => "配对手机号",
        "pair_code" => "配对码",
        "allowed_numbers" => "允许号码",
        "api_token" => "API 令牌",
        "from_phone" => "发送号码",
        "signing_secret" => "签名密钥",
        "api_url" => "API 地址",
        "tenant_id" => "租户 ID",
        "base_url" => "基础地址",
        "app_token" => "应用令牌",
        "webhook_secret" => "Webhook 密钥",
        "imap_host" => "IMAP 主机",
        "imap_port" => "IMAP 端口",
        "imap_folder" => "IMAP 文件夹",
        "smtp_host" => "SMTP 主机",
        "smtp_port" => "SMTP 端口",
        "smtp_tls" => "SMTP 使用 TLS",
        "username" => "用户名",
        "password" => "密码",
        "from_address" => "发件地址",
        "idle_timeout_secs" => "空闲超时",
        "allowed_senders" => "允许发送者",
        "server" => "服务器",
        "port" => "端口",
        "nickname" => "昵称",
        "channels" => "频道列表",
        "server_password" => "服务器密码",
        "nickserv_password" => "NickServ 密码",
        "sasl_password" => "SASL 密码",
        "verify_tls" => "验证 TLS",
        "app_id" => "应用 ID",
        "encrypt_key" => "加密密钥",
        "verification_token" => "验证令牌",
        "use_feishu" => "使用飞书端点",
        "receive_mode" => "接收模式",
        "draft_update_interval_ms" => "草稿刷新间隔",
        "max_draft_edits" => "最大草稿编辑次数",
        "client_id" => "应用 Key",
        "client_secret" => "应用密钥",
        "private_key" => "私钥",
        "relays" => "中继列表",
        "allowed_pubkeys" => "允许公钥",
        "api_key" => "API 密钥",
        "connection_id" => "连接 ID",
        "from_number" => "发送号码",
        "allowed_destinations" => "允许目的地",
        _ => label,
    }
}

pub(super) fn multiline_placeholder(placeholder: &'static str) -> &'static str {
    match placeholder {
        "逗号或换行分隔" => "每行一个，亦支持逗号分隔",
        "电话号码或邮箱，逗号或换行分隔" => "每行一个电话号码或邮箱，亦支持逗号分隔",
        "逗号或换行分隔，支持 *" => "每行一个，亦支持逗号分隔，可使用 *",
        _ => placeholder,
    }
}

/// 构建或处理 `text_row` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub(super) fn text_row<'a>(
    app: &'a App,
    label: &'static str,
    placeholder: &'static str,
    value: &'a str,
    key: &'static str,
    secure: bool,
) -> Element<'a, Message> {
    text_row_with_key(app, label, placeholder, value, key.to_string(), secure)
}

pub(super) fn text_row_with_key<'a>(
    app: &'a App,
    label: &'static str,
    placeholder: &'static str,
    value: &'a str,
    key: String,
    secure: bool,
) -> Element<'a, Message> {
    let display_label = localized_label(label);
    if is_multiline_list_key(&key) {
        let editor = app
            .channels_settings
            .text_editors
            .get(&key)
            .expect("multiline channel editor should be initialized");
        let key_for_action = key.clone();
        let editor = container(
            text_editor(editor)
                .placeholder(multiline_placeholder(placeholder))
                .on_action(move |action| {
                    Message::Settings(SettingsMessage::Channels(ChannelsMessage::TextEditorAction(
                        key_for_action.clone(),
                        action,
                    )))
                })
                .padding([9, 12])
                .height(Length::Fixed(MULTILINE_EDITOR_HEIGHT))
                .style(editor_style),
        )
        .width(Length::Fill);
        let editor: Element<'a, Message> = if placeholder == "逗号或换行分隔，支持 *" {
            let hint = if label == "group_reply.allowed_sender_ids" {
                "填写 * 表示允许所有发送者，推荐使用。"
            } else {
                "填写 * 表示允许所有用户，推荐使用。"
            };
            settings_control_with_hint(editor, hint)
        } else {
            editor.into()
        };
        return row![text(display_label).size(13).width(Length::Fixed(LABEL_WIDTH)), editor,]
            .spacing(16)
            .align_y(Alignment::Start)
            .into();
    }

    let key_for_input = key.clone();
    let mut input = text_input(placeholder, input_value(app, &key, value))
        .on_input(move |next| {
            Message::Settings(SettingsMessage::Channels(ChannelsMessage::TextChanged(
                key_for_input.clone(),
                next,
            )))
        })
        .padding([10, 12])
        .size(13)
        .style(settings_text_input_style)
        .width(Length::Fill);
    if secure {
        input = input.secure(true);
    }

    row![text(display_label).size(13).width(Length::Fixed(LABEL_WIDTH)), input,]
        .spacing(16)
        .align_y(Alignment::Center)
        .into()
}

/// 构建或处理 `bool_row` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub(super) fn bool_row<'a>(
    label: &'static str,
    checked: bool,
    description: &'static str,
    key: &'static str,
) -> Element<'a, Message> {
    bool_row_with_key(label, checked, description, key.to_string())
}

pub(super) fn bool_row_with_key<'a>(
    label: &'static str,
    checked: bool,
    description: &'static str,
    key: String,
) -> Element<'a, Message> {
    let key_for_toggle = key.clone();
    row![
        text(localized_label(label)).size(13).width(Length::Fixed(LABEL_WIDTH)),
        checkbox(checked)
            .label(description)
            .on_toggle(move |next| {
                Message::Settings(SettingsMessage::Channels(ChannelsMessage::BoolToggled(
                    key_for_toggle.clone(),
                    next,
                )))
            })
            .style(settings_checkbox_style),
    ]
    .spacing(16)
    .align_y(Alignment::Center)
    .into()
}

/// 构建或处理 `number_row` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub(super) fn number_row<'a>(
    label: &'static str,
    value: u32,
    min: u32,
    max: u32,
    suffix: &'static str,
    key: &'static str,
) -> Element<'a, Message> {
    number_row_with_key(label, value, min, max, suffix, key.to_string())
}

pub(super) fn number_row_with_hint<'a>(
    label: &'static str,
    value: u32,
    min: u32,
    max: u32,
    suffix: &'static str,
    key: &'static str,
    hint: &'static str,
) -> Element<'a, Message> {
    let display = if suffix.is_empty() { value.to_string() } else { format!("{value} {suffix}") };
    let key = key.to_string();

    row![
        text(localized_label(label)).size(13).width(Length::Fixed(LABEL_WIDTH)),
        settings_control_with_hint(
            row![
                NumberInput::new(value as f32, min as f32, max as f32, 1.0, 0, 0.15, move |raw| {
                    Message::Settings(SettingsMessage::Channels(ChannelsMessage::NumberChanged(
                        key.clone(),
                        raw.round() as u32,
                    )))
                },)
                .settings_style(),
                settings_value_badge(display),
            ]
            .spacing(16)
            .align_y(Alignment::Center),
            hint,
        ),
    ]
    .spacing(16)
    .align_y(Alignment::Center)
    .into()
}

pub(super) fn number_row_with_key<'a>(
    label: &'static str,
    value: u32,
    min: u32,
    max: u32,
    suffix: &'static str,
    key: String,
) -> Element<'a, Message> {
    let display = if suffix.is_empty() { value.to_string() } else { format!("{value} {suffix}") };
    let key_for_change = key.clone();

    row![
        text(localized_label(label)).size(13).width(Length::Fixed(LABEL_WIDTH)),
        NumberInput::new(value as f32, min as f32, max as f32, 1.0, 0, 0.15, move |raw| {
            Message::Settings(SettingsMessage::Channels(ChannelsMessage::NumberChanged(
                key_for_change.clone(),
                raw.round() as u32,
            )))
        })
        .settings_style(),
        settings_value_badge(display),
    ]
    .spacing(16)
    .align_y(Alignment::Center)
    .into()
}

/// 构建或处理 `pick_row` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub(super) fn pick_row<'a>(
    label: &'static str,
    selected: &'static str,
    options: &'static [LabeledOption],
    key: &'static str,
) -> Element<'a, Message> {
    pick_row_with_key(label, selected, options, key.to_string())
}

pub(super) fn pick_row_with_key<'a>(
    label: &'static str,
    selected: &'static str,
    options: &'static [LabeledOption],
    key: String,
) -> Element<'a, Message> {
    let selected = options.iter().find(|option| option.value == selected).copied();
    let key_for_pick = key.clone();
    row![
        text(localized_label(label)).size(13).width(Length::Fixed(LABEL_WIDTH)),
        pick_list(options, selected, move |next| {
            Message::Settings(SettingsMessage::Channels(ChannelsMessage::ReceiveModeChanged(
                key_for_pick.clone(),
                next.value.to_string(),
            )))
        })
        .padding([10, 14])
        .text_size(13)
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style)
        .width(Length::Fixed(260.0)),
    ]
    .spacing(16)
    .align_y(Alignment::Center)
    .into()
}

pub(super) fn instance_title_row<'a>(
    title: String,
    channel: &'static str,
    index: usize,
) -> Element<'a, Message> {
    row![
        text(title).size(13).width(Length::Fill),
        button(text("删除").size(12))
            .padding([6, 10])
            .on_press(Message::Settings(SettingsMessage::Channels(
                ChannelsMessage::InstanceRemoved(channel.to_string(), index),
            )))
            .style(rounded_action_btn_style),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .into()
}

pub(super) fn add_instance_row<'a>(
    channel: &'static str,
    label: &'static str,
) -> Element<'a, Message> {
    row![
        container(text("")).width(Length::Fixed(LABEL_WIDTH)),
        button(text(label).size(12))
            .padding([7, 12])
            .on_press(Message::Settings(SettingsMessage::Channels(ChannelsMessage::InstanceAdded(
                channel.to_string()
            ),)))
            .style(rounded_action_btn_style),
    ]
    .spacing(16)
    .align_y(Alignment::Center)
    .into()
}

/// 构建或处理 `hint_row` 对应的界面片段与交互数据。
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
pub(super) fn hint_row<'a>(message: &'a str) -> Element<'a, Message> {
    row![
        container(text("")).width(Length::Fixed(LABEL_WIDTH)),
        text(message).size(12).style(settings_muted_text_style),
    ]
    .spacing(16)
    .align_y(Alignment::Center)
    .into()
}

/// 构建或处理 `panel` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub(super) fn panel<'a>(
    id: &'static str,
    title: &'static str,
    description: &'static str,
    enabled: bool,
    body: Element<'a, Message>,
) -> Element<'a, Message> {
    let header = row![
        column![text(title).size(14), text(description).size(12).style(settings_muted_text_style),]
            .spacing(4)
            .width(Length::Fill),
        checkbox(enabled)
            .label("启用")
            .on_toggle(move |next| {
                Message::Settings(SettingsMessage::Channels(ChannelsMessage::EnabledToggled(
                    id.to_string(),
                    next,
                )))
            })
            .style(settings_checkbox_style),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let content = if enabled {
        column![header, settings_divider(), body].spacing(12)
    } else {
        column![header, settings_divider(), hint_row("启用后显示该通道的详细配置表单。")]
            .spacing(12)
    };

    settings_panel(content).into()
}

/// 构建或处理 `enabled_channels` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub(super) fn enabled_channels(app: &App) -> Vec<&'static str> {
    let s = &app.channels_settings;
    let mut items = Vec::new();
    if s.cli {
        items.push("CLI");
    }
    if s.telegram.is_some() {
        items.push("Telegram");
    }
    if s.discord.is_some() {
        items.push("Discord");
    }
    if s.slack.is_some() {
        items.push("Slack");
    }
    if s.mattermost.is_some() {
        items.push("Mattermost");
    }
    if s.webhook.is_some() {
        items.push("Webhook");
    }
    if s.imessage.is_some() {
        items.push("iMessage");
    }
    if s.signal.is_some() {
        items.push("Signal");
    }
    if s.whatsapp.is_some() {
        items.push("WhatsApp");
    }
    if s.linq.is_some() {
        items.push("Linq");
    }
    if s.wati.is_some() {
        items.push("WATI");
    }
    if s.nextcloud_talk.is_some() {
        items.push("Nextcloud Talk");
    }
    #[cfg(not(target_arch = "wasm32"))]
    if s.email.is_some() {
        items.push("Email");
    }
    if s.irc.is_some() {
        items.push("IRC");
    }
    if s.lark.is_some() {
        items.push("Lark");
    }
    if s.feishu.is_some() || !s.feishu_instances.is_empty() {
        items.push("飞书");
    }
    if s.dingtalk.is_some() || !s.dingtalk_instances.is_empty() {
        items.push("钉钉");
    }
    if s.qq.is_some() {
        items.push("QQ");
    }
    if s.wecom.is_some() {
        items.push("企业微信");
    }
    if s.wechat.is_some() {
        items.push("微信");
    }
    if s.nostr.is_some() {
        items.push("Nostr");
    }
    if s.clawdtalk.is_some() {
        items.push("ClawdTalk");
    }
    items
}
