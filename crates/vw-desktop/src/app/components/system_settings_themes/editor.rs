//! 主题编辑器：草稿构造、校验与表单视图。

use crate::app::components::color_picker::{parse_color, render_color_picker};
use crate::app::components::overlays::BelowOverlay;
use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, primary_action_btn_style, rounded_action_btn_style,
    settings_error_banner, settings_muted_text_style, settings_page_intro, settings_panel_style,
    settings_pick_list_menu_style, settings_pick_list_style, settings_section_card,
    settings_section_content, settings_segment_button_style, settings_text_input_style,
};
use crate::app::components::system_settings_themes::preview::theme_preview_panel;
use crate::app::message::settings::{
    SettingsMessage, ThemeChromeField, ThemeColorPickerTarget, ThemePaletteField, ThemesMessage,
};
use crate::app::state::{ThemeEditorDraft, ThemeEditorTab};
use crate::app::theme_catalog::{
    PanelImageFit, ThemeAssets, ThemeChrome, ThemeDefinition, ThemePaletteHex, is_bundled_preset_id,
};
use crate::app::{App, Message};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{
    Space, button, column, container, pick_list, row, scrollable, text, text_input,
};
use iced::{Alignment, Background, Color, Element, Length, Theme};

/// 将显示名 slug 化为合法主题 id 基底。
pub(crate) fn slugify_theme_id(name: &str) -> String {
    let mut out = String::new();
    let mut pending_sep: Option<char> = None;
    for c in name.chars() {
        let lower = c.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            if let Some(sep) = pending_sep.take() {
                if !out.is_empty() {
                    out.push(sep);
                }
            }
            out.push(lower);
        } else if lower == '_' {
            pending_sep = Some('_');
        } else if c.is_whitespace() || lower == '-' {
            pending_sep = Some('-');
        }
    }
    if out.is_empty() {
        return "theme".to_string();
    }
    out.chars().take(64).collect()
}

/// 在已占用 id 集合中分配唯一 id；冲突时追加 `-2`、`-3`…
pub(crate) fn allocate_unique_theme_id(base: &str, existing: &[String]) -> String {
    let base = if base.is_empty() { "theme" } else { base };
    if !existing.iter().any(|id| id == base) {
        return base.to_string();
    }
    let mut n = 2u32;
    loop {
        let candidate = format!("{base}-{n}");
        if candidate.len() <= 64 && !existing.iter().any(|id| id == &candidate) {
            return candidate;
        }
        n = n.saturating_add(1);
        if n > 10_000 {
            return format!("{base}-x");
        }
    }
}

/// 新建主题草稿（id 可编辑并由名称派生）。
pub(crate) fn new_theme_draft(existing_ids: &[String]) -> ThemeEditorDraft {
    let id = allocate_unique_theme_id("theme", existing_ids);
    ThemeEditorDraft {
        id,
        id_readonly: false,
        name: String::new(),
        tagline: String::new(),
        ..ThemeEditorDraft::default()
    }
}

fn optional_hex_to_draft(value: Option<&str>) -> String {
    value.unwrap_or("").to_string()
}

fn draft_optional_hex(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
}

fn draft_optional_u32(field: &str, value: &str) -> Result<Option<u32>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    trimmed
        .parse::<u32>()
        .map(Some)
        .map_err(|_| format!("invalid {field}: expected unsigned integer, got `{trimmed}`"))
}

fn draft_optional_u8(field: &str, value: &str) -> Result<Option<u8>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    trimmed
        .parse::<u8>()
        .map(Some)
        .map_err(|_| format!("invalid {field}: expected 0-255, got `{trimmed}`"))
}

/// 解析可选 image fit；空表示未设置（运行时默认 cover）。
fn draft_optional_image_fit(value: &str) -> Result<Option<String>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    crate::app::theme_catalog::PanelImageFit::parse(Some(trimmed))?;
    Ok(Some(trimmed.to_ascii_lowercase()))
}

