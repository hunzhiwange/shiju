//! Resolve the selected theme identity before the host applies cross-domain effects.
use iced::Theme;
use crate::{
    ActiveThemeSkin, clear_active_theme_skin, find_custom_theme_id, sync_active_theme_skin,
    theme_store_from_home, to_iced_theme,
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
    *id = theme_store_from_home().and_then(|store| {
        // The picker supplies a palette and name. The existing ID may belong to the previous theme.
        let matching_id = id.take().filter(|id| {
            store
                .load(id)
                .ok()
                .and_then(|definition| to_iced_theme(&definition).ok())
                .is_some_and(|candidate| {
                    candidate.to_string() == theme.to_string() && candidate.palette() == theme.palette()
                })
        });
        matching_id.or_else(|| find_custom_theme_id(theme, &store))
    });
    sync_active_theme_skin(id.as_deref(), skin);
}
