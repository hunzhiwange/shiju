//! 桌面应用顶部栏的按钮、菜单与窗口交互控件。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use super::widgets::{icon_svg, icon_toggle_button_opt, menu_container, menu_item_icon_btn};
use crate::app::assets::{self, Icon};
use crate::app::components::overlays::BelowOverlay;
use crate::app::message::view::MenuType;
use crate::app::state::{ExternalOpenApp, RuntimePlatform};
use crate::app::{App, Message, Screen, message};
use iced::widget::svg::Svg;
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{Space, button, column, container, image, row, text};
use iced::{Color, Element, Length, Theme};

pub(super) const OPEN_EXTERNAL_TARGETS: [ExternalOpenApp; 16] = [
    ExternalOpenApp::Finder,
    ExternalOpenApp::Trae,
    ExternalOpenApp::Windsurf,
    ExternalOpenApp::Kiro,
    ExternalOpenApp::Cursor,
    ExternalOpenApp::VSCode,
    ExternalOpenApp::Zed,
    ExternalOpenApp::TextMate,
    ExternalOpenApp::Antigravity,
    ExternalOpenApp::Terminal,
    ExternalOpenApp::ITerm2,
    ExternalOpenApp::Ghostty,
    ExternalOpenApp::Xcode,
    ExternalOpenApp::AndroidStudio,
    ExternalOpenApp::PowerShell,
    ExternalOpenApp::SublimeText,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ExternalAppLogo {
    Svg(Icon),
    Image(Icon),
}

pub(super) fn file_manager_label(platform: Option<RuntimePlatform>) -> &'static str {
    platform.map(RuntimePlatform::file_manager_label).unwrap_or("File Manager")
}

pub(super) fn external_app_label(
    target: ExternalOpenApp,
    platform: Option<RuntimePlatform>,
) -> &'static str {
    match target {
        ExternalOpenApp::Finder => file_manager_label(platform),
        _ => target.label(),
    }
}

pub(super) fn external_app_logo(
    target: ExternalOpenApp,
    platform: Option<RuntimePlatform>,
) -> ExternalAppLogo {
    match target {
        ExternalOpenApp::Finder => {
            if matches!(platform, Some(RuntimePlatform::MacOs)) {
                ExternalAppLogo::Image(Icon::AppFinder)
            } else {
                ExternalAppLogo::Svg(Icon::AppFileExplorer)
            }
        }
        ExternalOpenApp::VSCode => ExternalAppLogo::Svg(Icon::AppVSCode),
        ExternalOpenApp::Cursor => ExternalAppLogo::Svg(Icon::AppCursor),
        ExternalOpenApp::Trae => ExternalAppLogo::Svg(Icon::AppTrae),
        ExternalOpenApp::Windsurf => ExternalAppLogo::Image(Icon::AppWindsurf),
        ExternalOpenApp::Kiro => ExternalAppLogo::Svg(Icon::AppKiro),
        ExternalOpenApp::Zed => ExternalAppLogo::Svg(Icon::AppZed),
        ExternalOpenApp::TextMate => ExternalAppLogo::Image(Icon::AppTextMate),
        ExternalOpenApp::Antigravity => ExternalAppLogo::Svg(Icon::AppAntigravity),
        ExternalOpenApp::Terminal => ExternalAppLogo::Image(Icon::AppTerminal),
        ExternalOpenApp::ITerm2 => ExternalAppLogo::Svg(Icon::AppITerm2),
        ExternalOpenApp::Ghostty => ExternalAppLogo::Svg(Icon::AppGhostty),
        ExternalOpenApp::Xcode => ExternalAppLogo::Image(Icon::AppXcode),
        ExternalOpenApp::AndroidStudio => ExternalAppLogo::Svg(Icon::AppAndroidStudio),
        ExternalOpenApp::PowerShell => ExternalAppLogo::Svg(Icon::AppPowerShell),
        ExternalOpenApp::SublimeText => ExternalAppLogo::Svg(Icon::AppSublimeText),
    }
}

fn external_app_logo_element(
    target: ExternalOpenApp,
    platform: Option<RuntimePlatform>,
) -> Element<'static, Message> {
    match external_app_logo(target, platform) {
        ExternalAppLogo::Svg(icon) => Svg::new(assets::get_icon(icon))
            .width(Length::Fixed(16.0))
            .height(Length::Fixed(16.0))
            .into(),
        ExternalAppLogo::Image(icon) => image(assets::get_image(icon))
            .width(Length::Fixed(16.0))
            .height(Length::Fixed(16.0))
            .into(),
    }
}

pub(super) fn supported_external_apps(app: &App) -> Vec<ExternalOpenApp> {
    OPEN_EXTERNAL_TARGETS
        .into_iter()
        .filter(|target| app.open_external_exists.contains_key(target))
        .collect()
}