/// 从已有定义进入编辑（id 只读）。
pub(crate) fn draft_from_definition(definition: &ThemeDefinition) -> ThemeEditorDraft {
    ThemeEditorDraft {
        id: definition.id.clone(),
        id_readonly: true,
        builtin_base: None,
        name: definition.name.clone(),
        tagline: definition.tagline.clone().unwrap_or_default(),
        background: definition.palette.background.clone(),
        text: definition.palette.text.clone(),
        primary: definition.palette.primary.clone(),
        success: definition.palette.success.clone(),
        warning: definition.palette.warning.clone(),
        danger: definition.palette.danger.clone(),
        cover_relative: definition.assets.cover.clone(),
        background_relative: definition.assets.background.clone(),
        chat_header_relative: definition.assets.chat_header.clone(),
        chat_body_relative: definition.assets.chat_body.clone(),
        chat_input_relative: definition.assets.chat_input.clone(),
        cover_source: None,
        background_source: None,
        chat_header_source: None,
        chat_body_source: None,
        chat_input_source: None,
        chrome_input_bg: optional_hex_to_draft(definition.chrome.input_bg.as_deref()),
        chrome_input_border: optional_hex_to_draft(definition.chrome.input_border.as_deref()),
        chrome_input_text: optional_hex_to_draft(definition.chrome.input_text.as_deref()),
        chrome_panel_bg: optional_hex_to_draft(definition.chrome.panel_bg.as_deref()),
        chrome_panel_border: optional_hex_to_draft(definition.chrome.panel_border.as_deref()),
        chrome_button_bg: optional_hex_to_draft(definition.chrome.button_bg.as_deref()),
        chrome_button_text: optional_hex_to_draft(definition.chrome.button_text.as_deref()),
        chrome_radius_px: definition.chrome.radius_px.map(|v| v.to_string()).unwrap_or_default(),
        chrome_panel_alpha: definition
            .chrome
            .panel_alpha
            .map(|v| v.to_string())
            .unwrap_or_default(),
        chrome_chat_header_alpha: definition
            .chrome
            .chat_header_alpha
            .map(|v| v.to_string())
            .unwrap_or_default(),
        chrome_chat_body_alpha: definition
            .chrome
            .chat_body_alpha
            .map(|v| v.to_string())
            .unwrap_or_default(),
        chrome_chat_input_alpha: definition
            .chrome
            .chat_input_alpha
            .map(|v| v.to_string())
            .unwrap_or_default(),
        chrome_chat_header_image_fit: definition
            .chrome
            .chat_header_image_fit
            .clone()
            .unwrap_or_default(),
        chrome_chat_body_image_fit: definition
            .chrome
            .chat_body_image_fit
            .clone()
            .unwrap_or_default(),
        chrome_chat_input_image_fit: definition
            .chrome
            .chat_input_image_fit
            .clone()
            .unwrap_or_default(),
    }
}

/// 从内置主题（合并覆盖层）进入编辑。
pub(crate) fn draft_from_builtin(
    builtin_name: &str,
    palette: &ThemePaletteHex,
    overlay: Option<&ThemeDefinition>,
) -> ThemeEditorDraft {
    let slug = crate::app::theme_catalog::builtin_overlay_slug(builtin_name);
    if let Some(definition) = overlay {
        let mut draft = draft_from_definition(definition);
        draft.id = slug;
        draft.builtin_base = Some(builtin_name.to_string());
        draft.name = builtin_name.to_string();
        draft
    } else {
        ThemeEditorDraft {
            id: slug,
            id_readonly: true,
            builtin_base: Some(builtin_name.to_string()),
            name: builtin_name.to_string(),
            tagline: String::new(),
            background: palette.background.clone(),
            text: palette.text.clone(),
            primary: palette.primary.clone(),
            success: palette.success.clone(),
            warning: palette.warning.clone(),
            danger: palette.danger.clone(),
            ..ThemeEditorDraft::default()
        }
    }
}

