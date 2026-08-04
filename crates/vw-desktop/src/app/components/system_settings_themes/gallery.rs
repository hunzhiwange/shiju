//! 主题画廊：内置 + 自定义主题每行三卡、每页六卡分页展示。

use crate::app::components::system_settings_common::{
    danger_action_btn_style, rounded_action_btn_style, settings_error_banner,
    settings_muted_text_style, settings_page_intro, settings_panel_style, settings_section_card,
    settings_value_badge,
};
use crate::app::message::settings::{SettingsMessage, ThemesMessage};
use crate::app::theme_catalog::{
    ThemePaletteHex, ThemeStore, builtin_overlay_exists, bundled_preset_is_modified,
    current_themes_root, is_bundled_preset_id, theme_store_from_home,
};
use crate::app::{App, Message};
use iced::widget::image::Handle as ImageHandle;
use iced::widget::{Image, button, column, container, row, text};
use iced::{Alignment, Background, Border, Color, ContentFit, Element, Length, Theme};

/// 画廊每行卡片数。
pub const GALLERY_COLUMNS: usize = 3;
/// 画廊每页展示的卡片数（两行 × 三列）。
pub const GALLERY_PAGE_SIZE: usize = 6;

/// 画廊条目来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeGallerySource {
    Builtin,
    /// 仓库嵌入并种子到本地的预设主题（可编辑，不可删除）。
    Preset,
    Custom,
}

/// 画廊中的一条主题。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeGalleryEntry {
    pub id: String,
    pub name: String,
    pub tagline: Option<String>,
    pub source: ThemeGallerySource,
    pub palette: ThemePaletteHex,
    /// 封面图绝对路径（预设/自定义主题且文件存在时有值）。
    pub cover_path: Option<String>,
    /// 内置主题或预设主题是否相对默认内容已修改。
    pub modified: bool,
}

/// 总页数（至少 1，便于空列表仍显示空态页）。
pub fn gallery_page_count(total_entries: usize) -> usize {
    total_entries.div_ceil(GALLERY_PAGE_SIZE).max(1)
}

/// 将页码夹紧到合法范围 `[0, page_count - 1]`。
pub fn clamp_gallery_page(page_index: usize, total_entries: usize) -> usize {
    let max_page = gallery_page_count(total_entries).saturating_sub(1);
    page_index.min(max_page)
}

/// 返回当前页在条目列表中的半开区间 `[start, end)`。
pub fn gallery_page_range(total_entries: usize, page_index: usize) -> (usize, usize) {
    let page = clamp_gallery_page(page_index, total_entries);
    let start = page.saturating_mul(GALLERY_PAGE_SIZE);
    let end = (start + GALLERY_PAGE_SIZE).min(total_entries);
    (start, end)
}

/// 从 Iced 主题提取六色十六进制调色板。
pub fn palette_hex_from_theme(theme: &Theme) -> ThemePaletteHex {
    let palette = theme.palette();
    ThemePaletteHex {
        background: color_to_hex(palette.background),
        text: color_to_hex(palette.text),
        primary: color_to_hex(palette.primary),
        success: color_to_hex(palette.success),
        warning: color_to_hex(palette.warning),
        danger: color_to_hex(palette.danger),
    }
}

fn color_to_hex(color: Color) -> String {
    format!(
        "#{:02x}{:02x}{:02x}",
        (color.r * 255.0).round() as u8,
        (color.g * 255.0).round() as u8,
        (color.b * 255.0).round() as u8,
    )
}

fn parse_hex_color(value: &str) -> Color {
    if value.len() == 7 && value.as_bytes()[0] == b'#' {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&value[1..3], 16),
            u8::from_str_radix(&value[3..5], 16),
            u8::from_str_radix(&value[5..7], 16),
        ) {
            return Color::from_rgb8(r, g, b);
        }
    }
    Color::from_rgb8(0x88, 0x88, 0x88)
}

fn cover_path_for(store: &ThemeStore, id: &str, relative: Option<&str>) -> Option<String> {
    let relative = relative?;
    let path = store.theme_asset_path(id, relative);
    path.is_file().then(|| path.to_string_lossy().into_owned())
}

