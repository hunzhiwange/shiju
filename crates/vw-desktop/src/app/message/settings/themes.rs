//! 主题设置页消息：画廊应用/删除与编辑器草稿保存。

use std::path::PathBuf;

use crate::app::components::system_settings_themes::editor::{
    allocate_unique_theme_id, draft_from_builtin, draft_from_definition, existing_custom_ids,
    new_theme_draft, slugify_theme_id, try_definition_from_draft,
};
use crate::app::components::system_settings_themes::gallery::{
    gallery_page_count, palette_hex_from_theme, refresh_gallery_state,
};
use crate::app::message::view::{self, ViewMessage};
use crate::app::state::{ThemeEditorDraft, ThemeEditorTab, ThemesSettingsMode};
use crate::app::theme_catalog::{
    ActiveThemeSkin, ThemeAssetSources, ThemeAssets, ThemeChrome, ThemeDefinition,
    builtin_overlay_exists, builtin_overlay_slug, clear_active_theme_skin, current_themes_root,
    delete_builtin_overlay, ensure_bundled_presets, export_theme_definition_to_pack,
    export_theme_dir_to_pack, find_builtin_theme, import_theme_pack_into_store,
    is_bundled_preset_id, load_builtin_overlay, overlay_theme_store, resolve_app_theme_name,
    resolve_builtin_with_overlay, restore_bundled_preset, save_builtin_overlay_with_sources,
    sync_active_theme_skin, theme_store_from_home, to_iced_theme,
};
use crate::app::{App, Message};
use iced::Task;
use iced::Theme;

use super::messages::SettingsMessage;

pub use super::messages::{
    ThemeChromeField, ThemeColorPickerTarget, ThemePaletteField, ThemesMessage,
};

fn apply_theme(app: &mut App, theme: Theme) -> Task<Message> {
    view::update(app, ViewMessage::AppThemeSelected(theme))
}

fn refresh(app: &mut App) -> Task<Message> {
    let mut toast = Task::none();
    if let Some(root) = current_themes_root() {
        if let Err(err) = ensure_bundled_presets(&root) {
            tracing::warn!(
                target: "vw_desktop",
                error = %err,
                "failed to seed bundled theme presets"
            );
            toast = app.show_warning_toast(format!("内置预设主题同步失败: {err}"));
        }
    }
    refresh_gallery_state(app);
    toast
}

fn clear_editor(app: &mut App) {
    app.themes_settings.mode = ThemesSettingsMode::List;
    app.themes_settings.editor_tab = ThemeEditorTab::Basics;
    app.themes_settings.draft = None;
    app.themes_settings.color_picker_target = None;
    app.themes_settings.color_picker_format =
        crate::app::components::color_picker::ColorFormat::Hex;
}

fn sync_overlay_skin(app: &mut App, builtin_name: &str) {
    let Some(root) = current_themes_root() else {
        clear_active_theme_skin(app);
        return;
    };
    match load_builtin_overlay(&root, builtin_name) {
        Ok(Some(definition)) => {
            let store = overlay_theme_store(&root);
            app.active_theme_skin = Some(ActiveThemeSkin::from_definition(&store, &definition));
        }
        Ok(None) => clear_active_theme_skin(app),
        Err(err) => {
            tracing::warn!(
                target: "vw_desktop",
                theme = %builtin_name,
                error = %err,
                "failed to load builtin overlay skin"
            );
            clear_active_theme_skin(app);
        }
    }
}

fn is_active_builtin(app: &App, builtin_name: &str) -> bool {
    app.active_custom_theme_id.is_none() && app.app_theme.to_string() == builtin_name
}

fn set_draft_palette(draft: &mut ThemeEditorDraft, field: ThemePaletteField, value: String) {
    match field {
        ThemePaletteField::Background => draft.background = value,
        ThemePaletteField::Text => draft.text = value,
        ThemePaletteField::Primary => draft.primary = value,
        ThemePaletteField::Success => draft.success = value,
        ThemePaletteField::Warning => draft.warning = value,
        ThemePaletteField::Danger => draft.danger = value,
    }
}

