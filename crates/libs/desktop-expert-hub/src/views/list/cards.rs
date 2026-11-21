use super::{description_tooltip, detail_action_message, install_action_message};
use crate::message::skill_hub::SkillHubMessage;
use crate::views::Message;
use desktop_assets::assets::Icon;
use desktop_i18n::I18n;
use desktop_skill_hub::skill_hub::{InstalledSkill, MarketplaceSkill};
use desktop_ui_controls::single_line_label::SingleLineLabel;
use desktop_ui_controls::system_settings_common::{icon_svg, rounded_action_btn_style};
use gateway_shiju_skill::ShijuSkillSource;
use iced::widget::tooltip::Position;
use iced::widget::{Space, button, column, container, mouse_area, row, text, tooltip};
use iced::{Alignment, Background, Border, Color, Element, Font, Length, Theme, gradient};

pub(super) fn dark(theme: &Theme) -> bool {
    desktop_ui_controls::system_settings_common::theme::is_dark_theme(theme)
}

pub(super) fn ink(theme: &Theme) -> Color {
    if dark(theme) { theme.palette().text } else { Color::from_rgb8(23, 40, 69) }
}

pub(super) fn muted(theme: &Theme) -> Color {
    if dark(theme) {
        theme.palette().text.scale_alpha(0.76)
    } else {
        Color::from_rgb8(95, 107, 130)
    }
}

pub(super) fn accent(theme: &Theme) -> Color {
    if dark(theme) { Color::from_rgb8(245, 139, 98) } else { Color::from_rgb8(177, 66, 35) }
}

pub(super) fn surface(theme: &Theme) -> Color {
    if dark(theme) { theme.extended_palette().background.weak.color } else { Color::WHITE }
}

pub(super) fn edge(theme: &Theme) -> Color {
    if dark(theme) {
        theme.extended_palette().background.strong.color
    } else {
        Color::from_rgb8(231, 225, 216)
    }
}

pub(super) fn panel(theme: &Theme, radius: f32) -> container::Style {
    container::Style {
        background: Some(surface(theme).into()),
        text_color: Some(ink(theme)),
        border: Border { color: edge(theme), width: 1.0, radius: radius.into() },
        ..Default::default()
    }
}

pub(super) fn line() -> Element<'static, Message> {
    container(Space::new())
        .width(Length::Fill)
        .height(1)
        .style(|theme| container::Style {
            background: Some(edge(theme).into()),
            ..Default::default()
        })
        .into()
}

pub(in crate::views) fn muted_text(theme: &Theme) -> text::Style {
    text::Style { color: Some(muted(theme)) }
}

pub(super) fn bold() -> Font {
    Font { weight: iced::font::Weight::Bold, ..Font::default() }
}

fn card_style(theme: &Theme, featured: bool) -> container::Style {
    let mut style = panel(theme, 12.0);
    if featured {
        let base = surface(theme);
        let tint = if dark(theme) {
            Color::from_rgb8(63, 43, 39)
        } else {
            Color::from_rgb8(247, 234, 229)
        };
        style.background = Some(Background::Gradient(
            gradient::Linear::new(iced::Degrees(118.0))
                .add_stop(0.0, tint)
                .add_stop(0.68, base)
                .into(),
        ));
    }
    style
}

pub(super) fn avatar(id: u64, name: &str, size: f32) -> Element<'static, Message> {
    let initial = name.trim().chars().next().unwrap_or('?').to_ascii_uppercase();
    let tone = match id % 5 {
        0 => Color::from_rgb8(133, 76, 53),
        1 => Color::from_rgb8(155, 115, 54),
        2 => Color::from_rgb8(76, 97, 123),
        3 => Color::from_rgb8(82, 104, 68),
        _ => Color::from_rgb8(64, 100, 105),
    };
    container(text(initial.to_string()).size(size * 0.46).font(bold()))
        .center(Length::Fixed(size))
        .style(move |_| container::Style {
            text_color: Some(Color::WHITE),
            background: Some(tone.into()),
            border: Border { radius: (size / 2.0).into(), ..Default::default() },
            ..Default::default()
        })
        .into()
}

pub(in crate::views) fn icon_button(
    icon: Icon,
    label: &'static str,
    action: Option<Message>,
) -> Element<'static, Message> {
    let icon =
        icon_svg(icon, 16.0).style(|theme, _| iced::widget::svg::Style { color: Some(ink(theme)) });
    tooltip(
        button(container(icon).center(Length::Fixed(30.0)))
            .padding(0)
            .style(|theme, status| {
                let mut style = rounded_action_btn_style(theme, status);
                style.border.color = edge(theme);
                style.border.radius = 8.0.into();
                if status == button::Status::Active {
                    style.background = Some(surface(theme).into());
                }
                style
            })
            .on_press_maybe(action),
        container(text(label).size(12)).padding([5, 8]).style(|theme| panel(theme, 6.0)),
        Position::Top,
    )
    .gap(5)
    .into()
}