/// 校验草稿并构造可保存的主题定义。
pub(crate) fn try_definition_from_draft(
    draft: &ThemeEditorDraft,
) -> Result<ThemeDefinition, String> {
    let name = draft.name.trim();
    if name.is_empty() {
        return Err("主题名称不能为空".to_string());
    }
    let tagline = draft.tagline.trim();
    let mut definition = ThemeDefinition {
        id: draft.id.trim().to_string(),
        name: name.to_string(),
        tagline: if tagline.is_empty() { None } else { Some(tagline.to_string()) },
        palette: ThemePaletteHex {
            background: draft.background.trim().to_string(),
            text: draft.text.trim().to_string(),
            primary: draft.primary.trim().to_string(),
            success: draft.success.trim().to_string(),
            warning: draft.warning.trim().to_string(),
            danger: draft.danger.trim().to_string(),
        },
        assets: ThemeAssets {
            cover: draft.cover_relative.clone(),
            background: draft.background_relative.clone(),
            chat_header: draft.chat_header_relative.clone(),
            chat_body: draft.chat_body_relative.clone(),
            chat_input: draft.chat_input_relative.clone(),
        },
        chrome: ThemeChrome {
            input_bg: draft_optional_hex(&draft.chrome_input_bg),
            input_border: draft_optional_hex(&draft.chrome_input_border),
            input_text: draft_optional_hex(&draft.chrome_input_text),
            panel_bg: draft_optional_hex(&draft.chrome_panel_bg),
            panel_border: draft_optional_hex(&draft.chrome_panel_border),
            button_bg: draft_optional_hex(&draft.chrome_button_bg),
            button_text: draft_optional_hex(&draft.chrome_button_text),
            radius_px: draft_optional_u32("chrome.radius_px", &draft.chrome_radius_px)?,
            panel_alpha: draft_optional_u8("chrome.panel_alpha", &draft.chrome_panel_alpha)?,
            chat_header_alpha: draft_optional_u8(
                "chrome.chat_header_alpha",
                &draft.chrome_chat_header_alpha,
            )?,
            chat_body_alpha: draft_optional_u8(
                "chrome.chat_body_alpha",
                &draft.chrome_chat_body_alpha,
            )?,
            chat_input_alpha: draft_optional_u8(
                "chrome.chat_input_alpha",
                &draft.chrome_chat_input_alpha,
            )?,
            chat_header_image_fit: draft_optional_image_fit(&draft.chrome_chat_header_image_fit)?,
            chat_body_image_fit: draft_optional_image_fit(&draft.chrome_chat_body_image_fit)?,
            chat_input_image_fit: draft_optional_image_fit(&draft.chrome_chat_input_image_fit)?,
        },
    };
    definition.normalize();
    definition.validate().map_err(|err| err.to_string())?;
    Ok(definition)
}

/// 收集当前已占用的自定义主题 id（可选排除自身）。
pub(crate) fn existing_custom_ids(app: &App, exclude: Option<&str>) -> Vec<String> {
    app.themes_settings
        .entries
        .iter()
        .filter(|entry| {
            matches!(
                entry.source,
                crate::app::components::system_settings_themes::gallery::ThemeGallerySource::Custom
                    | crate::app::components::system_settings_themes::gallery::ThemeGallerySource::Preset
            )
        })
        .map(|entry| entry.id.clone())
        .filter(|id| exclude.is_none_or(|skip| id != skip))
        .collect()
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
    .padding([10, 0])
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
    let input = text_input(placeholder, value)
        .on_input(on_input)
        .padding([10, 12])
        .size(13)
        .style(settings_text_input_style)
        .width(Length::Fill);
    field_row(label, description, input)
}

fn parse_draft_hex(value: &str) -> Option<Color> {
    let trimmed = value.trim();
    parse_color(trimmed).or_else(|| {
        let hex = trimmed.trim_start_matches('#');
        if hex.len() != 6 {
            return None;
        }
        let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Color::from_rgb8(red, green, blue))
    })
}

fn format_theme_hex(color: Color) -> String {
    format!(
        "#{:02x}{:02x}{:02x}",
        (color.r * 255.0).round() as u8,
        (color.g * 255.0).round() as u8,
        (color.b * 255.0).round() as u8
    )
}

