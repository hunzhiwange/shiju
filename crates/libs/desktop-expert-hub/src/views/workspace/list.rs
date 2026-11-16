//! 桌面注入只读领域输入并映射消息。
use crate::views::workspace::{Message, ViewInput as WorkspaceInput};
use crate::views::{self, list as domain};
use desktop_skill_hub::skill_hub::*;
use iced::{Element, Theme};
fn map(message: views::Message) -> Message {
    match message {
        views::Message::SkillHub(value) => Message::SkillHub(value),
        views::Message::Connectors(value) => {
            Message::SkillHub(crate::views::workspace::SkillHubMessage::Connectors(value))
        }
        views::Message::StartBrowserLogin => {
            Message::Account(crate::views::workspace::AccountMessage::StartBrowserLogin)
        }
        views::Message::None => Message::None,
    }
}
pub(super) use domain::{
    MarketTagFilterOption, MineTagFilterOption, SCROLLBAR_GUTTER, SCROLLBAR_WIDTH,
};
pub(super) fn view<'a>(app: &WorkspaceInput<'a>) -> Element<'a, Message> {
    let input = views::ViewInput {
        expert_hub: &app.expert_hub,
        desktop_locale: app.desktop_locale,
        app_theme: &app.app_theme,
    };
    domain::view(
        &input,
        views::install_guide::view(&views::install_guide::InstallInput {
            state: &app.expert_hub.install_guide,
            desktop_locale: app.desktop_locale,
            project_path: &app.project_path,
            recent_projects: &app.recent_projects,
        }),
    )
    .map(map)
}
pub(super) fn market_search_messages(value: String) -> (Message, Message) {
    {
        let (a, b) = domain::market_search_messages(value);
        (map(a), map(b))
    }
}
pub(super) fn pagination_messages<T>(page: &Page<T>) -> (Option<Message>, Option<Message>) {
    {
        let (a, b) = domain::pagination_messages(page);
        (a.map(map), b.map(map))
    }
}
pub(super) fn access_action_message() -> Message {
    map(domain::access_action_message())
}
pub(super) fn detail_action_message(skill_id: u64) -> Message {
    map(domain::detail_action_message(skill_id))
}
pub(super) fn install_action_message(skill_id: u64, installed: bool) -> Message {
    map(domain::install_action_message(skill_id, installed))
}
pub(super) fn marketplace_quick_upgrade_action(
    skill: &MarketplaceSkill,
    installed: &[InstalledSkill],
) -> Option<SkillVersionAction> {
    domain::marketplace_quick_upgrade_action(skill, installed)
}
pub(super) fn installed_quick_upgrade_message(skill: &InstalledSkill) -> Option<Message> {
    domain::installed_quick_upgrade_message(skill).map(map)
}
pub(super) fn description_preview(description: &str) -> String {
    domain::description_preview(description)
}
pub(super) fn description_tooltip(description: String) -> Element<'static, Message> {
    domain::description_tooltip(description).map(map)
}
pub(super) fn skill_tab_specs() -> &'static [(&'static str, SkillTab)] {
    domain::skill_tab_specs()
}
pub(super) fn grid_column_count(width: f32) -> usize {
    domain::grid_column_count(width)
}
pub(super) fn mine_tag_filter_options<'a>(
    i18n: desktop_i18n::I18n,
    tags: impl Iterator<Item = &'a desktop_skill_hub::skill_hub::SkillTag>,
    selected_ids: &[u64],
) -> Vec<MineTagFilterOption> {
    domain::mine_tag_filter_options(i18n, tags, selected_ids)
}
pub(super) fn mine_catalog_filter_options(
    i18n: desktop_i18n::I18n,
    catalog: &LoadState<Vec<desktop_skill_hub::skill_hub::SkillTag>>,
    selected_ids: &[u64],
) -> (Vec<MineTagFilterOption>, Option<&'static str>) {
    domain::mine_catalog_filter_options(i18n, catalog, selected_ids)
}
pub(super) fn market_catalog_filter_options(
    i18n: desktop_i18n::I18n,
    catalog: &LoadState<Vec<desktop_skill_hub::skill_hub::SkillTag>>,
) -> (Vec<MarketTagFilterOption>, Option<&'static str>) {
    domain::market_catalog_filter_options(i18n, catalog)
}
pub(super) fn market_tag_option_style(
    theme: &Theme,
    selected: bool,
) -> iced::widget::button::Style {
    domain::market_tag_option_style(theme, selected)
}
pub(super) fn mine_named_tag_filter_label(name: &str, selected_ids: &[u64]) -> String {
    domain::mine_named_tag_filter_label(name, selected_ids)
}
pub(super) fn mine_status_filter_label(
    i18n: desktop_i18n::I18n,
    selected: &[MineSkillStatus],
) -> String {
    domain::mine_status_filter_label(i18n, selected)
}
pub(super) fn mine_filter_menu_style(theme: &Theme) -> iced::widget::container::Style {
    domain::mine_filter_menu_style(theme)
}
pub(super) fn mine_filter_menu_entry_style(
    theme: &Theme,
    selected: bool,
) -> iced::widget::container::Style {
    domain::mine_filter_menu_entry_style(theme, selected)
}