pub(super) fn source_label(i18n: I18n, source: Option<ShijuSkillSource>) -> &'static str {
    match source {
        Some(ShijuSkillSource::Marketplace) => i18n.__("市场技能"),
        Some(ShijuSkillSource::Tenant) => i18n.__("企业技能"),
        None => "",
    }
}

pub(in crate::views) fn updated_label(i18n: I18n, date: Option<&str>) -> String {
    let Some(date) = date
        .and_then(|value| value.get(..10))
        .and_then(|value| chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
    else {
        return String::new();
    };
    match (chrono::Local::now().date_naive() - date).num_days() {
        0 => i18n.__("今天更新").to_string(),
        days @ 1..=6 => {
            i18n.format("%s天前更新", &[days.to_string().into()]).expect("valid update date")
        }
        _ => i18n
            .format("%s 更新", &[date.format("%m-%d").to_string().into()])
            .expect("valid update date"),
    }
}

pub(in crate::views) fn card_identity(
    id: u64,
    name: &str,
    subtitle: String,
    title_tooltip: String,
    detail: Message,
) -> Element<'static, Message> {
    let title: Element<'static, Message> =
        SingleLineLabel::new(name.to_string(), 17.0).font(bold()).into();
    let identity = row![
        avatar(id, name, 38.0),
        column![
            title,
            container(text(subtitle).size(12).style(muted_text).wrapping(text::Wrapping::None))
                .width(Length::Fill)
                .clip(true),
        ]
        .spacing(2),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .width(Length::Fill);
    mouse_area(tooltip(identity, description_tooltip(title_tooltip), Position::Top).gap(5))
        .on_press(detail)
        .interaction(iced::mouse::Interaction::Pointer)
        .into()
}

pub(in crate::views) fn description_area(
    description: &str,
    width: f32,
    detail: Message,
) -> Element<'static, Message> {
    let preview = description.split_whitespace().collect::<Vec<_>>().join(" ");
    mouse_area(
        tooltip(
            container(
                text(card_preview(&preview, width))
                    .size(14)
                    .line_height(iced::Pixels(21.0))
                    .style(muted_text)
                    .width(Length::Fill),
            )
            .height(42)
            .clip(true),
            description_tooltip(description.to_string()),
            Position::Top,
        )
        .gap(5),
    )
    .on_press(detail)
    .interaction(iced::mouse::Interaction::Pointer)
    .into()
}

pub(in crate::views) fn card_layout(
    heading: Element<'static, Message>,
    description: Element<'static, Message>,
    footer: Element<'static, Message>,
    featured: bool,
) -> Element<'static, Message> {
    container(column![
        heading,
        Space::new().height(8),
        description,
        Space::new().height(10),
        line(),
        Space::new().height(10),
        footer,
    ])
    .padding(iced::Padding { top: 16.0, bottom: 14.0, left: 20.0, right: 20.0 })
    .width(Length::Fill)
    .height(Length::Fixed(167.0))
    .style(move |theme| card_style(theme, featured))
    .into()
}

pub(super) struct Card<'a> {
    pub id: u64,
    pub name: &'a str,
    pub description: &'a str,
    pub developer: Option<&'a str>,
    pub updated_at: Option<&'a str>,
    pub source: Option<ShijuSkillSource>,
    pub version: &'a str,
    pub properties: String,
    pub featured: bool,
    pub installed: bool,
    pub can_install: bool,
    pub pending: bool,
    pub upgrade: Option<(&'a str, Message)>,
    pub force_upgrade: bool,
    pub width: f32,
}

