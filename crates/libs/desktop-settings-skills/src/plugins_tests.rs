//! 验证插件目录的筛选、操作消息、详情面板和各类加载状态。
//!
//! 插件夹具组合加载、空、筛选、选中、忙碌和禁用状态，覆盖详情面板回退。


use crate::types::PluginCatalogItem;

fn plugin(
    id: &str,
    display_name: &str,
    enabled: bool,
    source_path: Option<&str>,
) -> PluginCatalogItem {
    PluginCatalogItem {
        id: id.to_string(),
        display_name: display_name.to_string(),
        version: "1.0.0".to_string(),
        description: "Shown in settings".to_string(),
        enabled,
        component_count: 2,
        source_path: source_path.map(ToOwned::to_owned),
        dependency_checks: Vec::new(),
        origin: Default::default(),
        hook_review: String::new(),
    }
}

#[test]
fn desktop_plugins_tab_renders_catalog_items() {
    let plugin = PluginCatalogItem {
        id: "demo-plugin".to_string(),
        display_name: "Demo Plugin".to_string(),
        version: "0.2.0".to_string(),
        description: "Shown in settings".to_string(),
        enabled: true,
        component_count: 3,
        source_path: Some("/tmp/demo-plugin".to_string()),
        dependency_checks: Vec::new(),
        origin: Default::default(),
        hook_review: String::new(),
    };

    assert!(super::plugin_matches_query(
        config_ui::DesktopLocale::SimplifiedChinese,
        &plugin,
        "DEMO"
    ));
    assert!(super::plugin_matches_query(
        config_ui::DesktopLocale::SimplifiedChinese,
        &plugin,
        "0.2"
    ));
    assert!(super::plugin_matches_query(
        config_ui::DesktopLocale::SimplifiedChinese,
        &plugin,
        "settings"
    ));
    assert!(super::plugin_matches_query(
        config_ui::DesktopLocale::SimplifiedChinese,
        &plugin,
        "/tmp"
    ));
    assert!(!super::plugin_matches_query(
        config_ui::DesktopLocale::SimplifiedChinese,
        &plugin,
        "missing"
    ));

    let _ = super::plugin_catalog_item(
        config_ui::DesktopLocale::SimplifiedChinese,
        plugin.clone(),
        true,
    );
    let _ = super::plugin_detail_panel(
        config_ui::DesktopLocale::SimplifiedChinese,
        Some(&plugin),
        false,
    );
}

#[test]
fn desktop_plugins_actions_dispatch_settings_messages() {
    let enable = super::plugin_set_enabled_message("demo-plugin", false);
    let delete = super::plugin_delete_message("demo-plugin");

    assert!(format!("{enable:?}").contains("PluginsSetEnabledRequested"));
    assert!(format!("{delete:?}").contains("PluginsDeleteRequested"));
}

#[test]
fn plugin_initials_use_name_words_then_fall_back_to_uppercase_id() {
    let mut plugin = PluginCatalogItem {
        id: "xy-plugin".to_string(),
        display_name: "remote_skill provider".to_string(),
        version: "1.0.0".to_string(),
        description: String::new(),
        enabled: false,
        component_count: 0,
        source_path: None,
        dependency_checks: Vec::new(),
        origin: Default::default(),
        hook_review: String::new(),
    };

    assert_eq!(super::plugin_initials(&plugin), "RS");

    plugin.display_name = "---___".to_string();
    assert_eq!(super::plugin_initials(&plugin), "XY");
}

#[test]
fn plugin_query_trims_whitespace_and_matches_empty_query() {
    let plugin = PluginCatalogItem {
        id: "demo-plugin".to_string(),
        display_name: "Demo Plugin".to_string(),
        version: "1.0.0".to_string(),
        description: "Local plugin".to_string(),
        enabled: true,
        component_count: 1,
        source_path: None,
        dependency_checks: Vec::new(),
        origin: Default::default(),
        hook_review: String::new(),
    };

    assert!(super::plugin_matches_query(
        config_ui::DesktopLocale::SimplifiedChinese,
        &plugin,
        "   "
    ));
    assert!(super::plugin_matches_query(
        config_ui::DesktopLocale::SimplifiedChinese,
        &plugin,
        "  DEMO  "
    ));
    assert!(!super::plugin_matches_query(
        config_ui::DesktopLocale::SimplifiedChinese,
        &plugin,
        "missing"
    ));
}

#[test]
fn plugin_detail_panel_builds_empty_busy_and_disabled_variants() {
    let disabled = plugin("disabled-plugin", "Disabled Plugin", false, None);

    let _ = super::plugin_detail_panel(
        config_ui::DesktopLocale::SimplifiedChinese,
        None,
        false,
    );
    let _ = super::plugin_detail_panel(
        config_ui::DesktopLocale::SimplifiedChinese,
        Some(&disabled),
        true,
    );
    let _ = super::plugin_catalog_item(
        config_ui::DesktopLocale::SimplifiedChinese,
        disabled,
        false,
    );
}