fn draft_hex_for_target(draft: &ThemeEditorDraft, target: ThemeColorPickerTarget) -> &str {
    match target {
        ThemeColorPickerTarget::Palette(ThemePaletteField::Background) => &draft.background,
        ThemeColorPickerTarget::Palette(ThemePaletteField::Text) => &draft.text,
        ThemeColorPickerTarget::Palette(ThemePaletteField::Primary) => &draft.primary,
        ThemeColorPickerTarget::Palette(ThemePaletteField::Success) => &draft.success,
        ThemeColorPickerTarget::Palette(ThemePaletteField::Warning) => &draft.warning,
        ThemeColorPickerTarget::Palette(ThemePaletteField::Danger) => &draft.danger,
        ThemeColorPickerTarget::Chrome(ThemeChromeField::InputBg) => &draft.chrome_input_bg,
        ThemeColorPickerTarget::Chrome(ThemeChromeField::InputBorder) => &draft.chrome_input_border,
        ThemeColorPickerTarget::Chrome(ThemeChromeField::InputText) => &draft.chrome_input_text,
        ThemeColorPickerTarget::Chrome(ThemeChromeField::PanelBg) => &draft.chrome_panel_bg,
        ThemeColorPickerTarget::Chrome(ThemeChromeField::PanelBorder) => &draft.chrome_panel_border,
        ThemeColorPickerTarget::Chrome(ThemeChromeField::ButtonBg) => &draft.chrome_button_bg,
        ThemeColorPickerTarget::Chrome(ThemeChromeField::ButtonText) => &draft.chrome_button_text,
        ThemeColorPickerTarget::Chrome(ThemeChromeField::RadiusPx)
        | ThemeColorPickerTarget::Chrome(ThemeChromeField::PanelAlpha)
        | ThemeColorPickerTarget::Chrome(ThemeChromeField::ChatHeaderAlpha)
        | ThemeColorPickerTarget::Chrome(ThemeChromeField::ChatBodyAlpha)
        | ThemeColorPickerTarget::Chrome(ThemeChromeField::ChatInputAlpha)
        | ThemeColorPickerTarget::Chrome(ThemeChromeField::ChatHeaderImageFit)
        | ThemeColorPickerTarget::Chrome(ThemeChromeField::ChatBodyImageFit)
        | ThemeColorPickerTarget::Chrome(ThemeChromeField::ChatInputImageFit) => "",
    }
}

fn color_picker_popover<'a>(
    app: &'a App,
    draft: &'a ThemeEditorDraft,
    target: ThemeColorPickerTarget,
) -> Element<'a, Message> {
    let current = parse_draft_hex(draft_hex_for_target(draft, target))
        .unwrap_or(Color::from_rgb8(0x38, 0xbd, 0xf8));
    let title = match target {
        ThemeColorPickerTarget::Palette(_) => "选择调色板颜色",
        ThemeColorPickerTarget::Chrome(_) => "选择控件颜色",
    };

    container(
        column![
            text(title).size(14),
            Space::new().height(8.0),
            render_color_picker(
                current,
                app.themes_settings.color_picker_format,
                false,
                |color| {
                    Message::Settings(SettingsMessage::Themes(
                        ThemesMessage::ColorPickerColorChanged(format_theme_hex(color)),
                    ))
                },
                |format| {
                    Message::Settings(SettingsMessage::Themes(
                        ThemesMessage::ColorPickerFormatChanged(format),
                    ))
                },
                || Message::None,
            ),
            Space::new().height(8.0),
            button(text("确定").size(13))
                .on_press(Message::Settings(SettingsMessage::Themes(
                    ThemesMessage::ColorPickerClosed,
                )))
                .padding([6, 16])
                .style(primary_action_btn_style),
        ]
        .spacing(4),
    )
    .width(Length::Fixed(280.0))
    .padding(16)
    .style(|theme: &Theme| {
        let mut style = settings_panel_style(theme);
        style.border.radius = 20.0.into();
        style
    })
    .into()
}

fn color_field_row<'a>(
    app: &'a App,
    draft: &'a ThemeEditorDraft,
    label: &'static str,
    description: &'static str,
    target: ThemeColorPickerTarget,
    value: &'a str,
    placeholder: &'static str,
    on_manual: impl Fn(String) -> ThemesMessage + 'a,
) -> Element<'a, Message> {
    let preview = parse_draft_hex(value).unwrap_or(Color::from_rgb8(0x88, 0x88, 0x88));
    let swatch = button(container(Space::new().width(18).height(18)).style(move |_: &Theme| {
        iced::widget::container::Style {
            background: Some(Background::Color(preview)),
            border: iced::Border {
                color: Color::from_rgb(0.55, 0.55, 0.55),
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        }
    }))
    .padding(4)
    .on_press(Message::Settings(SettingsMessage::Themes(ThemesMessage::ColorPickerToggled(target))))
    .style(rounded_action_btn_style);

    let open = app.themes_settings.color_picker_target == Some(target);
    let swatch = BelowOverlay::new(swatch, color_picker_popover(app, draft, target))
        .show(open)
        .gap(6.0)
        .on_close(Message::Settings(SettingsMessage::Themes(ThemesMessage::ColorPickerClosed)));

    let input = text_input(placeholder, value)
        .on_input(move |value| Message::Settings(SettingsMessage::Themes(on_manual(value))))
        .padding([10, 12])
        .size(13)
        .style(settings_text_input_style)
        .width(Length::Fill);

    let control = row![swatch, input].spacing(8).align_y(Alignment::Center);
    field_row(label, description, control)
}