fn set_draft_chrome(draft: &mut ThemeEditorDraft, field: ThemeChromeField, value: String) {
    match field {
        ThemeChromeField::InputBg => draft.chrome_input_bg = value,
        ThemeChromeField::InputBorder => draft.chrome_input_border = value,
        ThemeChromeField::InputText => draft.chrome_input_text = value,
        ThemeChromeField::PanelBg => draft.chrome_panel_bg = value,
        ThemeChromeField::PanelBorder => draft.chrome_panel_border = value,
        ThemeChromeField::ButtonBg => draft.chrome_button_bg = value,
        ThemeChromeField::ButtonText => draft.chrome_button_text = value,
        ThemeChromeField::RadiusPx => draft.chrome_radius_px = value,
        ThemeChromeField::PanelAlpha => draft.chrome_panel_alpha = value,
        ThemeChromeField::ChatHeaderAlpha => draft.chrome_chat_header_alpha = value,
        ThemeChromeField::ChatBodyAlpha => draft.chrome_chat_body_alpha = value,
        ThemeChromeField::ChatInputAlpha => draft.chrome_chat_input_alpha = value,
        ThemeChromeField::ChatHeaderImageFit => draft.chrome_chat_header_image_fit = value,
        ThemeChromeField::ChatBodyImageFit => draft.chrome_chat_body_image_fit = value,
        ThemeChromeField::ChatInputImageFit => draft.chrome_chat_input_image_fit = value,
    }
}

fn apply_color_picker_hex(
    draft: &mut ThemeEditorDraft,
    target: ThemeColorPickerTarget,
    hex: String,
) {
    match target {
        ThemeColorPickerTarget::Palette(field) => set_draft_palette(draft, field, hex),
        ThemeColorPickerTarget::Chrome(field) => set_draft_chrome(draft, field, hex),
    }
}

fn reset_color_picker(app: &mut App) {
    app.themes_settings.color_picker_target = None;
    app.themes_settings.color_picker_format =
        crate::app::components::color_picker::ColorFormat::Hex;
}

fn sync_new_theme_id(app: &mut App) {
    let (name, id_readonly) = match app.themes_settings.draft.as_ref() {
        Some(draft) => (draft.name.clone(), draft.id_readonly),
        None => return,
    };
    if id_readonly {
        return;
    }
    let existing = existing_custom_ids(app, None);
    let base = slugify_theme_id(&name);
    let id = allocate_unique_theme_id(&base, &existing);
    if let Some(draft) = app.themes_settings.draft.as_mut() {
        draft.id = id;
    }
}

fn pick_image_task(on_picked: fn(Option<String>) -> ThemesMessage) -> Task<Message> {
    Task::perform(
        async {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let handle = rfd::AsyncFileDialog::new()
                    .add_filter("Image", &["png", "jpg", "jpeg", "webp"])
                    .pick_file()
                    .await;
                handle.map(|f| f.path().to_string_lossy().into_owned())
            }
            #[cfg(target_arch = "wasm32")]
            {
                None
            }
        },
        move |picked| Message::Settings(SettingsMessage::Themes(on_picked(picked))),
    )
}

fn pick_theme_pack_task() -> Task<Message> {
    Task::perform(
        async {
            #[cfg(not(target_arch = "wasm32"))]
            {
                rfd::AsyncFileDialog::new()
                    .add_filter("VibeWindow Theme", &["vwtheme", "zip"])
                    .pick_file()
                    .await
                    .map(|file| file.path().to_string_lossy().into_owned())
            }
            #[cfg(target_arch = "wasm32")]
            {
                None
            }
        },
        |picked| {
            Message::Settings(SettingsMessage::Themes(ThemesMessage::ImportPackPicked(picked)))
        },
    )
}

