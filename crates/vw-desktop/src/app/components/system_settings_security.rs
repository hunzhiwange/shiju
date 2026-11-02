//! 系统安全设置界面组件
//!
//! 本模块提供安全配置的可视化界面，允许用户通过图形界面配置 VibeWindow 的各项安全参数。
//! 所有配置项都对应共享数据库中的 `security` 字段，并统一通过 Gateway 读写。
//!
//! # 主要功能
//!
//! - **沙箱配置**：启用/禁用沙箱、选择沙箱后端（landlock/firejail/bubblewrap/docker）
//! - **资源限制**：设置进程的最大内存、CPU 时长和子进程数量
//! - **审计日志**：配置审计日志的开启、路径、大小限制和签名
//! - **OTP 二次验证**：配置一次性密码验证机制，保护敏感操作
//! - **紧急停止**：配置紧急停止功能和恢复机制
//! - **系统调用异常检测**：监控系统调用行为，检测异常活动
//! - **安全防护**：配置 Canary Token 和语义注入防护
//!
//! # 使用方式
//!
//! 通过 [`view`] 函数创建安全设置界面的 UI 元素，该函数返回一个 Iced 框架的 `Element`。

use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, danger_action_btn_style, settings_checkbox_style, settings_divider,
    settings_error_banner, settings_muted_text_style, settings_page_intro,
    settings_pick_list_menu_style, settings_pick_list_style, settings_section_card,
    settings_section_content, settings_segment_button_style, settings_text_input_style,
    settings_value_badge,
};
use crate::app::components::system_settings_search::SettingsTabDescriptor;
use crate::app::i18n::settings_security::{Text, text as security_text};
use crate::app::state::{SecurityListKind, SecuritySettingsTab};
use crate::app::{App, Message, message};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{
    button, checkbox, column, container, pick_list, row, scrollable, slider, text, text_input,
};
use iced::{Alignment, Element, Length, Padding};
use vw_config_types::ui::DesktopLocale;

const LIST_EDITOR_MAX_HEIGHT: f32 = 176.0;

pub(super) const SECURITY_TABS: &[SettingsTabDescriptor<SecuritySettingsTab>] = &[
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
struct LabeledOption {
    value: &'static str,
    label: &'static str,
}

impl std::fmt::Display for LabeledOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

fn tab_button<'a>(
    label: &'static str,
    is_active: bool,
    message: Message,
) -> iced::widget::Button<'a, Message> {
    button(text(label)).on_press(message).padding([8, 14]).style(
        move |theme: &iced::Theme, status| settings_segment_button_style(theme, status, is_active),
    )
}

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

fn list_editor<'a>(
    locale: DesktopLocale,
    kind: SecurityListKind,
    label: &'static str,
    description: &'static str,
    placeholder: &'static str,
    items: &'a [String],
    draft: &'a str,
    pending_remove_index: Option<usize>,
) -> Element<'a, Message> {
    let draft_changed = move |value| {
        Message::Settings(message::SettingsMessage::SecurityListDraftChanged(kind, value))
    };
    let add_message = Message::Settings(message::SettingsMessage::SecurityListAdd(kind));
    let add_button = {
        let button = button(text(security_text(locale, Text::Add)))
            .padding([9, 14])
            .style(|theme, status| settings_segment_button_style(theme, status, false));
        if draft.trim().is_empty() { button } else { button.on_press(add_message.clone()) }
    };

    let mut item_rows = column![].spacing(6);
    for (index, item) in items.iter().enumerate() {
        let mut item_content = column![
            row![
                text(item).size(13).width(Length::Fill),
                button(text(security_text(locale, Text::Delete)).size(12))
                    .on_press(Message::Settings(
                        message::SettingsMessage::SecurityListRemoveRequested(kind, index),
                    ))
                    .padding([6, 10])
                    .style(|theme, status| { settings_segment_button_style(theme, status, false) }),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        ]
        .spacing(6);
        if pending_remove_index == Some(index) {
            item_content = item_content.push(
                row![
                    text(security_text(locale, Text::ConfirmItem)).size(12).width(Length::Fill),
                    button(text(security_text(locale, Text::Cancel)).size(12))
                        .on_press(Message::Settings(
                            message::SettingsMessage::SecurityListRemoveCanceled,
                        ))
                        .padding([6, 10])
                        .style(|theme, status| {
                            settings_segment_button_style(theme, status, false)
                        }),
                    button(text(security_text(locale, Text::ConfirmDelete)).size(12))
                        .on_press(Message::Settings(
                            message::SettingsMessage::SecurityListRemoveConfirmed(kind, index),
                        ))
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
        scrollable(container(item_rows).padding(Padding { right: 12.0, ..Padding::default() }))
            .height(Length::Shrink)
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4))),
    )
    .max_height(LIST_EDITOR_MAX_HEIGHT)
    .width(Length::Fill);

    column![
        column![text(label).size(13), text(description).size(11).style(settings_muted_text_style),]
            .spacing(4),
        row![
            text_input(placeholder, draft)
                .on_input(draft_changed)
                .on_submit(add_message)
                .padding([10, 12])
                .size(13)
                .style(settings_text_input_style)
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

fn bool_row<'a>(
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

fn bash_bool_row<'a>(
    locale: DesktopLocale,
    label: &'static str,
    description: &'static str,
    checked: bool,
    active: bool,
    on_toggle: impl Fn(bool) -> message::SettingsMessage + 'a,
) -> Element<'a, Message> {
    let checkbox = checkbox(checked).label(if active {
        security_text(locale, Text::On)
    } else {
        security_text(locale, Text::Inactive)
    });
    let checkbox = if active {
        checkbox.on_toggle(move |value| Message::Settings(on_toggle(value)))
    } else {
        checkbox
    };
    field_row(label, description, checkbox.style(settings_checkbox_style))
}

fn slider_row<'a>(
    label: &'static str,
    description: &'static str,
    slider: impl Into<Element<'a, Message>>,
    value: impl ToString,
) -> Element<'a, Message> {
    field_row(
        label,
        description,
        row![slider.into(), settings_value_badge(value)].spacing(12).align_y(Alignment::Center),
    )
}