fn palette_row<'a>(
    app: &'a App,
    draft: &'a ThemeEditorDraft,
    label: &'static str,
    description: &'static str,
    field: ThemePaletteField,
    value: &'a str,
) -> Element<'a, Message> {
    color_field_row(
        app,
        draft,
        label,
        description,
        ThemeColorPickerTarget::Palette(field),
        value,
        "#rrggbb",
        move |value| ThemesMessage::DraftPaletteChanged(field, value),
    )
}

fn chrome_color_row<'a>(
    app: &'a App,
    draft: &'a ThemeEditorDraft,
    label: &'static str,
    description: &'static str,
    field: ThemeChromeField,
    value: &'a str,
) -> Element<'a, Message> {
    color_field_row(
        app,
        draft,
        label,
        description,
        ThemeColorPickerTarget::Chrome(field),
        value,
        "#rrggbb",
        move |value| ThemesMessage::DraftChromeChanged(field, value),
    )
}

fn chrome_row<'a>(
    label: &'static str,
    description: &'static str,
    field: ThemeChromeField,
    value: &'a str,
    placeholder: &'static str,
) -> Element<'a, Message> {
    text_row(label, description, placeholder, value, move |value| {
        Message::Settings(SettingsMessage::Themes(ThemesMessage::DraftChromeChanged(field, value)))
    })
}

fn asset_status_label(relative: Option<&str>, source: Option<&str>) -> String {
    if let Some(source) = source.filter(|s| !s.trim().is_empty()) {
        let name =
            std::path::Path::new(source).file_name().and_then(|n| n.to_str()).unwrap_or(source);
        return format!("待复制：{name}");
    }
    if let Some(relative) = relative.filter(|s| !s.is_empty()) {
        return format!("已入库：{relative}");
    }
    "未选择".to_string()
}

fn asset_picker_row<'a>(
    label: &'static str,
    description: &'static str,
    relative: Option<&'a str>,
    source: Option<&'a str>,
    pick: ThemesMessage,
    clear: ThemesMessage,
) -> Element<'a, Message> {
    let status =
        text(asset_status_label(relative, source)).size(12).style(settings_muted_text_style);
    let controls = row![
        status,
        button(text("选择图片").size(12))
            .padding([8, 12])
            .on_press(Message::Settings(SettingsMessage::Themes(pick)))
            .style(rounded_action_btn_style),
        button(text("清除").size(12))
            .padding([8, 12])
            .on_press(Message::Settings(SettingsMessage::Themes(clear)))
            .style(rounded_action_btn_style),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    field_row(label, description, controls)
}

fn zone_image_fit_pick_list<'a>(value: &'a str, field: ThemeChromeField) -> Element<'a, Message> {
    let selected = PanelImageFit::parse(Some(value)).unwrap_or_default();
    pick_list(PanelImageFit::ALL, Some(selected), move |fit| {
        Message::Settings(SettingsMessage::Themes(ThemesMessage::DraftChromeChanged(
            field,
            fit.as_str().to_string(),
        )))
    })
    .padding([8, 12])
    .text_size(13)
    .width(Length::Fill)
    .style(settings_pick_list_style)
    .menu_style(settings_pick_list_menu_style)
    .into()
}

/// 窗口背景卡：选图 + 默认透明度（分区未单独设置时回退）。
fn background_asset_card(draft: &ThemeEditorDraft) -> Element<'_, Message> {
    column![
        settings_section_card("窗口背景", "主窗口壁纸；默认透明度供分区未单独设置时回退。",),
        asset_picker_row(
            "背景图",
            "主窗口壁纸资源，保存后由应用层加载。",
            draft.background_relative.as_deref(),
            draft.background_source.as_deref(),
            ThemesMessage::PickBackground,
            ThemesMessage::ClearBackground,
        ),
        chrome_row(
            "默认透明度",
            "0–255；分区未单独设置时回退到此值；有壁纸/分区图时留空使用默认半透明。",
            ThemeChromeField::PanelAlpha,
            &draft.chrome_panel_alpha,
            "例如 184",
        ),
    ]
    .spacing(4)
    .into()
}

