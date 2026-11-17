//! Skills catalog page composition.
use crate::Event as Message;
use crate::catalog::{catalog_group_section, catalog_matches_query, icon_svg};
use crate::types::SkillsDirectoryScope;
use crate::{BrowserContext, Event};
use desktop_assets::assets::Icon;
use desktop_i18n::settings_extensions::text as tr;
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, settings_field_label,
    settings_group_card, settings_muted_text_style, settings_select, settings_text_input_style,
};
use iced::widget::{column, container, row, text, text_input};
use iced::{Alignment, Element, Length};

pub use crate::browser_detail::{detail_modal_body, view_overlays};
pub use crate::browser_discovery::{
    discovery_order_text, discovery_order_view, provider_global_paths, provider_project_paths,
    scope_description, scope_source_matches, scope_title, skill_project_options,
};
pub use crate::browser_widgets::{
    DetailActionStyle, catalog_panel_style, detail_action_button, detail_source_note, empty_state,
    header_panel_style, loading_banner, refresh_button, scope_button, status_banner,
};

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
pub fn view<'a>(app: BrowserContext<'a>) -> Element<'a, Message> {
    let locale = app.desktop_locale;
    let s = app.skills_settings;
    let project_open = app.project_path.is_some();
    let skills = s
        .catalog
        .iter()
        .filter(|skill| catalog_matches_query(locale, skill, s.query.trim()))
        .filter(|skill| scope_source_matches(s.directory_scope, &skill.source))
        .cloned()
        .collect::<Vec<_>>();

    let search_input = text_input(tr(locale, "搜索技能"), s.query)
        .on_input(|value| Event::SkillsQueryChanged(value))
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style)
        .width(Length::Fill);
    let project_options = skill_project_options(app);
    let project_switcher: Element<'_, Message> = if project_options.is_empty() {
        text(tr(locale, "尚未打开项目；当前仅显示全局与内置技能。"))
            .size(12)
            .style(settings_muted_text_style)
            .into()
    } else {
        row![
            settings_field_label(tr(locale, "项目目录")),
            settings_select(project_options, (*app.project_path).clone(), |path| {
                Event::SkillsProjectSelected(path)
            })
            .placeholder(tr(locale, "选择项目目录"))
            .width(Length::Fill),
            text(if s.loading { tr(locale, "切换中") } else { tr(locale, "技能作用域") })
                .size(12)
                .style(settings_muted_text_style),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .into()
    };

    let filters = settings_group_card(
        container(
            column![
                row![icon_svg(Icon::Search, 16.0), search_input, refresh_button(locale, s.loading)]
                    .spacing(10)
                    .align_y(Alignment::Center),
                project_switcher,
                row![
                    scope_button(
                        tr(locale, "项目目录"),
                        SkillsDirectoryScope::Project,
                        s.directory_scope,
                        project_open,
                    ),
                    scope_button(
                        tr(locale, "父级目录"),
                        SkillsDirectoryScope::Ancestor,
                        s.directory_scope,
                        project_open,
                    ),
                    scope_button(
                        tr(locale, "全局目录"),
                        SkillsDirectoryScope::Global,
                        s.directory_scope,
                        true
                    ),
                    scope_button(
                        tr(locale, "内置技能"),
                        SkillsDirectoryScope::Bundled,
                        s.directory_scope,
                        true
                    ),
                    scope_button(
                        tr(locale, "全部目录"),
                        SkillsDirectoryScope::All,
                        s.directory_scope,
                        true
                    ),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .wrap(),
            ]
            .spacing(12),
        )
        .padding([14, 0]),
    );

    let mut content = column![filters].spacing(20).width(Length::Fill);

    if let Some(message) = s.status_message {
        let rendered = s
            .status_message_translation
            .as_ref()
            .map(|value| {
                value.render(desktop_i18n::I18n::new(locale)).expect("valid skills status template")
            })
            .unwrap_or_else(|| message.clone());
        content = content.push(status_banner(rendered, s.status_is_error));
    }

    let mut catalog_panel = column![
        row![
            column![
                text(tr(locale, scope_title(s.directory_scope))).size(18),
                text(scope_description(locale, s.directory_scope, s.directory_provider))
                    .size(13)
                    .style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fill),
            text(
                desktop_i18n::__!(desktop_i18n::I18n::new(locale), "%u 项技能", skills.len())
                    .expect("valid skill results template")
            )
            .size(12)
            .style(settings_muted_text_style),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    ]
    .spacing(20)
    .width(Length::Fill);

    if s.loading {
        catalog_panel =
            catalog_panel.push(loading_banner(tr(locale, "正在通过 gateway 同步技能目录...")));
    }

    let workspace =
        skills.iter().filter(|skill| skill.source == "workspace").cloned().collect::<Vec<_>>();
    let ancestor =
        skills.iter().filter(|skill| skill.source == "ancestor").cloned().collect::<Vec<_>>();
    let global =
        skills.iter().filter(|skill| skill.source == "global").cloned().collect::<Vec<_>>();
    let bundled =
        skills.iter().filter(|skill| skill.source == "bundled").cloned().collect::<Vec<_>>();

    if !workspace.is_empty() {
        catalog_panel = catalog_panel.push(catalog_group_section(
            locale,
            "workspace",
            workspace,
            s.selected_skill_id.as_deref(),
        ));
    }
    if !ancestor.is_empty() {
        catalog_panel = catalog_panel.push(catalog_group_section(
            locale,
            "ancestor",
            ancestor,
            s.selected_skill_id.as_deref(),
        ));
    }
    if !global.is_empty() {
        catalog_panel = catalog_panel.push(catalog_group_section(
            locale,
            "global",
            global,
            s.selected_skill_id.as_deref(),
        ));
    }
    if !bundled.is_empty() {
        catalog_panel = catalog_panel.push(catalog_group_section(
            locale,
            "bundled",
            bundled,
            s.selected_skill_id.as_deref(),
        ));
    }

    if skills.is_empty() && !s.loading {
        let (title, description) = match (s.directory_scope, project_open) {
            (SkillsDirectoryScope::Project, false) => (
                tr(locale, "未打开项目目录"),
                tr(locale, "打开项目后可查看当前工程的 skills 目录，或先切换到全部目录。"),
            ),
            (SkillsDirectoryScope::Project, true) => (
                tr(locale, "项目目录下没有技能"),
                tr(
                    locale,
                    "当前项目目录没有命中技能，可以切换到全部目录查看父级、全局和内置技能。",
                ),
            ),
            (SkillsDirectoryScope::Ancestor, false) => {
                (tr(locale, "未打开项目目录"), tr(locale, "打开项目后可查看父级技能目录。"))
            }
            (SkillsDirectoryScope::Ancestor, true) => (
                tr(locale, "父级目录下没有技能"),
                tr(locale, "项目父级目录没有命中技能，可以切换到全局目录或内置技能。"),
            ),
            (SkillsDirectoryScope::Global, _) => (
                tr(locale, "全局目录下没有技能"),
                tr(locale, "当前目录提供方的全局技能目录没有命中技能。"),
            ),
            (SkillsDirectoryScope::Bundled, _) => (
                tr(locale, "没有内置技能"),
                tr(locale, "内置技能目录为空，或当前搜索关键字没有匹配结果。"),
            ),
            (SkillsDirectoryScope::All, _) => (
                tr(locale, "没有匹配的技能"),
                tr(locale, "试试清空搜索关键字，或稍后刷新技能目录。"),
            ),
        };
        catalog_panel = catalog_panel.push(empty_state(title, description));
    }

    content.push(catalog_panel).into()
}
