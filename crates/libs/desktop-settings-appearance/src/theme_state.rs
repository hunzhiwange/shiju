//! Initial theme resolution and the active appearance state.
use iced::Theme;
pub struct State {
    /// 应用主题
    pub app_theme: Theme,
    /// 当前应用的自定义主题 id（配置键 `custom:<id>`）；内置主题时为 `None`
    pub active_custom_theme_id: Option<String>,
    /// 活动自定义主题的皮肤快照（背景路径与 chrome）；内置主题时为 `None`
    pub active_theme_skin: Option<desktop_theme::theme_catalog::ActiveThemeSkin>,
    /// 编辑器是否跟随系统主题
    pub editor_follow_system_theme: bool,
    /// 编辑器主题
    pub editor_theme: Theme,
}
impl State {
    pub fn new(system: &config_ui::AppSystemSettingsConfig) -> Self {
        // 应用程序主题（启动时轻量种子内置预设，失败仅 warn）
        if let Some(root) = desktop_theme::theme_catalog::current_themes_root() {
            if let Err(err) = desktop_theme::theme_catalog::ensure_bundled_presets(&root) {
                tracing::warn!(
                    target: "vw_desktop",
                    error = %err,
                    "failed to seed bundled theme presets on startup"
                );
            }
        }
        let theme_store = desktop_theme::theme_catalog::theme_store_from_home();
        let (cfg_app_theme, cfg_active_custom_theme_id) =
            match desktop_theme::theme_catalog::resolve_app_theme_name(
                &system.app_theme,
                theme_store.as_ref(),
            ) {
                Some(theme) => {
                    let custom_id = desktop_theme::theme_catalog::custom_theme_id_from_config(
                        &system.app_theme,
                    )
                    .map(str::to_string);
                    (theme, custom_id)
                }
                None => (Theme::Light, None),
            };
        // 编辑器主题配置
        let cfg_editor_follow_system_theme = system.editor_follow_system_theme;
        let cfg_editor_theme = desktop_theme::theme_catalog::resolve_app_theme_name(
            &system.editor_theme,
            theme_store.as_ref(),
        )
        .unwrap_or_else(|| cfg_app_theme.clone());

        Self {
            app_theme: cfg_app_theme,
            active_custom_theme_id: cfg_active_custom_theme_id,
            active_theme_skin: None,
            editor_follow_system_theme: cfg_editor_follow_system_theme,
            editor_theme: cfg_editor_theme,
        }
    }
}

impl State {
    pub fn apply_app_theme(
        &mut self,
        name: &str,
        store: Option<&desktop_theme::theme_catalog::ThemeStore>,
    ) -> bool {
        match desktop_theme::theme_catalog::resolve_app_theme_name(name, store) {
            Some(theme) => {
                self.app_theme = theme;
                self.active_custom_theme_id =
                    desktop_theme::theme_catalog::custom_theme_id_from_config(name)
                        .map(str::to_string);
                true
            }
            None => {
                tracing::warn!(target: "vw_desktop", theme = %name, "bootstrap could not resolve app theme; keeping current theme");
                false
            }
        }
    }
    pub fn apply_editor_theme(
        &mut self,
        name: &str,
        store: Option<&desktop_theme::theme_catalog::ThemeStore>,
    ) {
        self.editor_theme = desktop_theme::theme_catalog::resolve_app_theme_name(name, store)
            .unwrap_or_else(|| self.app_theme.clone());
    }
}