/// 构建内置 + 预设 + 自定义主题画廊条目（顺序：Builtin → Preset → Custom）。
pub fn build_gallery_entries(store: Option<&ThemeStore>) -> Result<Vec<ThemeGalleryEntry>, String> {
    let themes_root = current_themes_root();
    let mut entries = Vec::with_capacity(Theme::ALL.len());
    for theme in Theme::ALL.iter() {
        let name = theme.to_string();
        let modified = themes_root.as_ref().is_some_and(|root| builtin_overlay_exists(root, &name));
        let palette = if modified {
            if let Some(root) = themes_root.as_ref() {
                match crate::app::theme_catalog::load_builtin_overlay(root, &name) {
                    Ok(Some(definition)) => definition.palette,
                    _ => palette_hex_from_theme(theme),
                }
            } else {
                palette_hex_from_theme(theme)
            }
        } else {
            palette_hex_from_theme(theme)
        };
        entries.push(ThemeGalleryEntry {
            id: name.clone(),
            name,
            tagline: None,
            source: ThemeGallerySource::Builtin,
            palette,
            cover_path: None,
            modified,
        });
    }

    if let Some(store) = store {
        let listed = store.list().map_err(|err| err.to_string())?;
        let mut presets = Vec::new();
        let mut customs = Vec::new();
        for definition in listed {
            let cover_path =
                cover_path_for(store, &definition.id, definition.assets.cover.as_deref());
            let source = if is_bundled_preset_id(&definition.id) {
                ThemeGallerySource::Preset
            } else {
                ThemeGallerySource::Custom
            };
            let modified = source == ThemeGallerySource::Preset
                && themes_root.as_ref().is_some_and(|root| {
                    bundled_preset_is_modified(root, &definition.id).unwrap_or(false)
                });
            let entry = ThemeGalleryEntry {
                id: definition.id,
                name: definition.name,
                tagline: definition.tagline,
                source,
                palette: definition.palette,
                cover_path,
                modified,
            };
            match entry.source {
                ThemeGallerySource::Preset => presets.push(entry),
                ThemeGallerySource::Custom => customs.push(entry),
                ThemeGallerySource::Builtin => unreachable!("store themes are never Builtin"),
            }
        }
        entries.extend(presets);
        entries.extend(customs);
    }

    Ok(entries)
}

fn entry_is_active(app: &App, entry: &ThemeGalleryEntry) -> bool {
    match entry.source {
        ThemeGallerySource::Builtin => {
            app.active_custom_theme_id.is_none() && app.app_theme.to_string() == entry.id
        }
        ThemeGallerySource::Preset | ThemeGallerySource::Custom => {
            app.active_custom_theme_id.as_deref() == Some(entry.id.as_str())
        }
    }
}

fn palette_swatch_row(palette: &ThemePaletteHex) -> Element<'static, Message> {
    let swatches = [
        ("背景", &palette.background),
        ("文本", &palette.text),
        ("主色", &palette.primary),
        ("成功", &palette.success),
        ("警告", &palette.warning),
        ("危险", &palette.danger),
    ];
    row(swatches
        .into_iter()
        .map(|(label, hex)| {
            let color = parse_hex_color(hex);
            container(
                column![
                    container(text(" ").size(1))
                        .width(Length::Fixed(22.0))
                        .height(Length::Fixed(14.0))
                        .style(move |_theme: &Theme| iced::widget::container::Style {
                            background: Some(Background::Color(color)),
                            border: Border {
                                radius: 3.0.into(),
                                width: 1.0,
                                color: Color::from_rgba8(15, 23, 42, 0.18),
                            },
                            ..Default::default()
                        }),
                    text(label).size(8).style(settings_muted_text_style),
                ]
                .spacing(3)
                .align_x(Alignment::Center),
            )
            .into()
        })
        .collect::<Vec<_>>())
    .spacing(6)
    .into()
}

fn cover_thumbnail(path: &str) -> Element<'static, Message> {
    let handle = ImageHandle::from_path(std::path::PathBuf::from(path));
    container(
        Image::new(handle).content_fit(ContentFit::Cover).width(Length::Fill).height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fixed(72.0))
    .style(|_theme: &Theme| iced::widget::container::Style {
        background: Some(Background::Color(Color::from_rgb8(15, 23, 42))),
        border: Border {
            radius: 10.0.into(),
            width: 1.0,
            color: Color::from_rgba8(148, 163, 184, 0.35),
        },
        ..Default::default()
    })
    .into()
}

fn gallery_card_style(theme: &Theme) -> iced::widget::container::Style {
    let mut style = settings_panel_style(theme);
    style.border.radius = 14.0.into();
    style
}