/// 分区资源卡：选图 + 该区透明度 + 该区缩放下拉。
fn zone_asset_card<'a>(
    title: &'static str,
    description: &'static str,
    pick_label: &'static str,
    pick_description: &'static str,
    relative: Option<&'a str>,
    source: Option<&'a str>,
    pick: ThemesMessage,
    clear: ThemesMessage,
    alpha_field: ThemeChromeField,
    alpha_value: &'a str,
    fit_field: ThemeChromeField,
    fit_value: &'a str,
) -> Element<'a, Message> {
    column![
        settings_section_card(title, description),
        asset_picker_row(pick_label, pick_description, relative, source, pick, clear,),
        chrome_row("透明度", "0–255；仅本分区。", alpha_field, alpha_value, "例如 184",),
        field_row(
            "图缩放",
            "本分区背景图 ContentFit；默认覆盖裁剪。",
            zone_image_fit_pick_list(fit_value, fit_field),
        ),
    ]
    .spacing(4)
    .into()
}

/// 主题编辑器分段 Tab 标签（与现有 section 分组一致）。
pub(crate) fn editor_tab_labels() -> &'static [(&'static str, ThemeEditorTab)] {
    &[
        ("基本信息", ThemeEditorTab::Basics),
        ("调色板", ThemeEditorTab::Palette),
        ("资源", ThemeEditorTab::Assets),
        ("控件皮肤", ThemeEditorTab::Chrome),
        ("预览", ThemeEditorTab::Preview),
    ]
}

fn editor_tab_button(
    label: &'static str,
    tab: ThemeEditorTab,
    active_tab: ThemeEditorTab,
) -> Element<'static, Message> {
    let is_active = tab == active_tab;
    button(text(label).size(13))
        .padding([8, 14])
        .on_press(Message::Settings(SettingsMessage::Themes(ThemesMessage::EditorTabSelected(tab))))
        .style(move |theme: &Theme, status| settings_segment_button_style(theme, status, is_active))
        .into()
}

fn basics_tab(draft: &ThemeEditorDraft) -> Element<'_, Message> {
    let id_hint = if draft.builtin_base.is_some() {
        "内置覆盖层标识由主题名派生，只读"
    } else if draft.id_readonly {
        "编辑已有主题时标识只读"
    } else {
        "由名称自动生成；冲突时追加后缀"
    };

    column![
        settings_section_card("基本信息", "名称必填；新建时标识由名称生成。"),
        field_row(
            "标识",
            id_hint,
            text_input("", &draft.id)
                .padding([10, 12])
                .size(13)
                .style(settings_text_input_style)
                .width(Length::Fill),
        ),
        text_row(
            "名称",
            "显示在画廊与主题选择中。",
            "例如 Ocean Breeze",
            &draft.name,
            |value| {
                Message::Settings(SettingsMessage::Themes(ThemesMessage::DraftNameChanged(value)))
            },
        ),
        text_row(
            "标语",
            "可选的一句话描述。",
            "例如 Soft blue accents",
            &draft.tagline,
            |value| {
                Message::Settings(SettingsMessage::Themes(ThemesMessage::DraftTaglineChanged(
                    value,
                )))
            },
        ),
    ]
    .spacing(4)
    .into()
}

fn palette_tab<'a>(app: &'a App, draft: &'a ThemeEditorDraft) -> Element<'a, Message> {
    column![
        settings_section_card("调色板", "仅支持 #rrggbb 十六进制颜色。"),
        palette_row(
            app,
            draft,
            "背景",
            "面板与页面背景色。",
            ThemePaletteField::Background,
            &draft.background,
        ),
        palette_row(
            app,
            draft,
            "文本",
            "主文字颜色，需与背景对比充足。",
            ThemePaletteField::Text,
            &draft.text,
        ),
        palette_row(
            app,
            draft,
            "主色",
            "强调按钮与链接。",
            ThemePaletteField::Primary,
            &draft.primary,
        ),
        palette_row(
            app,
            draft,
            "成功",
            "成功状态标签。",
            ThemePaletteField::Success,
            &draft.success,
        ),
        palette_row(
            app,
            draft,
            "警告",
            "警告状态标签。",
            ThemePaletteField::Warning,
            &draft.warning,
        ),
        palette_row(
            app,
            draft,
            "危险",
            "危险/错误状态标签。",
            ThemePaletteField::Danger,
            &draft.danger,
        ),
    ]
    .spacing(4)
    .into()
}

