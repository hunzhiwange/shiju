use super::*;
use iced::widget::column;

pub(super) const CONTROL_WIDTH: f32 = 240.0;
pub(super) const LIST_EDITOR_MAX_HEIGHT: f32 = 176.0;

pub const SECURITY_TABS: &[SettingsTabDescriptor<SecuritySettingsTab>] = &[
    SettingsTabDescriptor::new(SecuritySettingsTab::Bash, "Bash", &["bash", "命令安全", "policy"]),
    SettingsTabDescriptor::new(
        SecuritySettingsTab::SandboxAndResources,
        "沙箱与资源",
        &["sandbox", "沙箱后端", "资源限制", "内存", "CPU", "子进程"],
    ),
    SettingsTabDescriptor::new(SecuritySettingsTab::Audit, "审计", &["audit", "审计日志", "签名"]),
    SettingsTabDescriptor::new(
        SecuritySettingsTab::OtpAndEstop,
        "OTP 与急停",
        &["OTP", "紧急停止", "E-Stop", "一次性密码"],
    ),
    SettingsTabDescriptor::new(
        SecuritySettingsTab::Syscall,
        "系统调用",
        &["syscall", "异常检测", "基线系统调用"],
    ),
    SettingsTabDescriptor::new(
        SecuritySettingsTab::Guard,
        "防护",
        &["Canary Token", "语义注入防护", "semantic guard"],
    ),
];

#[derive(Clone, PartialEq)]
pub(super) struct LabeledOption {
    pub(super) value: &'static str,
    pub(super) label: &'static str,
}

impl std::fmt::Display for LabeledOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

pub(super) fn field_row<'a>(
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    settings_row_with_flexible_control(label, description, CONTROL_WIDTH, control)
}

pub(super) fn field_row_with_help<'a>(
    app: &SecurityView<'_>,
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    settings_flexible_row_with_help(
        app,
        label,
        description,
        CONTROL_WIDTH,
        control,
    )
    .into()
}

pub(super) fn text_row<'a>(
    app: &SecurityView<'_>,
    label: &'static str,
    description: &'static str,
    placeholder: &'static str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    field_row_with_help(
        app,
        label,
        description,
        settings_text_input(placeholder, value).on_input(on_input).width(Length::Fill),
    )
}

pub(super) fn list_editor<'a>(
    app: &SecurityView<'_>,
    kind: SecurityListKind,
    label: &'static str,
    description: &'static str,
    placeholder: &'static str,
    items: &'a [String],
    draft: &'a str,
    pending_remove_index: Option<usize>,
) -> Element<'a, Message> {
    let locale = app.desktop_locale;
    let draft_changed = move |value| {
        Message::SecurityListDraftChanged(kind, value)
    };
    let add_message = Message::SecurityListAdd(kind);
    let add_button = {
        let button = button(text(security_text(locale, "添加")))
            .padding([8, 14])
            .height(38)
            .style(primary_action_btn_style);
        if draft.trim().is_empty() { button } else { button.on_press(add_message.clone()) }
    };

    let mut item_rows = column![].spacing(6);
    for (index, item) in items.iter().enumerate() {
        let mut item_content = column![
            row![
                text(item).size(13).width(Length::Fill),
                button(text(security_text(locale, "删除")).size(12))
                    .on_press(Message::SecurityListRemoveRequested(kind, index))
                    .padding([6, 10])
                    .style(rounded_action_btn_style),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        ]
        .spacing(6);
        if pending_remove_index == Some(index) {
            item_content = item_content.push(
                row![
                    text(security_text(locale, "确认删除该列表项？")).size(12).width(Length::Fill),
                    button(text(security_text(locale, "取消")).size(12))
                        .on_press(Message::SecurityListRemoveCanceled)
                        .padding([6, 10])
                        .style(rounded_action_btn_style),
                    button(text(security_text(locale, "确认删除")).size(12))
                        .on_press(Message::SecurityListRemoveConfirmed(kind, index))
                        .padding([6, 10])
                        .style(danger_action_btn_style),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            );
        }
        item_rows = item_rows.push(container(item_content).padding([6, 8]).width(Length::Fill));
    }

    let item_list = container(
        scrollable(container(item_rows).padding([0, 7]).width(Length::Fill))
            .height(Length::Shrink)
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4))),
    )
    .max_height(LIST_EDITOR_MAX_HEIGHT)
    .width(Length::Fill);

    column![
        settings_hover_help_label(
            app,
            label,
            settings_field_label(label),
            description,
        )
        .wrap(),
        row![
            settings_text_input(placeholder, draft)
                .on_input(draft_changed)
                .on_submit(add_message)
                .width(Length::Fill),
            add_button,
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        item_list,
    ]
    .spacing(10)
    .padding([14, 0])
    .width(Length::Fill)
    .into()
}