fn sandbox_enabled_options(locale: DesktopLocale) -> [LabeledOption; 3] {
    [
        LabeledOption { value: "auto", label: security_text(locale, Text::AutoDetect) },
        LabeledOption { value: "true", label: security_text(locale, Text::ForceEnable) },
        LabeledOption { value: "false", label: security_text(locale, Text::ForceDisable) },
    ]
}

fn sandbox_backend_options(locale: DesktopLocale, enabled: &str) -> Vec<LabeledOption> {
    match enabled {
        "false" => {
            vec![LabeledOption { value: "none", label: security_text(locale, Text::NoSandbox) }]
        }
        "true" => vec![
            LabeledOption { value: "auto", label: security_text(locale, Text::AutoSelect) },
            LabeledOption { value: "landlock", label: "Landlock" },
            LabeledOption { value: "firejail", label: "Firejail" },
            LabeledOption { value: "bubblewrap", label: "Bubblewrap" },
            LabeledOption { value: "docker", label: "Docker" },
        ],
        _ => vec![
            LabeledOption { value: "auto", label: security_text(locale, Text::AutoSelect) },
            LabeledOption { value: "landlock", label: "Landlock" },
            LabeledOption { value: "firejail", label: "Firejail" },
            LabeledOption { value: "bubblewrap", label: "Bubblewrap" },
            LabeledOption { value: "docker", label: "Docker" },
            LabeledOption { value: "none", label: security_text(locale, Text::NoSandbox) },
        ],
    }
}

fn sandbox_backend_description(locale: DesktopLocale, backend: &str) -> &'static str {
    match backend {
        "landlock" => security_text(locale, Text::LandlockHelp),
        "firejail" => security_text(locale, Text::FirejailHelp),
        "bubblewrap" => security_text(locale, Text::BubblewrapHelp),
        "docker" => security_text(locale, Text::DockerBoundaryHelp),
        "none" => security_text(locale, Text::NoSandboxHelp),
        _ => security_text(locale, Text::AutoSandboxHelp),
    }
}