fn gallery_card<'a>(app: &'a App, entry: &'a ThemeGalleryEntry) -> Element<'a, Message> {
    let is_active = entry_is_active(app, entry);
    let source_label = match entry.source {
        ThemeGallerySource::Builtin => {
            if entry.modified {
                "已修改"
            } else {
                "内置"
            }
        }
        ThemeGallerySource::Preset => "预设",
        ThemeGallerySource::Custom => "自定义",
    };

    let mut badges = row![settings_value_badge(source_label)].spacing(6);
    if is_active {
        badges = badges.push(settings_value_badge("当前应用"));
    }

    let header = column![
        text(entry.name.clone()).size(14),
        text(entry.tagline.clone().unwrap_or_else(|| match entry.source {
            ThemeGallerySource::Builtin => {
                if entry.modified {
                    "本地覆盖的内置主题".to_string()
                } else {
                    "Iced 内置主题".to_string()
                }
            }
            ThemeGallerySource::Preset => "内置预设主题".to_string(),
            ThemeGallerySource::Custom => "本地自定义主题".to_string(),
        }))
        .size(10)
        .style(settings_muted_text_style),
        badges,
    ]
    .spacing(6)
    .width(Length::Fill);

    let apply_msg = match entry.source {
        ThemeGallerySource::Builtin => Message::Settings(SettingsMessage::Themes(
            ThemesMessage::ApplyBuiltin(entry.id.clone()),
        )),
        ThemeGallerySource::Preset | ThemeGallerySource::Custom => {
            Message::Settings(SettingsMessage::Themes(ThemesMessage::ApplyCustom(entry.id.clone())))
        }
    };

    let mut actions = row![
        button(text(if is_active { "已应用" } else { "应用" }).size(11))
            .padding([6, 10])
            .on_press_maybe(if is_active { None } else { Some(apply_msg) })
            .style(rounded_action_btn_style),
    ]
    .spacing(6);

    actions = actions.push(
        button(text("导出").size(11))
            .padding([6, 10])
            .on_press(Message::Settings(SettingsMessage::Themes(ThemesMessage::ExportPack(
                entry.id.clone(),
            ))))
            .style(rounded_action_btn_style),
    );

    match entry.source {
        ThemeGallerySource::Builtin => {
            let edit_name = entry.id.clone();
            actions = actions.push(
                button(text("编辑").size(11))
                    .padding([6, 10])
                    .on_press(Message::Settings(SettingsMessage::Themes(
                        ThemesMessage::EditBuiltin(edit_name),
                    )))
                    .style(rounded_action_btn_style),
            );
            if entry.modified {
                let restore_name = entry.id.clone();
                actions = actions.push(
                    button(text("恢复默认").size(11))
                        .padding([6, 10])
                        .on_press(Message::Settings(SettingsMessage::Themes(
                            ThemesMessage::RestoreBuiltinDefault(restore_name),
                        )))
                        .style(danger_action_btn_style),
                );
            }
        }
        ThemeGallerySource::Preset => {
            let edit_id = entry.id.clone();
            actions = actions.push(
                button(text("编辑").size(11))
                    .padding([6, 10])
                    .on_press(Message::Settings(SettingsMessage::Themes(
                        ThemesMessage::EditCustom(edit_id),
                    )))
                    .style(rounded_action_btn_style),
            );
            if entry.modified {
                let restore_id = entry.id.clone();
                actions = actions.push(
                    button(text("恢复默认").size(11))
                        .padding([6, 10])
                        .on_press(Message::Settings(SettingsMessage::Themes(
                            ThemesMessage::RestorePresetDefault(restore_id),
                        )))
                        .style(danger_action_btn_style),
                );
            }
        }
        ThemeGallerySource::Custom => {
            let edit_id = entry.id.clone();
            let delete_id = entry.id.clone();
            actions = actions.push(
                button(text("编辑").size(11))
                    .padding([6, 10])
                    .on_press(Message::Settings(SettingsMessage::Themes(
                        ThemesMessage::EditCustom(edit_id),
                    )))
                    .style(rounded_action_btn_style),
            );
            actions = actions.push(
                button(text("删除").size(11))
                    .padding([6, 10])
                    .on_press(Message::Settings(SettingsMessage::Themes(
                        ThemesMessage::DeleteCustom(delete_id),
                    )))
                    .style(danger_action_btn_style),
            );
        }
    }

    let mut body = column![header].spacing(10).width(Length::Fill);
    if let Some(cover_path) = entry.cover_path.as_deref() {
        body = body.push(cover_thumbnail(cover_path));
    }
    body = body.push(palette_swatch_row(&entry.palette));
    body = body.push(actions);

    container(body)
        .padding([14, 14])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(gallery_card_style)
        .into()
}