pub(super) fn bool_row<'a>(
    label: &'static str,
    description: &'static str,
    checked: bool,
    checkbox_label: &'static str,
    on_toggle: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    field_row(
        label,
        description,
        checkbox(checked).label(checkbox_label).on_toggle(on_toggle).style(settings_checkbox_style),
    )
}

pub(super) fn bash_checkbox<'a>(
    locale: DesktopLocale,
    checked: bool,
    active: bool,
    on_toggle: impl Fn(bool) -> Message + 'a,
) -> Checkbox<'a, Message> {
    let checkbox = checkbox(checked).label(if active {
        security_text(locale, "开启")
    } else {
        security_text(locale, "已保存，当前不生效")
    });
    let checkbox = if active {
        checkbox.on_toggle(move |value| on_toggle(value))
    } else {
        checkbox
    };
    checkbox.style(settings_checkbox_style)
}

pub(super) fn bash_bool_row<'a>(
    app: &SecurityView<'_>,
    label: &'static str,
    description: &'static str,
    checked: bool,
    active: bool,
    on_toggle: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    field_row_with_help(
        app,
        label,
        description,
        bash_checkbox(app.desktop_locale, checked, active, on_toggle),
    )
}

pub(super) fn slider_row<'a>(
    app: &SecurityView<'_>,
    label: &'static str,
    description: &'static str,
    slider: impl Into<Element<'a, Message>>,
    value: impl ToString,
) -> Element<'a, Message> {
    field_row_with_help(
        app,
        label,
        description,
        row![slider.into(), settings_value_badge(value)].spacing(12).align_y(Alignment::Center),
    )
}

pub(super) fn sandbox_enabled_options(locale: DesktopLocale) -> [LabeledOption; 3] {
    [
        LabeledOption { value: "auto", label: security_text(locale, "自动检测") },
        LabeledOption { value: "true", label: security_text(locale, "强制启用") },
        LabeledOption { value: "false", label: security_text(locale, "强制禁用") },
    ]
}

pub(super) fn sandbox_backend_options(locale: DesktopLocale, enabled: &str) -> Vec<LabeledOption> {
    match enabled {
        "false" => {
            vec![LabeledOption { value: "none", label: security_text(locale, "禁止沙箱") }]
        }
        "true" => vec![
            LabeledOption { value: "auto", label: security_text(locale, "自动选择") },
            LabeledOption { value: "landlock", label: "Landlock" },
            LabeledOption { value: "firejail", label: "Firejail" },
            LabeledOption { value: "bubblewrap", label: "Bubblewrap" },
            LabeledOption { value: "docker", label: "Docker" },
        ],
        _ => vec![
            LabeledOption { value: "auto", label: security_text(locale, "自动选择") },
            LabeledOption { value: "landlock", label: "Landlock" },
            LabeledOption { value: "firejail", label: "Firejail" },
            LabeledOption { value: "bubblewrap", label: "Bubblewrap" },
            LabeledOption { value: "docker", label: "Docker" },
            LabeledOption { value: "none", label: security_text(locale, "禁止沙箱") },
        ],
    }
}

pub(super) fn sandbox_backend_description(locale: DesktopLocale, backend: &str) -> &'static str {
    match backend {
        "landlock" => security_text(
            locale,
            "使用 Linux Landlock 约束文件系统与进程权限，适合支持 Landlock 的 Linux 环境。",
        ),
        "firejail" => {
            security_text(locale, "使用 Firejail 进行沙箱隔离，可额外传入命令行参数定制策略。")
        }
        "bubblewrap" => security_text(
            locale,
            "使用 Bubblewrap 构造轻量级容器隔离，适合更细粒度的命名空间控制。",
        ),
        "docker" => {
            security_text(locale, "使用 Docker 容器作为执行边界，适合已有容器环境的部署方式。")
        }
        "none" => security_text(locale, "不使用任何沙箱。仅在你明确接受更低隔离级别时使用。"),
        _ => security_text(
            locale,
            "由运行时自动选择可用的最优沙箱实现；无法确定时会回退到安全允许的路径。",
        ),
    }
}