fn assets_tab(draft: &ThemeEditorDraft) -> Element<'_, Message> {
    column![
        settings_section_card("资源", "封面用于画廊；背景与聊天分区图供运行时铺图。"),
        asset_picker_row(
            "封面",
            "画廊卡片缩略图，支持 png/jpg/jpeg/webp。",
            draft.cover_relative.as_deref(),
            draft.cover_source.as_deref(),
            ThemesMessage::PickCover,
            ThemesMessage::ClearCover,
        ),
        background_asset_card(draft),
        zone_asset_card(
            "聊头分区",
            "聊天顶栏选图、透明度与缩放。",
            "聊头背景",
            "聊天顶栏分区图，叠在窗口壁纸之上。",
            draft.chat_header_relative.as_deref(),
            draft.chat_header_source.as_deref(),
            ThemesMessage::PickChatHeader,
            ThemesMessage::ClearChatHeader,
            ThemeChromeField::ChatHeaderAlpha,
            &draft.chrome_chat_header_alpha,
            ThemeChromeField::ChatHeaderImageFit,
            &draft.chrome_chat_header_image_fit,
        ),
        zone_asset_card(
            "消息区分区",
            "消息列表选图、透明度与缩放。",
            "消息区背景",
            "消息列表分区图。",
            draft.chat_body_relative.as_deref(),
            draft.chat_body_source.as_deref(),
            ThemesMessage::PickChatBody,
            ThemesMessage::ClearChatBody,
            ThemeChromeField::ChatBodyAlpha,
            &draft.chrome_chat_body_alpha,
            ThemeChromeField::ChatBodyImageFit,
            &draft.chrome_chat_body_image_fit,
        ),
        zone_asset_card(
            "输入区分区",
            "输入卡选图、透明度与缩放。",
            "输入区背景",
            "输入卡分区图。",
            draft.chat_input_relative.as_deref(),
            draft.chat_input_source.as_deref(),
            ThemesMessage::PickChatInput,
            ThemesMessage::ClearChatInput,
            ThemeChromeField::ChatInputAlpha,
            &draft.chrome_chat_input_alpha,
            ThemeChromeField::ChatInputImageFit,
            &draft.chrome_chat_input_image_fit,
        ),
    ]
    .spacing(4)
    .into()
}

fn chrome_tab<'a>(app: &'a App, draft: &'a ThemeEditorDraft) -> Element<'a, Message> {
    column![
        settings_section_card("控件皮肤", "可选覆盖；非法 hex 会阻止保存。"),
        chrome_color_row(
            app,
            draft,
            "输入背景",
            "可选；留空使用默认。",
            ThemeChromeField::InputBg,
            &draft.chrome_input_bg,
        ),
        chrome_color_row(
            app,
            draft,
            "输入边框",
            "可选。",
            ThemeChromeField::InputBorder,
            &draft.chrome_input_border,
        ),
        chrome_color_row(
            app,
            draft,
            "输入文字",
            "可选；未填时回退调色板文字色。",
            ThemeChromeField::InputText,
            &draft.chrome_input_text,
        ),
        chrome_color_row(
            app,
            draft,
            "面板背景",
            "可选。",
            ThemeChromeField::PanelBg,
            &draft.chrome_panel_bg,
        ),
        chrome_color_row(
            app,
            draft,
            "面板边框",
            "可选。",
            ThemeChromeField::PanelBorder,
            &draft.chrome_panel_border,
        ),
        chrome_color_row(
            app,
            draft,
            "按钮背景",
            "可选。",
            ThemeChromeField::ButtonBg,
            &draft.chrome_button_bg,
        ),
        chrome_color_row(
            app,
            draft,
            "按钮文字",
            "可选。",
            ThemeChromeField::ButtonText,
            &draft.chrome_button_text,
        ),
        chrome_row(
            "圆角",
            "像素；留空使用默认。",
            ThemeChromeField::RadiusPx,
            &draft.chrome_radius_px,
            "例如 10",
        ),
    ]
    .spacing(4)
    .into()
}