fn pagination_controls(app: &App) -> Element<'_, Message> {
    let total = app.themes_settings.entries.len();
    let page_count = gallery_page_count(total);
    let page = clamp_gallery_page(app.themes_settings.page_index, total);
    let can_prev = page > 0;
    let can_next = page + 1 < page_count;

    row![
        button(text("上一页").size(12))
            .padding([8, 14])
            .on_press_maybe(can_prev.then_some(Message::Settings(SettingsMessage::Themes(
                ThemesMessage::GalleryPrevPage,
            ))))
            .style(rounded_action_btn_style),
        text(format!("第 {} / {} 页", page + 1, page_count))
            .size(12)
            .style(settings_muted_text_style),
        button(text("下一页").size(12))
            .padding([8, 14])
            .on_press_maybe(can_next.then_some(Message::Settings(SettingsMessage::Themes(
                ThemesMessage::GalleryNextPage,
            ))))
            .style(rounded_action_btn_style),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .into()
}

/// 渲染主题画廊视图。
pub fn view(app: &App) -> Element<'_, Message> {
    let total = app.themes_settings.entries.len();
    let page = clamp_gallery_page(app.themes_settings.page_index, total);
    let (start, end) = gallery_page_range(total, page);

    let mut cards: Vec<Element<'_, Message>> = app
        .themes_settings
        .entries
        .get(start..end)
        .unwrap_or(&[])
        .iter()
        .map(|entry| {
            container(gallery_card(app, entry))
                .width(Length::FillPortion(1))
                .height(Length::Shrink)
                .into()
        })
        .collect();

    // 不足一页时用空白占位，保持每行三列宽度比例稳定。
    while cards.len() < GALLERY_PAGE_SIZE && total > 0 {
        cards.push(container(text("")).width(Length::FillPortion(1)).into());
    }

    let cards_grid: Element<'_, Message> = if total == 0 {
        container(text("暂无主题，点击刷新或新建主题。").size(13).style(settings_muted_text_style))
            .padding([12, 4])
            .width(Length::Fill)
            .into()
    } else {
        let mut rows = column![].spacing(12).width(Length::Fill);
        let mut remaining = cards;
        while !remaining.is_empty() {
            let take = remaining.len().min(GALLERY_COLUMNS);
            let row_cards: Vec<Element<'_, Message>> = remaining.drain(..take).collect();
            rows = rows.push(row(row_cards).spacing(12).width(Length::Fill).height(Length::Shrink));
        }
        rows.into()
    };

    let toolbar = row![
        button(text("刷新").size(12))
            .padding([8, 14])
            .on_press(Message::Settings(SettingsMessage::Themes(ThemesMessage::Refresh)))
            .style(rounded_action_btn_style),
        button(text("新建主题").size(12))
            .padding([8, 14])
            .on_press(Message::Settings(SettingsMessage::Themes(ThemesMessage::NewThemeRequested,)))
            .style(rounded_action_btn_style),
        button(text("导入主题包").size(12))
            .padding([8, 14])
            .on_press(Message::Settings(SettingsMessage::Themes(ThemesMessage::ImportPack,)))
            .style(rounded_action_btn_style),
        container(pagination_controls(app)).width(Length::Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let mut body = column![
        settings_page_intro("主题", "浏览内置、预设与自定义主题，应用或管理本地主题包。"),
        settings_section_card(
            "主题画廊",
            "所有主题均可导出；预设主题可编辑并恢复默认、不可删除，自定义主题可编辑或删除。",
        ),
        toolbar,
    ]
    .spacing(12);

    if let Some(error) = app.themes_settings.error.as_deref() {
        body = body.push(settings_error_banner(error));
    }

    body = body.push(cards_grid);

    body.spacing(12).into()
}

/// 供消息层刷新画廊列表，并夹紧当前页码。
pub fn refresh_gallery_state(app: &mut App) {
    let store = theme_store_from_home();
    match build_gallery_entries(store.as_ref()) {
        Ok(entries) => {
            let total = entries.len();
            app.themes_settings.entries = entries;
            app.themes_settings.page_index =
                clamp_gallery_page(app.themes_settings.page_index, total);
            app.themes_settings.error = None;
        }
        Err(error) => {
            app.themes_settings.error = Some(error);
        }
    }
}
