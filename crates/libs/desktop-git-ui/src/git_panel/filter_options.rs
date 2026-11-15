#[derive(Default)]
pub struct FilterState<'a> {
    pub desktop_locale: config_ui::DesktopLocale,
    pub git_filter_query: &'a str,
    pub git_filter_included: bool,
    pub git_filter_excluded: bool,
    pub git_filter_new: bool,
    pub git_filter_modified: bool,
    pub git_filter_deleted: bool,
}
#[derive(Debug, Clone)]
pub enum FilterMessage {
    FilterQueryChanged(String), FilterToggleIncluded(bool), FilterToggleExcluded(bool),
    FilterToggleNew(bool), FilterToggleModified(bool), FilterToggleDeleted(bool),
    FilterHelpOpen, ToggleFilterOptions(bool), ClearFilters,
}
use iced::widget::{button, column, container, row, svg, text, text_input, toggler};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use desktop_assets::assets::Icon;
use desktop_ui_controls::system_settings_common::{
    icon_svg, round_icon_btn_style, rounded_action_btn_style, settings_panel_style,
    settings_text_input_style, settings_value_badge,
};


pub const FILTER_HELP_TITLE: &str = "筛选选项";

pub const FILTER_HELP_TEXT: &str = r#"筛选选项说明

路径筛选

在输入框中输入文件名或路径片段，只显示匹配的变更文件。

已包含到提交

只显示当前已选中、会被本次提交包含的文件。

已排除出提交

只显示当前未选中、不会被本次提交包含的文件。

新增文件

只显示 Git 状态为新增或未跟踪的文件。

修改文件

只显示 Git 状态为修改或重命名的文件。

删除文件

只显示 Git 状态为删除的文件。

清除筛选

重置路径查询和所有状态开关，恢复完整变更列表。"#;

pub fn help_icon_style(theme: &Theme, _status: svg::Status) -> svg::Style {
    svg::Style { color: Some(theme.palette().text.scale_alpha(0.78)) }
}

pub fn help_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let is_hovered = matches!(
        status,
        iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed
    );

    iced::widget::button::Style {
        background: is_hovered.then(|| palette.background.weak.color.scale_alpha(0.72).into()),
        text_color: theme.palette().text,
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 999.0.into() },
        ..Default::default()
    }
}

pub fn filter_toggle_row_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark =
        theme.palette().background.r + theme.palette().background.g + theme.palette().background.b
            < 1.5;

    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark {
            palette.background.weak.color.scale_alpha(0.18)
        } else {
            Color::from_rgba8(246, 248, 252, 0.96)
        })),
        border: Border {
            width: 1.0,
            color: if is_dark {
                palette.background.strong.color.scale_alpha(0.78)
            } else {
                Color::from_rgba8(15, 23, 42, 0.06)
            },
            radius: 14.0.into(),
        },
        ..Default::default()
    }
}

pub fn filter_query_changed(value: String) -> FilterMessage {
    FilterMessage::FilterQueryChanged(value)
}

pub fn filter_toggle_included(enabled: bool) -> FilterMessage {
    FilterMessage::FilterToggleIncluded(enabled)
}

pub fn filter_toggle_excluded(enabled: bool) -> FilterMessage {
    FilterMessage::FilterToggleExcluded(enabled)
}

pub fn filter_toggle_new(enabled: bool) -> FilterMessage {
    FilterMessage::FilterToggleNew(enabled)
}

pub fn filter_toggle_modified(enabled: bool) -> FilterMessage {
    FilterMessage::FilterToggleModified(enabled)
}

pub fn filter_toggle_deleted(enabled: bool) -> FilterMessage {
    FilterMessage::FilterToggleDeleted(enabled)
}

fn filter_toggle_row<'a>(
    title: &'static str,
    count: usize,
    control: impl Into<Element<'a, FilterMessage>>,
) -> Element<'a, FilterMessage> {
    let title_block = row![text(title).size(13), settings_value_badge(count.to_string()),]
        .spacing(8)
        .align_y(Alignment::Center);

    let control = container(control.into())
        .width(Length::Fixed(40.0))
        .center_x(Length::Shrink)
        .center_y(Length::Shrink);

    container(
        row![container(title_block).width(Length::Fill).center_y(Length::Shrink), control,]
            .spacing(12)
            .align_y(Alignment::Center),
    )
    .padding([10, 12])
    .width(Length::Fill)
    .style(filter_toggle_row_style)
    .into()
}