/// 创建安全设置界面的视图元素
///
/// 该函数构建一个包含所有安全配置选项的表单界面，包括文本输入框、滑块和复选框等控件。
/// 每个配置项都会触发相应的消息以更新应用状态。
///
/// # 参数
///
/// - `app`: 应用程序状态的不可变引用，从中读取当前的安全配置值
///
/// # 返回值
///
/// 返回一个 Iced `Element`，包含完整的安全设置界面。如果启用了帮助模态框，
/// 则返回包含模态框的堆叠布局。
///
/// # 界面布局
///
/// 界面从上到下依次包含：
/// 1. 标题栏（带帮助按钮）
/// 2. 沙箱配置区域
/// 3. 资源限制配置区域
/// 4. 审计日志配置区域
/// 5. OTP 二次验证配置区域
/// 6. 紧急停止配置区域
/// 7. 系统调用异常检测配置区域
/// 8. 安全防护配置区域
///
/// # 示例
///
/// ```ignore
/// let settings_view = system_settings_security::view(&app);
/// // 将 settings_view 添加到主界面中
/// ```
pub fn view(app: &App) -> Element<'_, Message> {
    let locale = app.desktop_locale;
    let s = &app.security_settings;
    let bash_security_row = bool_row(
        security_text(locale, Text::BashSecurity),
        security_text(locale, Text::BashHelp),
        s.bash.enabled,
        security_text(locale, Text::On),
        |v| Message::Settings(message::SettingsMessage::SecurityBashSecurityEnabledToggled(v)),
    );
    use message::settings::{BashPolicyCheck as P, BashValidatorCheck as V};
    let policy_active = s.bash.enabled;
    let policy_items_active = policy_active && s.bash.policy.enabled;
    let validator_active = s.bash.enabled;
    let validator_items_active = validator_active && s.bash.validators.enabled;
    let policy_group_row = bash_bool_row(
        locale,
        security_text(locale, Text::Policy),
        security_text(locale, Text::PolicyHelp),
        s.bash.policy.enabled,
        policy_active,
        |v| message::SettingsMessage::SecurityBashPolicyCheckToggled(P::Enabled, v),
    );
    let policy_rows = column![
        policy_group_row,
        settings_divider(),
        bash_bool_row(
            locale,
            security_text(locale, Text::ReadOnlyConstraint),
            security_text(locale, Text::ReadOnlyConstraintHelp),
            s.bash.policy.read_only,
            policy_items_active,
            |v| message::SettingsMessage::SecurityBashPolicyCheckToggled(P::ReadOnly, v)
        ),
        settings_divider(),
        bash_bool_row(
            locale,
            security_text(locale, Text::DangerousShell),
            security_text(locale, Text::DangerousShellHelp),
            s.bash.policy.unsafe_shell_patterns,
            policy_items_active,
            |v| message::SettingsMessage::SecurityBashPolicyCheckToggled(P::UnsafeShellPatterns, v)
        ),
        settings_divider(),
        bash_bool_row(
            locale,
            security_text(locale, Text::CommandRules),
            security_text(locale, Text::CommandRulesHelp),
            s.bash.policy.command_rules,
            policy_items_active,
            |v| message::SettingsMessage::SecurityBashPolicyCheckToggled(P::CommandRules, v)
        ),
        settings_divider(),
        bash_bool_row(
            locale,
            security_text(locale, Text::WorkspacePaths),
            security_text(locale, Text::WorkspacePathsHelp),
            s.bash.policy.workspace_paths,
            policy_items_active,
            |v| message::SettingsMessage::SecurityBashPolicyCheckToggled(P::WorkspacePaths, v)
        ),
        settings_divider(),
        bash_bool_row(
            locale,
            security_text(locale, Text::ForbiddenPaths),
            security_text(locale, Text::ForbiddenPathsHelp),
            s.bash.policy.forbidden_paths,
            policy_items_active,
            |v| message::SettingsMessage::SecurityBashPolicyCheckToggled(P::ForbiddenPaths, v)
        ),
        settings_divider(),
        bash_bool_row(
            locale,
            security_text(locale, Text::RiskLevel),
            security_text(locale, Text::RiskLevelHelp),
            s.bash.policy.risk,
            policy_items_active,
            |v| message::SettingsMessage::SecurityBashPolicyCheckToggled(P::Risk, v)
        ),
        settings_divider(),
        bash_bool_row(
            locale,
            security_text(locale, Text::Redirects),
            security_text(locale, Text::RedirectsHelp),
            s.bash.policy.redirects,
            policy_items_active,
            |v| message::SettingsMessage::SecurityBashPolicyCheckToggled(P::Redirects, v)
        ),
    ]
    .spacing(0);
    let validator_group_row = bash_bool_row(
        locale,
        security_text(locale, Text::ShellSyntax),
        security_text(locale, Text::ShellSyntaxHelp),
        s.bash.validators.enabled,
        validator_active,
        |v| message::SettingsMessage::SecurityBashValidatorCheckToggled(V::Enabled, v),
    );
    macro_rules! validator_row {
        ($label:expr, $description:expr, $field:ident, $variant:ident) => {
            bash_bool_row(
                locale,
                $label,
                $description,
                s.bash.validators.$field,
                validator_items_active,
                |v| message::SettingsMessage::SecurityBashValidatorCheckToggled(V::$variant, v),
            )
        };
    }
    let validator_rows = column![
        validator_group_row,
        settings_divider(),
        validator_row!(
            security_text(locale, Text::EmptyCommand),
            security_text(locale, Text::EmptyCommandHelp),
            empty_command,
            EmptyCommand
        ),
        settings_divider(),
        validator_row!("Heredoc", security_text(locale, Text::HeredocHelp), heredoc, Heredoc),
        settings_divider(),
        validator_row!(
            "Git commit",
            security_text(locale, Text::GitCommitHelp),
            git_commit,
            GitCommit
        ),
        settings_divider(),
        validator_row!("jq", security_text(locale, Text::JqHelp), jq, Jq),
        settings_divider(),
        validator_row!(
            security_text(locale, Text::Metacharacters),
            security_text(locale, Text::MetacharactersHelp),
            metacharacters,
            Metacharacters
        ),
        settings_divider(),
        validator_row!(
            security_text(locale, Text::DangerousVars),
            security_text(locale, Text::DangerousVarsHelp),
            dangerous_vars,
            DangerousVars
        ),
        settings_divider(),
        validator_row!(
            security_text(locale, Text::Substitution),
            security_text(locale, Text::SubstitutionHelp),
            substitution,
            Substitution
        ),
        settings_divider(),
        validator_row!(
            security_text(locale, Text::SyntaxRedirection),
            security_text(locale, Text::SyntaxRedirectionHelp),
            redirection,
            Redirection
        ),
        settings_divider(),
        validator_row!(
            security_text(locale, Text::NewlineInjection),
            security_text(locale, Text::NewlineInjectionHelp),
            newline_injection,
            NewlineInjection
        ),
        settings_divider(),
        validator_row!(
            security_text(locale, Text::IfsInjection),
            security_text(locale, Text::IfsInjectionHelp),
            ifs_injection,
            IfsInjection
        ),
        settings_divider(),
        validator_row!(
            security_text(locale, Text::ProcessEnvironment),
            security_text(locale, Text::ProcessEnvironmentHelp),
            proc_environ,
            ProcEnviron
        ),
        settings_divider(),
        validator_row!(
            security_text(locale, Text::Malformed),
            security_text(locale, Text::MalformedHelp),
            malformed,
            Malformed
        ),
        settings_divider(),
        validator_row!(
            security_text(locale, Text::ObfuscatedFlags),
            security_text(locale, Text::ObfuscatedFlagsHelp),
            obfuscated_flags,
            ObfuscatedFlags
        ),
        settings_divider(),
        validator_row!(
            security_text(locale, Text::Backslash),
            security_text(locale, Text::BackslashHelp),
            backslash,
            Backslash
        ),
        settings_divider(),
        validator_row!(
            security_text(locale, Text::BraceExpansion),
            security_text(locale, Text::BraceExpansionHelp),
            brace_expansion,
            BraceExpansion
        ),
        settings_divider(),
        validator_row!(
            security_text(locale, Text::UnicodeWhitespace),
            security_text(locale, Text::UnicodeWhitespaceHelp),
            unicode_whitespace,
            UnicodeWhitespace
        ),
        settings_divider(),
        validator_row!(
            security_text(locale, Text::HashComment),
            security_text(locale, Text::HashCommentHelp),
            hash_comment,
            HashComment
        ),
        settings_divider(),
        validator_row!(
            security_text(locale, Text::CommentQuote),
            security_text(locale, Text::CommentQuoteHelp),
            comment_quote_desync,
            CommentQuoteDesync
        ),
        settings_divider(),
        validator_row!(
            security_text(locale, Text::QuotedNewline),
            security_text(locale, Text::QuotedNewlineHelp),
            quoted_newline,
            QuotedNewline
        ),
        settings_divider(),
        validator_row!(
            security_text(locale, Text::ZshDangerous),
            security_text(locale, Text::ZshDangerousHelp),
            zsh_dangerous,
            ZshDangerous
        ),
        settings_divider(),
        validator_row!(
            security_text(locale, Text::ControlChars),
            security_text(locale, Text::ControlCharsHelp),
            control_chars,
            ControlChars
        ),
    ]
    .spacing(0);
    let enabled_options = sandbox_enabled_options(locale);
    let enabled_selected =
        enabled_options.iter().find(|option| option.value == s.sandbox_enabled_input).cloned();
    let backend_options = sandbox_backend_options(locale, &s.sandbox_enabled_input);
    let backend_selected =
        backend_options.iter().find(|option| option.value == s.sandbox_backend_input).cloned();
    let help_btn = crate::app::components::system_settings_common::icon_btn(
        crate::app::assets::Icon::QuestionCircle,
        crate::app::i18n::settings_runtime::text(
            locale,
            crate::app::i18n::settings_runtime::Text::Help,
        ),
        Some(Message::Settings(message::SettingsMessage::SecurityHelpOpen)),
    );
    let sandbox_enabled_row = field_row(
        security_text(locale, Text::SandboxEnabled),
        security_text(locale, Text::SandboxEnabledHelp),
        pick_list(enabled_options, enabled_selected, |value| {
            Message::Settings(message::SettingsMessage::SecuritySandboxEnabledChanged(
                value.value.to_string(),
            ))
        })
        .padding([10, 14])
        .text_size(13)
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style)
        .width(Length::Fixed(280.0)),
    );

    let sandbox_backend_row = field_row(
        security_text(locale, Text::SandboxBackend),
        security_text(locale, Text::SandboxBackendHelp),
        pick_list(backend_options, backend_selected, |value| {
            Message::Settings(message::SettingsMessage::SecuritySandboxBackendChanged(
                value.value.to_string(),
            ))
        })
        .padding([10, 14])
        .text_size(13)
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style)
        .width(Length::Fixed(280.0)),
    );

    let sandbox_backend_card = settings_section_card(
        security_text(locale, Text::BackendDescription),
        sandbox_backend_description(locale, &s.sandbox_backend_input),
    );

    let firejail_args_row = list_editor(
        locale,
        SecurityListKind::SandboxFirejailArgs,
        security_text(locale, Text::FirejailArgs),
        security_text(locale, Text::FirejailArgsHelp),
        security_text(locale, Text::FirejailExample),
        &s.sandbox_firejail_args,
        s.list_editor.draft(SecurityListKind::SandboxFirejailArgs),
        s.list_editor.pending_remove_index(SecurityListKind::SandboxFirejailArgs),
    );

    let max_memory_row = slider_row(
        security_text(locale, Text::MaxMemory),
        security_text(locale, Text::MaxMemoryHelp),
        slider(32.0..=65_536.0, s.resources_max_memory_mb as f32, |v| {
            Message::Settings(message::SettingsMessage::SecurityResourcesMaxMemoryMbChanged(
                v.round() as u32,
            ))
        })
        .width(Length::Fill),
        format!("{} MB", s.resources_max_memory_mb),
    );

    let max_cpu_row = slider_row(
        security_text(locale, Text::MaxCpuTime),
        security_text(locale, Text::MaxCpuTimeHelp),
        slider(1.0..=3600.0, s.resources_max_cpu_time_seconds as f32, |v| {
            Message::Settings(message::SettingsMessage::SecurityResourcesMaxCpuTimeSecondsChanged(
                v.round() as u64,
            ))
        })
        .width(Length::Fill),
        format!("{} s", s.resources_max_cpu_time_seconds),
    );

    let max_subprocesses_row = slider_row(
        security_text(locale, Text::MaxSubprocesses),
        security_text(locale, Text::MaxSubprocessesHelp),
        slider(1.0..=10_000.0, s.resources_max_subprocesses as f32, |v| {
            Message::Settings(message::SettingsMessage::SecurityResourcesMaxSubprocessesChanged(
                v.round() as u32,
            ))
        })
        .width(Length::Fill),
        s.resources_max_subprocesses,
    );

    let memory_monitoring_row = bool_row(
        security_text(locale, Text::MemoryMonitoring),
        security_text(locale, Text::MemoryMonitoringHelp),
        s.resources_memory_monitoring,
        security_text(locale, Text::On),
        |v| {
            Message::Settings(message::SettingsMessage::SecurityResourcesMemoryMonitoringToggled(v))
        },
    );

    let audit_enabled_row = bool_row(
        security_text(locale, Text::AuditLog),
        security_text(locale, Text::AuditLogHelp),
        s.audit_enabled,
        security_text(locale, Text::On),
        |v| Message::Settings(message::SettingsMessage::SecurityAuditEnabledToggled(v)),
    );

    let audit_log_path_row = text_row(
        security_text(locale, Text::AuditPath),
        security_text(locale, Text::AuditPathHelp),
        "audit.log",
        &s.audit_log_path,
        |v| Message::Settings(message::SettingsMessage::SecurityAuditLogPathChanged(v)),
    );

    let audit_size_row = slider_row(
        security_text(locale, Text::AuditMaxSize),
        security_text(locale, Text::AuditMaxSizeHelp),
        slider(1.0..=10_000.0, s.audit_max_size_mb as f32, |v| {
            Message::Settings(message::SettingsMessage::SecurityAuditMaxSizeMbChanged(
                v.round() as u32
            ))
        })
        .width(Length::Fill),
        format!("{} MB", s.audit_max_size_mb),
    );

    let audit_sign_row = bool_row(
        security_text(locale, Text::AuditSignature),
        security_text(locale, Text::AuditSignatureHelp),
        s.audit_sign_events,
        security_text(locale, Text::SignEvents),
        |v| Message::Settings(message::SettingsMessage::SecurityAuditSignEventsToggled(v)),
    );

    let otp_enabled_row = bool_row(
        "OTP",
        security_text(locale, Text::OtpHelp),
        s.otp_enabled,
        security_text(locale, Text::On),
        |v| Message::Settings(message::SettingsMessage::SecurityOtpEnabledToggled(v)),
    );

    let otp_method_row = text_row(
        security_text(locale, Text::OtpMethod),
        security_text(locale, Text::OtpMethodHelp),
        "totp | pairing | cli-prompt",
        &s.otp_method_input,
        |v| Message::Settings(message::SettingsMessage::SecurityOtpMethodChanged(v)),
    );

    let otp_ttl_row = slider_row(
        "OTP TTL",
        security_text(locale, Text::OtpTtlHelp),
        slider(1.0..=600.0, s.otp_token_ttl_secs as f32, |v: f32| {
            Message::Settings(message::SettingsMessage::SecurityOtpTokenTtlSecsChanged(
                v.round() as u64
            ))
        })
        .width(Length::Fill),
        format!("{} s", s.otp_token_ttl_secs),
    );

    let otp_cache_row = slider_row(
        security_text(locale, Text::OtpCache),
        security_text(locale, Text::OtpCacheHelp),
        slider(1.0..=3600.0, s.otp_cache_valid_secs as f32, |v: f32| {
            Message::Settings(message::SettingsMessage::SecurityOtpCacheValidSecsChanged(
                v.round() as u64
            ))
        })
        .width(Length::Fill),
        format!("{} s", s.otp_cache_valid_secs),
    );

    let otp_actions_row = list_editor(
        locale,
        SecurityListKind::OtpGatedActions,
        security_text(locale, Text::OtpActions),
        security_text(locale, Text::OtpActionsHelp),
        security_text(locale, Text::ShellExample),
        &s.otp_gated_actions,
        s.list_editor.draft(SecurityListKind::OtpGatedActions),
        s.list_editor.pending_remove_index(SecurityListKind::OtpGatedActions),
    );

    let otp_domains_row = list_editor(
        locale,
        SecurityListKind::OtpGatedDomains,
        security_text(locale, Text::OtpDomains),
        security_text(locale, Text::OtpDomainsHelp),
        security_text(locale, Text::DomainExample),
        &s.otp_gated_domains,
        s.list_editor.draft(SecurityListKind::OtpGatedDomains),
        s.list_editor.pending_remove_index(SecurityListKind::OtpGatedDomains),
    );

    let otp_domain_categories_row = list_editor(
        locale,
        SecurityListKind::OtpGatedDomainCategories,
        security_text(locale, Text::OtpCategories),
        security_text(locale, Text::OtpCategoriesHelp),
        security_text(locale, Text::PaymentsExample),
        &s.otp_gated_domain_categories,
        s.list_editor.draft(SecurityListKind::OtpGatedDomainCategories),
        s.list_editor.pending_remove_index(SecurityListKind::OtpGatedDomainCategories),
    );

    let estop_enabled_row = bool_row(
        security_text(locale, Text::Estop),
        security_text(locale, Text::EstopHelp),
        s.estop_enabled,
        security_text(locale, Text::On),
        |v| Message::Settings(message::SettingsMessage::SecurityEstopEnabledToggled(v)),
    );

    let estop_state_file_row = text_row(
        security_text(locale, Text::EstopFile),
        security_text(locale, Text::EstopFileHelp),
        vw_config_types::paths::ESTOP_STATE_FILE_PATH,
        &s.estop_state_file,
        |v| Message::Settings(message::SettingsMessage::SecurityEstopStateFileChanged(v)),
    );

    let estop_require_otp_row = bool_row(
        security_text(locale, Text::ResumeOtp),
        security_text(locale, Text::ResumeOtpHelp),
        s.estop_require_otp_to_resume,
        security_text(locale, Text::On),
        |v| Message::Settings(message::SettingsMessage::SecurityEstopRequireOtpToResumeToggled(v)),
    );

    let syscall_enabled_row = bool_row(
        security_text(locale, Text::SyscallAnomaly),
        security_text(locale, Text::SyscallAnomalyHelp),
        s.syscall_anomaly_enabled,
        security_text(locale, Text::On),
        |v| Message::Settings(message::SettingsMessage::SecuritySyscallAnomalyEnabledToggled(v)),
    );

    let syscall_strict_row = bool_row(
        security_text(locale, Text::StrictMode),
        security_text(locale, Text::StrictModeHelp),
        s.syscall_anomaly_strict_mode,
        security_text(locale, Text::On),
        |v| Message::Settings(message::SettingsMessage::SecuritySyscallAnomalyStrictModeToggled(v)),
    );

    let syscall_alert_unknown_row = bool_row(
        security_text(locale, Text::UnknownSyscall),
        security_text(locale, Text::UnknownSyscallHelp),
        s.syscall_anomaly_alert_on_unknown_syscall,
        security_text(locale, Text::On),
        |v| {
            Message::Settings(
                message::SettingsMessage::SecuritySyscallAnomalyAlertOnUnknownSyscallToggled(v),
            )
        },
    );

    let denied_row = slider_row(
        security_text(locale, Text::DenyThreshold),
        security_text(locale, Text::DenyThresholdHelp),
        slider(1.0..=10_000.0, s.syscall_anomaly_max_denied_events_per_minute as f32, |v: f32| {
            Message::Settings(
                message::SettingsMessage::SecuritySyscallAnomalyMaxDeniedEventsPerMinuteChanged(
                    v.round() as u32,
                ),
            )
        })
        .width(Length::Fill),
        s.syscall_anomaly_max_denied_events_per_minute,
    );

    let total_row = slider_row(
        security_text(locale, Text::EventThreshold),
        security_text(locale, Text::EventThresholdHelp),
        slider(1.0..=100_000.0, s.syscall_anomaly_max_total_events_per_minute as f32, |v: f32| {
            Message::Settings(
                message::SettingsMessage::SecuritySyscallAnomalyMaxTotalEventsPerMinuteChanged(
                    v.round() as u32,
                ),
            )
        })
        .width(Length::Fill),
        s.syscall_anomaly_max_total_events_per_minute,
    );

    let alerts_row = slider_row(
        security_text(locale, Text::AlertThreshold),
        security_text(locale, Text::AlertThresholdHelp),
        slider(1.0..=10_000.0, s.syscall_anomaly_max_alerts_per_minute as f32, |v: f32| {
            Message::Settings(
                message::SettingsMessage::SecuritySyscallAnomalyMaxAlertsPerMinuteChanged(
                    v.round() as u32,
                ),
            )
        })
        .width(Length::Fill),
        s.syscall_anomaly_max_alerts_per_minute,
    );

    let cooldown_row = slider_row(
        security_text(locale, Text::AlertCooldown),
        security_text(locale, Text::AlertCooldownHelp),
        slider(1.0..=3600.0, s.syscall_anomaly_alert_cooldown_secs as f32, |v: f32| {
            Message::Settings(
                message::SettingsMessage::SecuritySyscallAnomalyAlertCooldownSecsChanged(
                    v.round() as u64
                ),
            )
        })
        .width(Length::Fill),
        format!("{} s", s.syscall_anomaly_alert_cooldown_secs),
    );

    let syscall_log_path_row = text_row(
        security_text(locale, Text::SyscallLog),
        security_text(locale, Text::SyscallLogHelp),
        "syscall-anomalies.log",
        &s.syscall_anomaly_log_path,
        |v| Message::Settings(message::SettingsMessage::SecuritySyscallAnomalyLogPathChanged(v)),
    );

    let baseline_syscalls_row = list_editor(
        locale,
        SecurityListKind::SyscallAnomalyBaselineSyscalls,
        security_text(locale, Text::SyscallBaseline),
        security_text(locale, Text::SyscallBaselineHelp),
        security_text(locale, Text::OpenatExample),
        &s.syscall_anomaly_baseline_syscalls,
        s.list_editor.draft(SecurityListKind::SyscallAnomalyBaselineSyscalls),
        s.list_editor.pending_remove_index(SecurityListKind::SyscallAnomalyBaselineSyscalls),
    );

    let canary_row = bool_row(
        "Canary Token",
        security_text(locale, Text::CanaryHelp),
        s.canary_tokens,
        security_text(locale, Text::On),
        |v| Message::Settings(message::SettingsMessage::SecurityCanaryTokensToggled(v)),
    );

    let semantic_guard_row = bool_row(
        security_text(locale, Text::SemanticGuard),
        security_text(locale, Text::SemanticGuardHelp),
        s.semantic_guard,
        security_text(locale, Text::On),
        |v| Message::Settings(message::SettingsMessage::SecuritySemanticGuardToggled(v)),
    );

    let semantic_collection_row = text_row(
        security_text(locale, Text::SemanticCollection),
        security_text(locale, Text::SemanticCollectionHelp),
        "semantic_guard",
        &s.semantic_guard_collection,
        |v| Message::Settings(message::SettingsMessage::SecuritySemanticGuardCollectionChanged(v)),
    );

    let semantic_threshold_row = slider_row(
        security_text(locale, Text::SemanticThreshold),
        security_text(locale, Text::SemanticThresholdHelp),
        slider(0.0..=1.0, s.semantic_guard_threshold as f32, |v| {
            Message::Settings(message::SettingsMessage::SecuritySemanticGuardThresholdChanged(
                v as f64,
            ))
        })
        .width(Length::Fill),
        format!("{:.2}", s.semantic_guard_threshold),
    );

    let mut sandbox_rows = column![sandbox_enabled_row, settings_divider(), sandbox_backend_row]
        .spacing(0)
        .width(Length::Fill);
    if s.sandbox_backend_input == "firejail" {
        sandbox_rows = sandbox_rows.push(settings_divider()).push(firejail_args_row);
    }

    let tabs = SECURITY_TABS.iter().fold(
        row![].spacing(8).align_y(Alignment::Center),
        |row, descriptor| {
            row.push(tab_button(
                security_tab_label(locale, descriptor.tab),
                s.active_tab == descriptor.tab,
                Message::Settings(message::SettingsMessage::SecurityTabSelected(descriptor.tab)),
            ))
        },
    );

    let content: Element<'_, Message> = match s.active_tab {
        SecuritySettingsTab::Bash => column![
            settings_section_card(
                security_text(locale, Text::BashSecurity),
                security_text(locale, Text::SwitchPriorityHelp)
            ),
            settings_section_content(bash_security_row),
            settings_section_card(
                security_text(locale, Text::Policy),
                security_text(locale, Text::PolicyIntro)
            ),
            settings_section_content(policy_rows),
            settings_section_card(
                security_text(locale, Text::ShellSyntax),
                security_text(locale, Text::SyntaxIntro)
            ),
            settings_section_content(validator_rows),
        ]
        .spacing(16)
        .into(),
        SecuritySettingsTab::SandboxAndResources => column![
            settings_section_card(
                security_text(locale, Text::Sandbox),
                security_text(locale, Text::SandboxIntro)
            ),
            settings_section_content(sandbox_rows),
            sandbox_backend_card,
            settings_section_card(
                security_text(locale, Text::Resources),
                security_text(locale, Text::ResourcesIntro)
            ),
            settings_section_content(
                column![
                    max_memory_row,
                    settings_divider(),
                    max_cpu_row,
                    settings_divider(),
                    max_subprocesses_row,
                    settings_divider(),
                    memory_monitoring_row,
                ]
                .spacing(0)
            ),
        ]
        .spacing(16)
        .into(),
        SecuritySettingsTab::Audit => column![
            settings_section_card(
                security_text(locale, Text::AuditLog),
                security_text(locale, Text::AuditIntro)
            ),
            settings_section_content(
                column![
                    audit_enabled_row,
                    settings_divider(),
                    audit_log_path_row,
                    settings_divider(),
                    audit_size_row,
                    settings_divider(),
                    audit_sign_row,
                ]
                .spacing(0)
            ),
        ]
        .spacing(16)
        .into(),
        SecuritySettingsTab::OtpAndEstop => column![
            settings_section_card("OTP", security_text(locale, Text::OtpIntro)),
            settings_section_content(
                column![
                    otp_enabled_row,
                    settings_divider(),
                    otp_method_row,
                    settings_divider(),
                    otp_ttl_row,
                    settings_divider(),
                    otp_cache_row,
                    settings_divider(),
                    otp_actions_row,
                    settings_divider(),
                    otp_domains_row,
                    settings_divider(),
                    otp_domain_categories_row,
                ]
                .spacing(0)
            ),
            settings_section_card(
                security_text(locale, Text::Estop),
                security_text(locale, Text::EstopIntro)
            ),
            settings_section_content(
                column![
                    estop_enabled_row,
                    settings_divider(),
                    estop_state_file_row,
                    settings_divider(),
                    estop_require_otp_row,
                ]
                .spacing(0)
            ),
        ]
        .spacing(16)
        .into(),
        SecuritySettingsTab::Syscall => column![
            settings_section_card(
                security_text(locale, Text::SyscallAnomaly),
                security_text(locale, Text::SyscallIntro)
            ),
            settings_section_content(
                column![
                    syscall_enabled_row,
                    settings_divider(),
                    syscall_strict_row,
                    settings_divider(),
                    syscall_alert_unknown_row,
                    settings_divider(),
                    denied_row,
                    settings_divider(),
                    total_row,
                    settings_divider(),
                    alerts_row,
                    settings_divider(),
                    cooldown_row,
                    settings_divider(),
                    syscall_log_path_row,
                    settings_divider(),
                    baseline_syscalls_row,
                ]
                .spacing(0)
            ),
        ]
        .spacing(16)
        .into(),
        SecuritySettingsTab::Guard => column![
            settings_section_card(
                security_text(locale, Text::Guard),
                security_text(locale, Text::GuardIntro)
            ),
            settings_section_content(
                column![
                    canary_row,
                    settings_divider(),
                    semantic_guard_row,
                    settings_divider(),
                    semantic_collection_row,
                    settings_divider(),
                    semantic_threshold_row,
                ]
                .spacing(0)
            ),
        ]
        .spacing(16)
        .into(),
    };

    let mut col = column![
        row![
            settings_page_intro(
                security_text(locale, Text::SecurityConfig),
                security_text(locale, Text::SecurityIntro)
            ),
            container(text(" ")).width(Length::Fill),
            help_btn,
        ]
        .align_y(Alignment::Start),
        tabs,
    ]
    .spacing(16)
    .width(Length::Fill);

    if let Some(error) = s.save_error.as_deref() {
        col = col.push(settings_error_banner(
            crate::app::i18n::settings_runtime::validation_error(locale, error),
        ));
    }

    col.push(content).into()
}

pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let locale = app.desktop_locale;
    let s = &app.security_settings;
    if !s.show_help_modal {
        return dialog;
    }

    let help_text = crate::app::i18n::settings_security::help(locale);

    crate::app::components::system_settings_common::with_settings_help_modal(
        app,
        dialog,
        security_text(locale, Text::SecurityHelpTitle),
        help_text,
        Message::Settings(message::SettingsMessage::SecurityHelpClose),
    )
}
#[cfg(test)]
#[path = "system_settings_security_tests.rs"]
mod system_settings_security_tests;

pub(super) fn security_tab_label(locale: DesktopLocale, tab: SecuritySettingsTab) -> &'static str {
    match tab {
        SecuritySettingsTab::Bash => "Bash",
        SecuritySettingsTab::SandboxAndResources => security_text(locale, Text::SandboxResources),
        SecuritySettingsTab::Audit => security_text(locale, Text::Audit),
        SecuritySettingsTab::OtpAndEstop => security_text(locale, Text::OtpEstop),
        SecuritySettingsTab::Syscall => security_text(locale, Text::Syscalls),
        SecuritySettingsTab::Guard => security_text(locale, Text::Guard),
    }
}