fn save_theme_pack_task(theme_id: String) -> Task<Message> {
    let default_id = if find_builtin_theme(&theme_id).is_some() {
        builtin_overlay_slug(&theme_id)
    } else {
        theme_id.clone()
    };
    Task::perform(
        async move {
            #[cfg(not(target_arch = "wasm32"))]
            {
                rfd::AsyncFileDialog::new()
                    .add_filter("VibeWindow Theme", &["vwtheme"])
                    .set_file_name(format!("{default_id}.vwtheme"))
                    .save_file()
                    .await
                    .map(|file| file.path().to_string_lossy().into_owned())
            }
            #[cfg(target_arch = "wasm32")]
            {
                None
            }
        },
        move |path| {
            Message::Settings(SettingsMessage::Themes(ThemesMessage::ExportPackPicked {
                theme_id: theme_id.clone(),
                path,
            }))
        },
    )
}

fn export_theme_pack(theme_id: &str, pack_path: &str) -> Result<(), String> {
    if let Some(builtin) = find_builtin_theme(theme_id) {
        if let Some(root) = current_themes_root() {
            if builtin_overlay_exists(&root, theme_id) {
                let theme_dir = root.join("overlays").join(builtin_overlay_slug(theme_id));
                return export_theme_dir_to_pack(theme_dir, pack_path)
                    .map_err(|err| err.to_string());
            }
        }

        let definition = ThemeDefinition {
            id: builtin_overlay_slug(theme_id),
            name: theme_id.to_string(),
            tagline: None,
            palette: palette_hex_from_theme(builtin),
            assets: ThemeAssets::default(),
            chrome: ThemeChrome::default(),
        };
        return export_theme_definition_to_pack(&definition, pack_path)
            .map_err(|err| err.to_string());
    }

    let store = theme_store_from_home().ok_or_else(|| "无法定位主题仓库".to_string())?;
    let definition =
        store.load(theme_id).map_err(|err| format!("加载主题 `{theme_id}` 失败: {err}"))?;
    let theme_dir = store
        .theme_asset_path(&definition.id, "theme.json")
        .parent()
        .map(PathBuf::from)
        .ok_or_else(|| format!("无法定位主题 `{theme_id}` 的目录"))?;
    export_theme_dir_to_pack(theme_dir, pack_path).map_err(|err| err.to_string())
}

fn apply_cover_picked(draft: &mut ThemeEditorDraft, path: Option<String>) {
    if let Some(path) = path.filter(|p| !p.trim().is_empty()) {
        draft.cover_source = Some(path);
    }
}

fn apply_background_picked(draft: &mut ThemeEditorDraft, path: Option<String>) {
    if let Some(path) = path.filter(|p| !p.trim().is_empty()) {
        draft.background_source = Some(path);
    }
}

fn apply_chat_header_picked(draft: &mut ThemeEditorDraft, path: Option<String>) {
    if let Some(path) = path.filter(|p| !p.trim().is_empty()) {
        draft.chat_header_source = Some(path);
    }
}

fn apply_chat_body_picked(draft: &mut ThemeEditorDraft, path: Option<String>) {
    if let Some(path) = path.filter(|p| !p.trim().is_empty()) {
        draft.chat_body_source = Some(path);
    }
}

fn apply_chat_input_picked(draft: &mut ThemeEditorDraft, path: Option<String>) {
    if let Some(path) = path.filter(|p| !p.trim().is_empty()) {
        draft.chat_input_source = Some(path);
    }
}

fn clear_cover(draft: &mut ThemeEditorDraft) {
    draft.cover_source = None;
    draft.cover_relative = None;
}

fn clear_background(draft: &mut ThemeEditorDraft) {
    draft.background_source = None;
    draft.background_relative = None;
}

fn clear_chat_header(draft: &mut ThemeEditorDraft) {
    draft.chat_header_source = None;
    draft.chat_header_relative = None;
}