/// 构建 Git 筛选选项面板视图
///
/// 该函数创建一个包含多种筛选选项的 UI 面板，允许用户通过以下方式筛选 Git 文件列表：
/// - 文本查询：匹配文件路径
/// - 状态开关：按文件的 Git 状态筛选
///
/// # 参数
///
/// - `app`: 当前筛选条件与界面语言的只读投影
/// - `included_count`: 已包含到提交的文件数量
/// - `excluded_count`: 已排除出提交的文件数量
/// - `new_count`: 新增文件的数量
/// - `modified_count`: 修改文件的数量
/// - `deleted_count`: 删除文件的数量
///
/// # 返回值
///
/// 返回一个 `Element<FilterMessage>`，即渲染后的 UI 组件
///
/// # 示例
///
/// ```ignore
/// let filter_view = view(
///     &app,
///     included_files.len(),
///     excluded_files.len(),
///     new_files.len(),
///     modified_files.len(),
///     deleted_files.len(),
/// );
/// ```
pub fn view(
    app: &FilterState<'_>,
    included_count: usize,
    excluded_count: usize,
    new_count: usize,
    modified_count: usize,
    deleted_count: usize,
) -> Element<'static, FilterMessage> {
    let help_btn = button(
        container(icon_svg(Icon::QuestionCircle, 14.0).style(help_icon_style))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .padding(0)
    .width(Length::Fixed(24.0))
    .height(Length::Fixed(24.0))
    .style(help_button_style)
    .on_press(FilterMessage::FilterHelpOpen);

    // 标题栏：包含"筛选选项"文本、帮助入口和关闭按钮
    let title_text = container(
        row![
            text(desktop_i18n::I18n::new(app.desktop_locale).__("筛选选项")).size(14),
            container(help_btn).center_x(Length::Shrink).center_y(Length::Shrink),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .center_y(Length::Shrink);

    let title = row![
        title_text,
        // 关闭按钮，点击后隐藏筛选选项面板
        container(
            button(
                container(icon_svg(Icon::X, 11.0))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Center)
            )
            .padding(0)
            .width(Length::Fixed(28.0))
            .height(Length::Fixed(28.0))
            .on_press(FilterMessage::ToggleFilterOptions(false))
            .style(round_icon_btn_style)
        )
        .center_x(Length::Shrink)
        .center_y(Length::Shrink)
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    // 筛选文本输入框：用于输入文件路径查询关键字
    let filter_input = text_input(
        desktop_i18n::I18n::new(app.desktop_locale).__("筛选"),
        app.git_filter_query,
    )
    .on_input(filter_query_changed)
    .padding([10, 12])
    .size(13)
    .style(settings_text_input_style)
    .width(Length::Fill);

    // 各状态筛选开关
    // 已包含到提交的文件筛选开关
    let included = toggler(app.git_filter_included).on_toggle(filter_toggle_included);

    // 已排除出提交的文件筛选开关
    let excluded = toggler(app.git_filter_excluded).on_toggle(filter_toggle_excluded);

    // 新增文件筛选开关
    let newf = toggler(app.git_filter_new).on_toggle(filter_toggle_new);

    // 修改文件筛选开关
    let modifiedf = toggler(app.git_filter_modified).on_toggle(filter_toggle_modified);

    // 删除文件筛选开关
    let deletedf = toggler(app.git_filter_deleted).on_toggle(filter_toggle_deleted);

    // 清除所有筛选条件的按钮
    let clear =
        button(text(desktop_i18n::I18n::new(app.desktop_locale).__("清除筛选")).size(12))
            .padding([8, 12])
            .on_press(FilterMessage::ClearFilters)
            .style(rounded_action_btn_style);

    // 组装完整的筛选选项面板
    container(
        column![
            title,
            filter_input,
            // 各筛选选项行：开关 + 标签（含数量）
            column![
                filter_toggle_row(
                    desktop_i18n::I18n::new(app.desktop_locale).__("已包含到提交"),
                    included_count,
                    included,
                ),
                filter_toggle_row(
                    desktop_i18n::I18n::new(app.desktop_locale).__("已排除出提交"),
                    excluded_count,
                    excluded,
                ),
                filter_toggle_row(
                    desktop_i18n::I18n::new(app.desktop_locale).__("新增文件"),
                    new_count,
                    newf
                ),
                filter_toggle_row(
                    desktop_i18n::I18n::new(app.desktop_locale).__("修改文件"),
                    modified_count,
                    modifiedf,
                ),
                filter_toggle_row(
                    desktop_i18n::I18n::new(app.desktop_locale).__("删除文件"),
                    deleted_count,
                    deletedf
                ),
            ]
            .spacing(6),
            row![
                container(text("")).width(Length::Fill).center_y(Length::Shrink),
                container(clear).center_x(Length::Shrink).center_y(Length::Shrink)
            ]
            .align_y(Alignment::Center)
            .spacing(8)
        ]
        .spacing(12),
    )
    .padding([16, 18])
    .style(settings_panel_style)
    .into()
}
