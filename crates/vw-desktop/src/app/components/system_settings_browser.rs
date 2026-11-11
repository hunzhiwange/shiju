//! 系统设置中 browser 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_checkbox_style,
    settings_control_with_hint, settings_divider, settings_flexible_row_with_help,
    settings_group_card, settings_group_heading_with_help, settings_page_heading_with_help,
    settings_row_with_flexible_control, settings_select, settings_text_editor, settings_text_input,
};
use crate::app::i18n::I18n;
use crate::app::message::settings::{BrowserMessage, SettingsMessage};
use crate::app::{App, Message};
use iced::widget::{button, checkbox, column, container, row, text};
use iced::{Element, Length};

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

fn browser_open_options() -> [LabeledOption; 3] {
    [
        LabeledOption { value: "default", label: "默认" },
        LabeledOption { value: "new_window", label: "新窗口" },
        LabeledOption { value: "new_tab", label: "新标签页" },
    ]
}

fn backend_options() -> [LabeledOption; 4] {
    [
        LabeledOption { value: "agent_browser", label: "代理浏览器（默认）" },
        LabeledOption { value: "native", label: "原生浏览器（WebDriver）" },
        LabeledOption { value: "computer_use", label: "系统控制（Sidecar）" },
        LabeledOption { value: "auto", label: "自动" },
    ]
}

fn field_row<'a>(
    app: &'a App,
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    settings_flexible_row_with_help(
        I18n::new(app.desktop_locale),
        &app.app_theme,
        label,
        description,
        360.0,
        control,
    )
    .into()
}

fn text_row<'a>(
    app: &'a App,
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

fn bool_row<'a>(
    app: &'a App,
    label: &'static str,
    description: &'static str,
    checked: bool,
    checkbox_label: &'static str,
    on_toggle: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    field_row(
        app,
        label,
        description,
        checkbox(checked).label(checkbox_label).on_toggle(on_toggle).style(settings_checkbox_style),
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
pub fn view(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.desktop_locale);
    column![
        settings_page_heading_with_help(
            i18n,
            &app.app_theme,
            i18n.__("浏览器配置"),
            i18n.__("配置允许站点、打开方式与浏览器自动化后端。")
        ),
        content(app),
    ]
    .spacing(16)
    .width(Length::Fill)
    .into()
}

