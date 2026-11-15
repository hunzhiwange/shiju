//! Resolve the selected theme identity before the host applies cross-domain effects.
use iced::Theme;
use crate::{
    ActiveThemeSkin, clear_active_theme_skin, find_custom_theme_id, sync_active_theme_skin,
    theme_store_from_home,
};

pub fn sync_active_custom_theme_id(
    theme: &Theme,
    id: &mut Option<String>,
    skin: &mut Option<ActiveThemeSkin>,
) {
    if !matches!(theme, Theme::Custom(_)) {
        *id = None;
        clear_active_theme_skin(skin);
        return;
    }
    if id.is_none() {
        if let Some(store) = theme_store_from_home() {
            *id = find_custom_theme_id(theme, &store);
        }
    }
    sync_active_theme_skin(id.as_deref(), skin);
}