#[test]
fn plugins_view_builds_loading_empty_filtered_and_selected_fallback_states() {
    let mut app = Fixture::default();

    app.skills_settings.plugin_loading = true;
    let _ = super::view(app.view());

    app.skills_settings.plugin_loading = false;
    app.skills_settings.plugin_status_message = Some("加载失败".to_string());
    app.skills_settings.plugin_status_is_error = true;
    let _ = super::view(app.view());

    app.skills_settings.plugin_catalog = vec![
        plugin("alpha-plugin", "Alpha Plugin", true, Some("/tmp/alpha")),
        plugin("beta-plugin", "Beta Plugin", false, None),
    ];
    app.skills_settings.query = "alpha".to_string();
    app.skills_settings.selected_plugin_id = Some("missing-plugin".to_string());
    let _ = super::view(app.view());

    app.skills_settings.query = "nothing".to_string();
    let _ = super::view(app.view());
}

struct Fixture { skills_settings: Settings, theme: iced::Theme }
impl Default for Fixture { fn default() -> Self { Self { skills_settings: Settings::default(), theme: iced::Theme::Dark } } }
#[derive(Default)]
struct Settings {
    translation: Option<desktop_i18n::format::Message>,
    plugin_loading: bool,
    plugin_status_message: Option<String>,
    plugin_status_is_error: bool,
    plugin_catalog: Vec<PluginCatalogItem>,
    query: String,
    selected_plugin_id: Option<String>,
}
impl Fixture {
    fn view(&self) -> super::ViewState<'_> {
        super::ViewState {
            desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
            app_theme: &self.theme,
            query: &self.skills_settings.query,
            plugin_catalog: &self.skills_settings.plugin_catalog,
            selected_plugin_id: &self.skills_settings.selected_plugin_id,
            plugin_loading: self.skills_settings.plugin_loading,
            plugin_status_message: &self.skills_settings.plugin_status_message,
            plugin_status_message_translation: &self.skills_settings.translation,
            plugin_status_is_error: self.skills_settings.plugin_status_is_error,
        }
    }
}

#[tokio::test]
async fn desktop_gettext_extensions_visible_copy() {
    use crate::update::{
        catalog::{self, Event},
        state::State,
    };
    use config_ui::DesktopLocale::{English, SimplifiedChinese, TraditionalChinese};
    let name = "私人插件 %s";
    let raw = "HTTP 503 / 私人插件/%n/%s";
    let mut state = State::default();
    catalog::update(
        &mut state,
        &None,
        Event::PluginsSetEnabledCompleted {
            plugin_id: "private-plugin".into(),
            enabled: true,
            result: Ok(name.into()),
        },
    );
    let feedback = state.plugin_status_message_translation.as_ref().unwrap();
    use crate::plugin_install::{self, Event as InstallEvent, PluginInstallForm};
    use iced_runtime::futures::futures::StreamExt;
    let mut form = PluginInstallForm::new(7, None);
    form.source = "npm:私人插件@1.0".into();
    form.begin_prepare().unwrap();
    let mut install = Some(form);
    let mut operation = 7;
    let pending = plugin_install::update(
        &mut plugin_install::State {
            plugin_operation_id: &mut operation,
            plugin_install: &mut install,
        },
        InstallEvent::PluginsInstallPrepared { request_id: 7, result: Err(raw.into()) },
    );
    let mut outputs = iced_runtime::task::into_stream(pending.collect()).unwrap();
    let notice = match outputs.next().await.unwrap() {
        iced_runtime::Action::Output(events) => events
            .into_iter()
            .find_map(|event| match event {
                InstallEvent::Toast(_, feedback) => Some(feedback),
                _ => None,
            })
            .expect("plugin failure notification"),
        _ => panic!("expected plugin notification"),
    };

    let mut invalid = Some(PluginInstallForm::new(8, None));
    let _ = plugin_install::update(
        &mut plugin_install::State {
            plugin_operation_id: &mut operation,
            plugin_install: &mut invalid,
        },
        InstallEvent::PluginsInstallPrepare,
    );
    let validation = invalid.as_ref().unwrap().failure_translation.as_ref().unwrap();
    for (locale, enabled, failure) in [
        (English, "Enabled plugin: ", "Plugin installation failed: "),
        (TraditionalChinese, "已啟用外掛：", "外掛安裝失敗："),
        (SimplifiedChinese, "已启用插件: ", "插件安装失败："),
    ] {
        assert_eq!(
            feedback.render(desktop_i18n::I18n::new(locale)).unwrap(),
            format!("{enabled}{name}")
        );
        assert_eq!(
            validation.render(desktop_i18n::I18n::new(locale)).unwrap(),
            match locale {
                English => "Source cannot be empty",
                TraditionalChinese => "來源不可為空",
                _ => "来源不能为空",
            }
        );
        assert_eq!(super::failure_feedback(locale, raw), format!("{failure}{raw}"));
        assert_eq!(
            notice.render(desktop_i18n::I18n::new(locale)).unwrap(),
            format!("{failure}{raw}")
        );
        assert_eq!(
            desktop_i18n::settings_extensions::feedback(desktop_i18n::I18n::new(locale), raw),
            raw
        );
        assert_eq!(
            desktop_i18n::settings_extensions::feedback(
                desktop_i18n::I18n::new(locale),
                "技能导入失败：HTTP 503 / 私人插件/%n/%s"
            ),
            format!(
                "{}{raw}",
                match locale {
                    English => "Skill import failed: ",
                    TraditionalChinese => "技能導入失敗：",
                    _ => "技能导入失败：",
                }
            )
        );
        let item = plugin("private-plugin", name, true, Some("/私人/%s"));
        let _ = super::plugin_detail_panel(locale, Some(&item), false);
        assert_eq!(item.display_name, name);
        assert_eq!(item.source_path.as_deref(), Some("/私人/%s"));
    }
}