fn clear_chat_body(draft: &mut ThemeEditorDraft) {
    draft.chat_body_source = None;
    draft.chat_body_relative = None;
}

fn clear_chat_input(draft: &mut ThemeEditorDraft) {
    draft.chat_input_source = None;
    draft.chat_input_relative = None;
}

fn asset_sources_from_draft(draft: &ThemeEditorDraft) -> ThemeAssetSources {
    ThemeAssetSources {
        cover: draft.cover_source.as_ref().map(PathBuf::from),
        background: draft.background_source.as_ref().map(PathBuf::from),
        chat_header: draft.chat_header_source.as_ref().map(PathBuf::from),
        chat_body: draft.chat_body_source.as_ref().map(PathBuf::from),
        chat_input: draft.chat_input_source.as_ref().map(PathBuf::from),
    }
}

/// 处理主题页消息。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::Themes(message) = message else {
        return Task::none();
    };

    match message {
        ThemesMessage::Refresh => refresh(app),
        ThemesMessage::GalleryPrevPage => {
            app.themes_settings.page_index = app.themes_settings.page_index.saturating_sub(1);
            Task::none()
        }
        ThemesMessage::GalleryNextPage => {
            let max_page = gallery_page_count(app.themes_settings.entries.len()).saturating_sub(1);
            app.themes_settings.page_index = (app.themes_settings.page_index + 1).min(max_page);
            Task::none()
        }
        ThemesMessage::ImportPack => pick_theme_pack_task(),
        ThemesMessage::ImportPackPicked(None) => Task::none(),
        ThemesMessage::ImportPackPicked(Some(path)) => {
            let Some(store) = theme_store_from_home() else {
                app.themes_settings.error = Some("无法定位主题仓库".to_string());
                return app.show_error_toast("无法定位主题仓库");
            };
            match import_theme_pack_into_store(&path, &store) {
                Ok(installed) => {
                    refresh_gallery_state(app);
                    if let Some(error) = app.themes_settings.error.clone() {
                        return app.show_error_toast(format!(
                            "主题包已导入为 `{}`，但刷新画廊失败: {error}",
                            installed.id
                        ));
                    }
                    app.show_success_toast(format!("已导入主题包为 `{}`", installed.id))
                }
                Err(err) => {
                    let message = format!("导入主题包失败: {err}");
                    app.themes_settings.error = Some(message.clone());
                    app.show_error_toast(message)
                }
            }
        }
        ThemesMessage::ExportPack(theme_id) => save_theme_pack_task(theme_id),
        ThemesMessage::ExportPackPicked { theme_id: _, path: None } => Task::none(),
        ThemesMessage::ExportPackPicked { theme_id, path: Some(path) } => {
            match export_theme_pack(&theme_id, &path) {
                Ok(()) => {
                    app.themes_settings.error = None;
                    app.show_success_toast(format!("已导出主题包 `{theme_id}`"))
                }
                Err(err) => {
                    let message = format!("导出主题包 `{theme_id}` 失败: {err}");
                    app.themes_settings.error = Some(message.clone());
                    app.show_error_toast(message)
                }
            }
        }
        ThemesMessage::ApplyBuiltin(name) => {
            let Some(builtin) = find_builtin_theme(&name).cloned() else {
                app.themes_settings.error = Some(format!("未找到内置主题 `{name}`"));
                return app.show_error_toast(format!("未找到内置主题 `{name}`"));
            };
            let root = current_themes_root();
            let theme = match resolve_builtin_with_overlay(&builtin, root.as_deref()) {
                Ok(theme) => theme,
                Err(err) => {
                    let message = format!("应用内置主题覆盖失败: {err}");
                    app.themes_settings.error = Some(message.clone());
                    return app.show_error_toast(message);
                }
            };
            app.active_custom_theme_id = None;
            let task = apply_theme(app, theme);
            sync_overlay_skin(app, &name);
            refresh_gallery_state(app);
            Task::batch([task, app.show_success_toast(format!("已应用主题 {name}"))])
        }
        ThemesMessage::ApplyCustom(id) => {
            let Some(store) = theme_store_from_home() else {
                app.themes_settings.error = Some("无法定位主题仓库".to_string());
                return app.show_error_toast("无法定位主题仓库");
            };
            let definition = match store.load(&id) {
                Ok(definition) => definition,
                Err(err) => {
                    let message = format!("加载主题 `{id}` 失败: {err}");
                    app.themes_settings.error = Some(message.clone());
                    return app.show_error_toast(message);
                }
            };
            let theme = match to_iced_theme(&definition) {
                Ok(theme) => theme,
                Err(err) => {
                    let message = format!("转换主题 `{id}` 失败: {err}");
                    app.themes_settings.error = Some(message.clone());
                    return app.show_error_toast(message);
                }
            };
            app.active_custom_theme_id = Some(id.clone());
            sync_active_theme_skin(app);
            let task = apply_theme(app, theme);
            refresh_gallery_state(app);
            Task::batch([task, app.show_success_toast(format!("已应用主题 {}", definition.name))])
        }
        ThemesMessage::DeleteCustom(id) => {
            if find_builtin_theme(&id).is_some() {
                let message = format!("内置主题 `{id}` 不可删除");
                app.themes_settings.error = Some(message.clone());
                return app.show_error_toast(message);
            }
            if is_bundled_preset_id(&id) {
                let message = format!("内置预设主题 `{id}` 不可删除");
                app.themes_settings.error = Some(message.clone());
                return app.show_error_toast(message);
            }
            let Some(store) = theme_store_from_home() else {
                app.themes_settings.error = Some("无法定位主题仓库".to_string());
                return app.show_error_toast("无法定位主题仓库");
            };

            let mut tasks = Vec::new();
            if app.active_custom_theme_id.as_deref() == Some(id.as_str()) {
                app.active_custom_theme_id = None;
                clear_active_theme_skin(app);
                tasks.push(apply_theme(app, Theme::Light));
            }

            if let Err(err) = store.delete(&id) {
                let message = format!("删除主题 `{id}` 失败: {err}");
                app.themes_settings.error = Some(message.clone());
                return app.show_error_toast(message);
            }

            refresh_gallery_state(app);
            tasks.push(app.show_success_toast(format!("已删除主题 `{id}`")));
            Task::batch(tasks)
        }
        ThemesMessage::NewThemeRequested => {
            refresh_gallery_state(app);
            let existing = existing_custom_ids(app, None);
            app.themes_settings.draft = Some(new_theme_draft(&existing));
            app.themes_settings.mode = ThemesSettingsMode::Edit;
            app.themes_settings.editor_tab = ThemeEditorTab::Basics;
            app.themes_settings.error = None;
            reset_color_picker(app);
            Task::none()
        }
        ThemesMessage::EditCustom(id) => {
            let Some(store) = theme_store_from_home() else {
                app.themes_settings.error = Some("无法定位主题仓库".to_string());
                return app.show_error_toast("无法定位主题仓库");
            };
            let definition = match store.load(&id) {
                Ok(definition) => definition,
                Err(err) => {
                    let message = format!("加载主题 `{id}` 失败: {err}");
                    app.themes_settings.error = Some(message.clone());
                    return app.show_error_toast(message);
                }
            };
            app.themes_settings.draft = Some(draft_from_definition(&definition));
            app.themes_settings.mode = ThemesSettingsMode::Edit;
            app.themes_settings.editor_tab = ThemeEditorTab::Basics;
            app.themes_settings.error = None;
            reset_color_picker(app);
            Task::none()
        }
        ThemesMessage::EditBuiltin(name) => {
            let Some(builtin) = find_builtin_theme(&name) else {
                app.themes_settings.error = Some(format!("未找到内置主题 `{name}`"));
                return app.show_error_toast(format!("未找到内置主题 `{name}`"));
            };
            let Some(root) = current_themes_root() else {
                app.themes_settings.error = Some("无法定位主题仓库".to_string());
                return app.show_error_toast("无法定位主题仓库");
            };
            let overlay = match load_builtin_overlay(&root, &name) {
                Ok(overlay) => overlay,
                Err(err) => {
                    let message = format!("加载内置覆盖失败: {err}");
                    app.themes_settings.error = Some(message.clone());
                    return app.show_error_toast(message);
                }
            };
            let palette = palette_hex_from_theme(builtin);
            app.themes_settings.draft = Some(draft_from_builtin(&name, &palette, overlay.as_ref()));
            app.themes_settings.mode = ThemesSettingsMode::Edit;
            app.themes_settings.editor_tab = ThemeEditorTab::Basics;
            app.themes_settings.error = None;
            reset_color_picker(app);
            Task::none()
        }
        ThemesMessage::RestoreBuiltinDefault(name) => {
            let Some(root) = current_themes_root() else {
                app.themes_settings.error = Some("无法定位主题仓库".to_string());
                return app.show_error_toast("无法定位主题仓库");
            };
            if let Err(err) = delete_builtin_overlay(&root, &name) {
                let message = format!("恢复默认失败: {err}");
                app.themes_settings.error = Some(message.clone());
                return app.show_error_toast(message);
            }

            let mut tasks = Vec::new();
            if is_active_builtin(app, &name) {
                let Some(builtin) = find_builtin_theme(&name).cloned() else {
                    app.themes_settings.error = Some(format!("未找到内置主题 `{name}`"));
                    return app.show_error_toast(format!("未找到内置主题 `{name}`"));
                };
                app.active_custom_theme_id = None;
                clear_active_theme_skin(app);
                tasks.push(apply_theme(app, builtin));
            }

            if app.themes_settings.draft.as_ref().and_then(|draft| draft.builtin_base.as_deref())
                == Some(name.as_str())
            {
                clear_editor(app);
            }

            refresh_gallery_state(app);
            tasks.push(app.show_success_toast(format!("已恢复内置主题 {name} 的默认值")));
            Task::batch(tasks)
        }
        ThemesMessage::RestorePresetDefault(id) => {
            let Some(root) = current_themes_root() else {
                app.themes_settings.error = Some("无法定位主题仓库".to_string());
                return app.show_error_toast("无法定位主题仓库");
            };
            if let Err(err) = restore_bundled_preset(&root, &id) {
                let message = format!("恢复预设主题 `{id}` 默认值失败: {err}");
                app.themes_settings.error = Some(message.clone());
                return app.show_error_toast(message);
            }

            let mut tasks = Vec::new();
            if app.active_custom_theme_id.as_deref() == Some(id.as_str()) {
                tasks.push(update(
                    app,
                    SettingsMessage::Themes(ThemesMessage::ApplyCustom(id.clone())),
                ));
            }

            if app.themes_settings.draft.as_ref().map(|draft| draft.id.as_str())
                == Some(id.as_str())
            {
                clear_editor(app);
            }

            refresh_gallery_state(app);
            tasks.push(app.show_success_toast(format!("已恢复预设主题 `{id}` 的默认值")));
            Task::batch(tasks)
        }
        ThemesMessage::CancelEdit => {
            clear_editor(app);
            app.themes_settings.error = None;
            Task::none()
        }
        ThemesMessage::EditorTabSelected(tab) => {
            app.themes_settings.editor_tab = tab;
            reset_color_picker(app);
            Task::none()
        }
        ThemesMessage::DraftNameChanged(name) => {
            if let Some(draft) = app.themes_settings.draft.as_mut() {
                draft.name = name;
            }
            sync_new_theme_id(app);
            Task::none()
        }
        ThemesMessage::DraftTaglineChanged(tagline) => {
            if let Some(draft) = app.themes_settings.draft.as_mut() {
                draft.tagline = tagline;
            }
            Task::none()
        }
        ThemesMessage::DraftPaletteChanged(field, value) => {
            if let Some(draft) = app.themes_settings.draft.as_mut() {
                set_draft_palette(draft, field, value);
            }
            Task::none()
        }
        ThemesMessage::PickCover => pick_image_task(ThemesMessage::CoverPicked),
        ThemesMessage::CoverPicked(path) => {
            if let Some(draft) = app.themes_settings.draft.as_mut() {
                apply_cover_picked(draft, path);
            }
            Task::none()
        }
        ThemesMessage::ClearCover => {
            if let Some(draft) = app.themes_settings.draft.as_mut() {
                clear_cover(draft);
            }
            Task::none()
        }
        ThemesMessage::PickBackground => pick_image_task(ThemesMessage::BackgroundPicked),
        ThemesMessage::BackgroundPicked(path) => {
            if let Some(draft) = app.themes_settings.draft.as_mut() {
                apply_background_picked(draft, path);
            }
            Task::none()
        }
        ThemesMessage::ClearBackground => {
            if let Some(draft) = app.themes_settings.draft.as_mut() {
                clear_background(draft);
            }
            Task::none()
        }
        ThemesMessage::PickChatHeader => pick_image_task(ThemesMessage::ChatHeaderPicked),
        ThemesMessage::ChatHeaderPicked(path) => {
            if let Some(draft) = app.themes_settings.draft.as_mut() {
                apply_chat_header_picked(draft, path);
            }
            Task::none()
        }
        ThemesMessage::ClearChatHeader => {
            if let Some(draft) = app.themes_settings.draft.as_mut() {
                clear_chat_header(draft);
            }
            Task::none()
        }
        ThemesMessage::PickChatBody => pick_image_task(ThemesMessage::ChatBodyPicked),
        ThemesMessage::ChatBodyPicked(path) => {
            if let Some(draft) = app.themes_settings.draft.as_mut() {
                apply_chat_body_picked(draft, path);
            }
            Task::none()
        }
        ThemesMessage::ClearChatBody => {
            if let Some(draft) = app.themes_settings.draft.as_mut() {
                clear_chat_body(draft);
            }
            Task::none()
        }
        ThemesMessage::PickChatInput => pick_image_task(ThemesMessage::ChatInputPicked),
        ThemesMessage::ChatInputPicked(path) => {
            if let Some(draft) = app.themes_settings.draft.as_mut() {
                apply_chat_input_picked(draft, path);
            }
            Task::none()
        }
        ThemesMessage::ClearChatInput => {
            if let Some(draft) = app.themes_settings.draft.as_mut() {
                clear_chat_input(draft);
            }
            Task::none()
        }
        ThemesMessage::DraftChromeChanged(field, value) => {
            if let Some(draft) = app.themes_settings.draft.as_mut() {
                set_draft_chrome(draft, field, value);
            }
            Task::none()
        }
        ThemesMessage::ColorPickerToggled(target) => {
            if app.themes_settings.color_picker_target == Some(target) {
                reset_color_picker(app);
            } else {
                app.themes_settings.color_picker_target = Some(target);
                app.themes_settings.color_picker_format =
                    crate::app::components::color_picker::ColorFormat::Hex;
            }
            Task::none()
        }
        ThemesMessage::ColorPickerClosed => {
            reset_color_picker(app);
            Task::none()
        }
        ThemesMessage::ColorPickerColorChanged(hex) => {
            let Some(target) = app.themes_settings.color_picker_target else {
                return Task::none();
            };
            if let Some(draft) = app.themes_settings.draft.as_mut() {
                apply_color_picker_hex(draft, target, hex);
            }
            Task::none()
        }
        ThemesMessage::ColorPickerFormatChanged(format) => {
            app.themes_settings.color_picker_format = format;
            Task::none()
        }
        ThemesMessage::SaveDraft => {
            let Some(draft) = app.themes_settings.draft.clone() else {
                app.themes_settings.error = Some("没有可保存的主题草稿".to_string());
                return app.show_error_toast("没有可保存的主题草稿");
            };

            if let Some(builtin_name) = draft.builtin_base.clone() {
                let Some(root) = current_themes_root() else {
                    app.themes_settings.error = Some("无法定位主题仓库".to_string());
                    return app.show_error_toast("无法定位主题仓库");
                };

                let definition = match try_definition_from_draft(&draft) {
                    Ok(definition) => definition,
                    Err(err) => {
                        app.themes_settings.error = Some(err.clone());
                        return app.show_error_toast(err);
                    }
                };
                let sources = asset_sources_from_draft(&draft);
                let saved = match save_builtin_overlay_with_sources(
                    &root,
                    &builtin_name,
                    &definition,
                    &sources,
                ) {
                    Ok(saved) => saved,
                    Err(err) => {
                        let message = format!("保存内置覆盖失败: {err}");
                        app.themes_settings.error = Some(message.clone());
                        return app.show_error_toast(message);
                    }
                };

                let mut tasks = Vec::new();
                if is_active_builtin(app, &builtin_name) {
                    match resolve_app_theme_name(&builtin_name, None) {
                        Some(theme) => {
                            app.active_custom_theme_id = None;
                            tasks.push(apply_theme(app, theme));
                            sync_overlay_skin(app, &builtin_name);
                        }
                        None => {
                            let message = format!("保存后刷新内置主题 `{builtin_name}` 失败");
                            app.themes_settings.error = Some(message.clone());
                            return app.show_error_toast(message);
                        }
                    }
                }

                clear_editor(app);
                refresh_gallery_state(app);
                tasks.push(app.show_success_toast(format!("已保存内置主题覆盖 {}", saved.name)));
                return Task::batch(tasks);
            }

            let Some(store) = theme_store_from_home() else {
                app.themes_settings.error = Some("无法定位主题仓库".to_string());
                return app.show_error_toast("无法定位主题仓库");
            };

            // Finalize id for new themes before validation.
            sync_new_theme_id(app);

            let Some(draft) = app.themes_settings.draft.clone() else {
                app.themes_settings.error = Some("没有可保存的主题草稿".to_string());
                return app.show_error_toast("没有可保存的主题草稿");
            };

            let definition = match try_definition_from_draft(&draft) {
                Ok(definition) => definition,
                Err(err) => {
                    app.themes_settings.error = Some(err.clone());
                    return app.show_error_toast(err);
                }
            };

            let sources = asset_sources_from_draft(&draft);
            let saved = match store.save_with_sources(&definition, &sources) {
                Ok(saved) => saved,
                Err(err) => {
                    let message = format!("保存主题失败: {err}");
                    app.themes_settings.error = Some(message.clone());
                    return app.show_error_toast(message);
                }
            };

            let mut tasks = Vec::new();
            if app.active_custom_theme_id.as_deref() == Some(saved.id.as_str()) {
                sync_active_theme_skin(app);
                match to_iced_theme(&saved) {
                    Ok(theme) => {
                        tasks.push(apply_theme(app, theme));
                    }
                    Err(err) => {
                        let message = format!("保存后刷新应用主题失败: {err}");
                        app.themes_settings.error = Some(message.clone());
                        return app.show_error_toast(message);
                    }
                }
            }

            clear_editor(app);
            refresh_gallery_state(app);
            tasks.push(app.show_success_toast(format!("已保存主题 {}", saved.name)));
            Task::batch(tasks)
        }
    }
}

#[cfg(test)]
#[path = "themes_tests.rs"]
mod themes_tests;