impl Card<'_> {
    pub(super) fn view(self, i18n: I18n) -> Element<'static, Message> {
        let detail = detail_action_message(self.id);
        let developer = self
            .developer
            .filter(|name| !name.trim().is_empty())
            .map(|name| format!("@{name}"))
            .unwrap_or_default();
        let title_tooltip = if self.properties.is_empty() {
            self.name.to_string()
        } else {
            format!("{}\n{}", self.name, self.properties)
        };
        let identity = card_identity(self.id, self.name, developer, title_tooltip, detail.clone());
        let mut heading = row![identity].spacing(8).height(44).align_y(Alignment::Center);
        if self.can_install {
            let (icon, label) = if self.installed {
                (Icon::Trash, i18n.__("卸载"))
            } else {
                (Icon::Plus, i18n.__("安装"))
            };
            heading = heading.push(icon_button(
                icon,
                if self.pending { i18n.__("处理中…") } else { label },
                (!self.pending).then(|| install_action_message(self.id, self.installed)),
            ));
        }

        let updated = updated_label(i18n, self.updated_at);
        let source = source_label(i18n, self.source);
        let meta = match (source.is_empty(), updated.is_empty()) {
            (false, false) => format!("{source}  │  {updated}"),
            (false, true) => source.to_string(),
            _ => updated,
        };
        let mut footer = row![
            container(text(meta).size(12).style(muted_text).wrapping(text::Wrapping::None))
                .width(Length::Fill)
                .clip(true),
            text(self.version.to_string()).size(12).style(muted_text),
        ]
        .spacing(6)
        .align_y(Alignment::Center);
        if let Some((version, message)) = self.upgrade {
            let key = if self.force_upgrade { "必须升级至 %s" } else { "升级至 %s" };
            let label = i18n.format(key, &[version.to_string().into()]).expect("valid version");
            footer = footer.push(
                button(text(format!("↑ {label}")).size(12).font(bold()))
                    .padding(0)
                    .on_press_maybe((!self.pending).then_some(message))
                    .style(|theme, status| {
                        let mut style = button::text(theme, status);
                        style.text_color =
                            accent(theme).scale_alpha(if status == button::Status::Disabled {
                                0.55
                            } else {
                                1.0
                            });
                        style
                    }),
            );
        }
        card_layout(
            heading.into(),
            description_area(self.description, self.width, detail),
            footer.clip(true).into(),
            self.featured,
        )
    }
}

fn card_preview(description: &str, width: f32) -> String {
    let budget = ((width - 44.0) / 14.0 * 2.0).max(1.0);
    let mut used = 0.0;
    let mut preview = String::new();
    let mut chars = description.chars().peekable();
    while let Some(ch) = chars.next() {
        used += if ch.is_ascii() { 0.53 } else { 1.0 };
        if chars.peek().is_some() && used > budget - 1.0 {
            preview.push('…');
            break;
        }
        preview.push(ch);
    }
    preview
}

pub(super) fn marketplace(
    i18n: I18n,
    skill: &MarketplaceSkill,
    installed: &[InstalledSkill],
    busy: bool,
    featured: bool,
    width: f32,
) -> Element<'static, Message> {
    let record = installed.iter().find(|item| item.skill_id == skill.id);
    let upgrade = record.filter(|item| item.upgrade_available).map(|item| {
        let message = super::marketplace_quick_upgrade_action(skill, installed)
            .map(SkillHubMessage::ChangeSkillVersion)
            .unwrap_or(SkillHubMessage::UpgradeSkill(skill.id));
        (item.current_version.as_str(), Message::SkillHub(message))
    });
    let tags = super::tag_summary(&skill.tags);
    let company_tags = super::tag_summary(&skill.company_tags);
    Card {
        id: skill.id,
        name: &skill.name,
        description: &skill.description,
        developer: skill.developer_display_name.as_deref(),
        updated_at: Some(&skill.updated_at),
        source: skill.distribution_source,
        version: skill
            .marketplace_version
            .as_deref()
            .or_else(|| record.map(|item| item.current_version.as_str()))
            .unwrap_or_default(),
        properties: [
            skill.slug.as_str(),
            skill.format.as_str(),
            skill.status.as_str(),
            &tags,
            &company_tags,
        ]
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join(" · "),
        featured,
        installed: skill.is_installed,
        can_install: true,
        pending: busy,
        upgrade,
        force_upgrade: record.is_some_and(|item| item.force_upgrade_required),
        width,
    }
    .view(i18n)
}

pub(super) fn installed(
    i18n: I18n,
    skill: &InstalledSkill,
    busy: bool,
    width: f32,
) -> Element<'static, Message> {
    Card {
        id: skill.skill_id,
        name: &skill.name,
        description: &skill.description,
        developer: skill.developer_display_name.as_deref(),
        updated_at: skill.updated_at.as_deref(),
        source: skill.distribution_source,
        version: &skill.installed_version,
        properties: format!("{} · {} · {}", skill.slug, skill.format, skill.current_version),
        featured: false,
        installed: true,
        can_install: true,
        pending: busy,
        upgrade: super::installed_quick_upgrade_message(skill)
            .map(|message| (skill.current_version.as_str(), message)),
        force_upgrade: skill.force_upgrade_required,
        width,
    }
    .view(i18n)
}