/// 构建或处理 `open_external_module` 对应的界面片段与交互数据。
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
pub(super) fn open_external_module(app: &App) -> Element<'_, Message> {
    if !matches!(app.screen, Screen::Project) {
        return Space::new().into();
    }

    let app_logo = |target: ExternalOpenApp| -> Element<'static, Message> {
        external_app_logo_element(target, app.open_external_platform)
    };

    let open_external_tip =
        crate::app::i18n::I18n::new(app.desktop_locale).__("在外部应用打开项目");
    let can_open_external_preferred =
        app.project_path.is_some() && app.can_open_external(app.open_external_app);
    let open_external_primary: Element<'_, Message> = {
        let enabled = can_open_external_preferred;
        let text_alpha = if enabled { 1.0 } else { 0.35 };
        let logo = app_logo(app.open_external_app);
        let label = text(
            crate::app::i18n::I18n::new(app.desktop_locale)
                .__(external_app_label(app.open_external_app, app.open_external_platform)),
        )
        .size(13)
        .style(move |theme: &Theme| iced::widget::text::Style {
            color: Some(theme.palette().text.scale_alpha(text_alpha)),
        });
        let content: Element<'_, Message> = row![
            container(logo).width(Length::Fixed(18.0)).height(Length::Fixed(18.0)),
            Space::new().width(6),
            label,
        ]
        .align_y(iced::Alignment::Center)
        .into();

        let base = button(content).height(Length::Fixed(24.0)).padding([4, 8]).style(
            move |theme: &Theme, status| {
                let palette = theme.extended_palette();
                let bg = if !enabled {
                    None
                } else {
                    match status {
                        iced::widget::button::Status::Hovered => {
                            Some(palette.background.weak.color.into())
                        }
                        iced::widget::button::Status::Pressed => {
                            Some(palette.background.strong.color.into())
                        }
                        _ => None,
                    }
                };
                iced::widget::button::Style {
                    background: bg,
                    border: iced::Border {
                        width: 0.0,
                        color: Color::TRANSPARENT,
                        radius: 0.0.into(),
                    },
                    text_color: theme.palette().text.scale_alpha(text_alpha),
                    ..Default::default()
                }
            },
        );

        let btn = if enabled {
            base.on_press(Message::View(message::ViewMessage::OpenProjectInExternalPreferred))
        } else {
            base
        };
        let tip_content = container(text(open_external_tip.to_string()).size(12))
            .padding([6, 10])
            .style(|theme: &Theme| iced::widget::container::Style {
                text_color: Some(theme.palette().text),
                background: Some(iced::Background::Color(theme.palette().background)),
                border: iced::Border {
                    width: 1.0,
                    color: theme.extended_palette().background.strong.color.scale_alpha(0.70),
                    radius: 10.0.into(),
                },
                shadow: iced::Shadow {
                    color: Color::BLACK.scale_alpha(0.18),
                    offset: iced::Vector::new(0.0, 10.0),
                    blur_radius: 26.0,
                },
                ..Default::default()
            });

        Tooltip::new(btn, tip_content, TooltipPosition::Bottom).gap(2.0).into()
    };

    let supported_apps = supported_external_apps(app);

    let open_external_menu_btn = icon_toggle_button_opt(
        Icon::ChevronDown,
        crate::app::i18n::I18n::new(app.desktop_locale).__("选择外部应用"),
        TooltipPosition::Bottom,
        app.active_menu == Some(MenuType::OpenExternal),
        Some(Message::View(message::ViewMessage::ToggleMenu(Some(MenuType::OpenExternal)))),
    );

    let mut open_external_items: Vec<Element<'_, Message>> = Vec::new();
    for target in supported_apps {
        let enabled = app.can_open_external(target);
        open_external_items.push(menu_item_icon_btn(
            app_logo(target),
            crate::app::i18n::I18n::new(app.desktop_locale)
                .__(external_app_label(target, app.open_external_platform)),
            (app.open_external_app == target).then_some("✓"),
            enabled
                .then_some(Message::View(message::ViewMessage::OpenProjectInExternalWith(target))),
        ));
    }

    let separator: Element<'_, Message> =
        container(container(Space::new().width(Length::Fill).height(Length::Fixed(1.0))).style(
            |theme: &Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(
                    theme.extended_palette().background.strong.color.scale_alpha(0.70),
                )),
                ..Default::default()
            },
        ))
        .padding([6, 12])
        .into();
    open_external_items.push(separator);

    let can_copy_path = app.project_path.is_some();
    open_external_items.push(menu_item_icon_btn(
        icon_svg(Icon::Copy).into(),
        crate::app::i18n::I18n::new(app.desktop_locale).__("复制路径"),
        None,
        can_copy_path.then_some(Message::View(message::ViewMessage::CopyProjectPath)),
    ));

    let open_external_content = menu_container(column(open_external_items).into());
    let open_external_menu: Element<'_, Message> =
        BelowOverlay::new(open_external_menu_btn, open_external_content)
            .show(app.active_menu == Some(MenuType::OpenExternal))
            .on_close(Message::View(message::ViewMessage::ToggleMenu(None)))
            .into();

    let open_external_divider: Element<'_, Message> = container(
        container(Space::new().width(Length::Fixed(1.0)).height(Length::Fixed(16.0))).style(
            |theme: &Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(
                    theme.extended_palette().background.strong.color.scale_alpha(0.65),
                )),
                ..Default::default()
            },
        ),
    )
    .height(Length::Fixed(24.0))
    .align_y(iced::Alignment::Center)
    .into();

    container(row![open_external_primary, open_external_divider, open_external_menu].spacing(0))
        .style(|theme: &Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(
                theme.extended_palette().background.weak.color,
            )),
            border: iced::Border {
                width: 1.0,
                color: theme.extended_palette().background.strong.color.scale_alpha(0.70),
                radius: 6.0.into(),
            },
            ..Default::default()
        })
        .into()
}