fn preview_tab(draft: &ThemeEditorDraft) -> Element<'_, Message> {
    column![
        settings_section_card("预览", "暗黑草稿下请确保文字可读。"),
        theme_preview_panel(draft),
    ]
    .spacing(4)
    .into()
}

fn editor_tab_content<'a>(
    app: &'a App,
    draft: &'a ThemeEditorDraft,
    active_tab: ThemeEditorTab,
) -> Element<'a, Message> {
    match active_tab {
        ThemeEditorTab::Basics => basics_tab(draft),
        ThemeEditorTab::Palette => palette_tab(app, draft),
        ThemeEditorTab::Assets => assets_tab(draft),
        ThemeEditorTab::Chrome => chrome_tab(app, draft),
        ThemeEditorTab::Preview => preview_tab(draft),
    }
}

/// 渲染主题编辑器视图。
pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let Some(draft) = app.themes_settings.draft.as_ref() else {
        return column![
            settings_page_intro("主题编辑器", "当前没有可编辑的草稿。"),
            button(text("返回画廊").size(12))
                .padding([8, 14])
                .on_press(Message::Settings(SettingsMessage::Themes(ThemesMessage::CancelEdit,)))
                .style(rounded_action_btn_style),
        ]
        .spacing(12)
        .into();
    };

    let title = if draft.builtin_base.is_some() {
        "编辑内置主题"
    } else if draft.id_readonly {
        "编辑主题"
    } else {
        "新建主题"
    };
    let is_bundled_preset = is_bundled_preset_id(&draft.id);

    let mut actions = row![
        button(text("保存").size(12))
            .padding([8, 16])
            .on_press(Message::Settings(SettingsMessage::Themes(ThemesMessage::SaveDraft,)))
            .style(primary_action_btn_style),
        button(text("取消").size(12))
            .padding([8, 16])
            .on_press(Message::Settings(SettingsMessage::Themes(ThemesMessage::CancelEdit,)))
            .style(rounded_action_btn_style),
    ]
    .spacing(8);

    if let Some(builtin_name) = draft.builtin_base.clone() {
        actions = actions.push(
            button(text("恢复默认").size(12))
                .padding([8, 16])
                .on_press(Message::Settings(SettingsMessage::Themes(
                    ThemesMessage::RestoreBuiltinDefault(builtin_name),
                )))
                .style(crate::app::components::system_settings_common::danger_action_btn_style),
        );
    }
    if is_bundled_preset {
        let preset_id = draft.id.clone();
        actions = actions.push(
            button(text("恢复默认").size(12))
                .padding([8, 16])
                .on_press(Message::Settings(SettingsMessage::Themes(
                    ThemesMessage::RestorePresetDefault(preset_id),
                )))
                .style(crate::app::components::system_settings_common::danger_action_btn_style),
        );
    }

    let intro = if draft.builtin_base.is_some() {
        "修改将保存为本地覆盖层，可随时恢复 Iced 内置默认值。"
    } else if is_bundled_preset {
        "此预设可编辑并随时恢复嵌入默认值，但不可删除。"
    } else {
        "按 Tab 分段编辑命名、配色、资源与控件皮肤，预览后保存到本地仓库。"
    };

    let tabs = editor_tab_labels().iter().fold(
        row![].spacing(8).align_y(Alignment::Center),
        |row, (label, tab)| {
            row.push(editor_tab_button(label, *tab, app.themes_settings.editor_tab))
        },
    );

    let mut body = column![
        settings_page_intro(title, intro),
        tabs,
        editor_tab_content(app, draft, app.themes_settings.editor_tab),
        actions,
    ]
    .spacing(12);

    if let Some(error) = app.themes_settings.error.as_deref() {
        body = body.push(settings_error_banner(error));
    }

    settings_section_content(
        scrollable(body.width(Length::Fill))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
            .height(Length::Fill),
    )
    .into()
}

#[cfg(test)]
#[path = "editor_tests.rs"]
mod editor_tests;