pub(super) fn content(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let s = &app.browser_settings;

    let enabled_row = bool_row(
        app,
        i18n.__("启用"),
        i18n.__("控制浏览器与网页自动化相关工具。"),
        s.enabled,
        i18n.__("启用浏览器相关工具"),
        |value| Message::Settings(SettingsMessage::Browser(BrowserMessage::EnabledToggled(value))),
    );

    let allowed_domains_row = field_row(
        app,
        i18n.__("允许域名"),
        i18n.__("每行一个域名，或使用逗号分隔，例: example.com\n*.example.org"),
        settings_control_with_hint(
            settings_text_editor(&s.allowed_domains_editor)
                .placeholder(i18n.__("允许域名"))
                .on_action(|action| {
                    Message::Settings(SettingsMessage::Browser(
                        BrowserMessage::AllowedDomainsEditorAction(action),
                    ))
                })
                .height(Length::Fixed(96.0)),
            i18n.__("默认的 `*` 允许所有公网域名。列表为空时拒绝所有站点。"),
        ),
    );

    let browser_open_options = browser_open_options().map(|mut option| {
        option.label = i18n.__(option.label);
        option
    });
    let browser_open_selected =
        browser_open_options.iter().find(|opt| opt.value == s.browser_open).cloned();
    let browser_open_pick = settings_select(browser_open_options, browser_open_selected, |value| {
        Message::Settings(SettingsMessage::Browser(BrowserMessage::BrowserOpenChanged(
            value.value.to_string(),
        )))
    })
    .width(Length::Fill);

    let browser_open_row = field_row(
        app,
        i18n.__("打开方式"),
        i18n.__("决定 `browser_open` 默认尝试的窗口行为。"),
        settings_control_with_hint(
            browser_open_pick,
            i18n.__("当前运行时完整支持 default；其余选项会先写入配置。"),
        ),
    );

    let session_name_row = text_row(
        app,
        i18n.__("会话名称"),
        i18n.__("用于隔离浏览器 session 的可选标识。"),
        i18n.__("可选，会话隔离名称"),
        &s.session_name_input,
        |value| {
            Message::Settings(SettingsMessage::Browser(BrowserMessage::SessionNameChanged(value)))
        },
    );

    let backend_options = backend_options().map(|mut option| {
        option.label = i18n.__(option.label);
        option
    });
    let backend_selected = backend_options.iter().find(|opt| opt.value == s.backend).cloned();
    let backend_pick = settings_select(backend_options, backend_selected, |value| {
        Message::Settings(SettingsMessage::Browser(BrowserMessage::BackendChanged(
            value.value.to_string(),
        )))
    })
    .width(Length::Fill);

    let backend_control: Element<'_, Message> = match s.backend.as_str() {
        "native" => {
            settings_control_with_hint(backend_pick, i18n.__("原生浏览器需要可用的 WebDriver。"))
        }
        "computer_use" => {
            settings_control_with_hint(backend_pick, i18n.__("系统控制需要 sidecar。"))
        }
        _ => backend_pick.into(),
    };
    let mut install_button = button(text(i18n.__(if s.installing {
        "安装中…"
    } else if matches!(s.backend.as_str(), "agent_browser" | "auto") {
        "一键安装"
    } else {
        "手工安装"
    })))
    .padding([8, 14])
    .style(primary_action_btn_style);
    if !s.installing && matches!(s.backend.as_str(), "agent_browser" | "auto") {
        install_button = install_button
            .on_press(Message::Settings(SettingsMessage::Browser(BrowserMessage::Install)));
    }
    if !s.installing && !matches!(s.backend.as_str(), "agent_browser" | "auto") {
        install_button =
            install_button.on_press(Message::Settings(SettingsMessage::SystemHelpOpen(
                crate::app::components::system_settings::SystemTab::Browser,
            )));
    }
    let install_help = button(text(i18n.__("安装帮助")))
        .padding([8, 14])
        .style(rounded_action_btn_style)
        .on_press(Message::Settings(SettingsMessage::SystemHelpOpen(
            crate::app::components::system_settings::SystemTab::Browser,
        )));
    let backend_control =
        column![backend_control, row![install_button, install_help].spacing(8)].spacing(8);
    let backend_row = field_row(
        app,
        i18n.__("后端实现"),
        i18n.__("默认使用代理浏览器；原生浏览器需要可用 WebDriver；系统控制需要 sidecar。"),
        backend_control,
    );

    let native_section = column![
        settings_group_heading_with_help(
            i18n,
            &app.app_theme,
            i18n.__("Rust Native 后端"),
            i18n.__("当后端为 native 时，使用 WebDriver 驱动浏览器自动化。"),
        ),
        settings_group_card(
            column![
                bool_row(
                    app,
                    i18n.__("无头模式"),
                    i18n.__("决定是否在无界面模式下启动浏览器。"),
                    s.native_headless,
                    i18n.__("启用无头模式"),
                    |value| {
                        Message::Settings(SettingsMessage::Browser(
                            BrowserMessage::NativeHeadlessToggled(value),
                        ))
                    }
                ),
                settings_divider(),
                text_row(
                    app,
                    i18n.__("WebDriver 地址"),
                    i18n.__("连接本地或远程 WebDriver 服务。"),
                    "http://127.0.0.1:9515",
                    &s.native_webdriver_url,
                    |value| {
                        Message::Settings(SettingsMessage::Browser(
                            BrowserMessage::NativeWebdriverUrlChanged(value),
                        ))
                    },
                ),
                settings_divider(),
                text_row(
                    app,
                    i18n.__("Chrome 路径"),
                    i18n.__("可选，覆盖自动探测到的 Chrome/Chromium 路径。"),
                    i18n.__("可选，Chrome/Chromium 可执行文件路径"),
                    &s.native_chrome_path_input,
                    |value| {
                        Message::Settings(SettingsMessage::Browser(
                            BrowserMessage::NativeChromePathChanged(value),
                        ))
                    },
                ),
            ]
            .spacing(0)
        ),
    ]
    .spacing(16);

    let computer_use_section = column![
        settings_group_heading_with_help(
            i18n,
            &app.app_theme,
            i18n.__("系统级控制"),
            i18n.__("配置本地 sidecar 端点，用于鼠标、键盘、截图等系统级浏览器操作。"),
        ),
        settings_group_card(
            column![
                text_row(
                    app,
                    i18n.__("操作端点"),
                    i18n.__("接收系统级浏览器动作请求的 sidecar URL。"),
                    "http://127.0.0.1:8787/v1/actions",
                    &s.computer_use_endpoint,
                    |value| {
                        Message::Settings(SettingsMessage::Browser(
                            BrowserMessage::ComputerUseEndpointChanged(value),
                        ))
                    },
                ),
                settings_divider(),
                field_row(
                    app,
                    i18n.__("访问密钥"),
                    i18n.__("为 sidecar 请求附带可选的 Bearer Token。"),
                    settings_text_input(
                        i18n.__("可选，sidecar Bearer Token"),
                        &s.computer_use_api_key_input,
                    )
                    .secure(true)
                    .on_input(|value| {
                        Message::Settings(SettingsMessage::Browser(
                            BrowserMessage::ComputerUseApiKeyChanged(value),
                        ))
                    })
                    .width(Length::Fill),
                ),
                settings_divider(),
                text_row(
                    app,
                    i18n.__("超时时间（毫秒）"),
                    i18n.__("控制系统级浏览器动作的请求超时。"),
                    "15000",
                    &s.computer_use_timeout_ms_input,
                    |value| {
                        Message::Settings(SettingsMessage::Browser(
                            BrowserMessage::ComputerUseTimeoutMsChanged(value),
                        ))
                    },
                ),
                settings_divider(),
                settings_row_with_flexible_control(
                    i18n.__("允许远程端点"),
                    i18n.__("仅在明确可信时允许 sidecar 部署在公网。"),
                    360.0,
                    checkbox(s.computer_use_allow_remote_endpoint)
                        .label(i18n.__("允许公网 sidecar 端点"))
                        .on_toggle(|value| {
                            Message::Settings(SettingsMessage::Browser(
                                BrowserMessage::ComputerUseAllowRemoteEndpointToggled(value),
                            ))
                        })
                        .style(settings_checkbox_style),
                ),
                settings_divider(),
                text_row(
                    app,
                    i18n.__("窗口白名单"),
                    i18n.__("限定允许控制的窗口标题或进程名。"),
                    i18n.__("逗号或换行分隔窗口标题/进程名"),
                    &s.computer_use_window_allowlist_input,
                    |value| {
                        Message::Settings(SettingsMessage::Browser(
                            BrowserMessage::ComputerUseWindowAllowlistChanged(value),
                        ))
                    },
                ),
                settings_divider(),
                text_row(
                    app,
                    i18n.__("最大 X 坐标"),
                    i18n.__("可选，限制鼠标操作的最大 X 边界。"),
                    i18n.__("可选，限制最大 X 坐标"),
                    &s.computer_use_max_coordinate_x_input,
                    |value| {
                        Message::Settings(SettingsMessage::Browser(
                            BrowserMessage::ComputerUseMaxCoordinateXChanged(value),
                        ))
                    },
                ),
                settings_divider(),
                text_row(
                    app,
                    i18n.__("最大 Y 坐标"),
                    i18n.__("可选，限制鼠标操作的最大 Y 边界。"),
                    i18n.__("可选，限制最大 Y 坐标"),
                    &s.computer_use_max_coordinate_y_input,
                    |value| {
                        Message::Settings(SettingsMessage::Browser(
                            BrowserMessage::ComputerUseMaxCoordinateYChanged(value),
                        ))
                    },
                ),
            ]
            .spacing(0)
        ),
    ]
    .spacing(16);

    let mut content = column![
        settings_group_heading_with_help(
            i18n,
            &app.app_theme,
            i18n.__("基础行为"),
            i18n.__("域名范围、窗口行为与运行后端。")
        ),
        settings_group_card(
            column![
                enabled_row,
                settings_divider(),
                allowed_domains_row,
                settings_divider(),
                browser_open_row,
                settings_divider(),
                session_name_row,
                settings_divider(),
                backend_row,
            ]
            .spacing(0)
        ),
    ]
    .spacing(16)
    .width(Length::Fill);

    if matches!(s.backend.as_str(), "native") {
        content = content.push(native_section);
    }

    content = content.push(computer_use_section);

    container(content).width(Length::Fill).into()
}
#[cfg(test)]
#[path = "system_settings_browser_tests.rs"]
mod system_settings_browser_tests;

#[cfg(test)]
#[path = "system_settings_browser_ui_refresh_26_tests.rs"]
mod ui_refresh_26_tests;
