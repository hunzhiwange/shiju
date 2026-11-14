//! 原生资源只在 UI 线程创建、保存和销毁；启动项操作由后台任务调用。

use std::cell::RefCell;
use std::path::PathBuf;

use crate::app::message::GeneralSettingsMessage;
use auto_launch::{AutoLaunch, AutoLaunchBuilder};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, Submenu};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use vw_gateway_client::vw_api_types::agent_apps::AppId;

const APP_PAGES: &[(AppId, &str, &str)] = &[
    (AppId::Claude, "shiju.tray.app.claude", "Claude"),
    (AppId::ClaudeDesktop, "shiju.tray.app.claude-desktop", "Claude Desktop"),
    (AppId::Codex, "shiju.tray.app.codex", "Codex"),
    (AppId::Gemini, "shiju.tray.app.gemini", "Gemini"),
    (AppId::GrokBuild, "shiju.tray.app.grokbuild", "Grok Build"),
    (AppId::OpenCode, "shiju.tray.app.opencode", "OpenCode"),
    (AppId::OpenClaw, "shiju.tray.app.openclaw", "OpenClaw"),
    (AppId::Hermes, "shiju.tray.app.hermes", "Hermes"),
    (AppId::Pi, "shiju.tray.app.pi", "Pi"),
    (AppId::Mcode, "shiju.tray.app.mcode", "Mcode"),
    (AppId::Kimi, "shiju.tray.app.kimi", "Kimi"),
    (AppId::Dsh, "shiju.tray.app.dsh", "Dsh"),
    (AppId::Agy, "shiju.tray.app.agy", "Agy"),
    (AppId::Omp, "shiju.tray.app.omp", "Omp"),
];

const SHOW_ID: &str = "shiju.tray.show";
const QUIT_ID: &str = "shiju.tray.quit";
thread_local! {
    static TRAY: RefCell<Option<LocalizedTray>> = const { RefCell::new(None) };
}

struct LocalizedTray {
    _icon: TrayIcon,
    apps: Submenu,
    providers: Submenu,
    provider_rows: Vec<(String, String, MenuItem)>,
    provider_signature: String,
    show: MenuItem,
    quit: MenuItem,
    status: MenuItem,
    shutting_down: bool,
    background: bool,
}

/// Update existing native items without creating a tray when the preference is disabled.
pub(crate) fn refresh_tray_locale(locale: vw_config_types::ui::DesktopLocale) {
    TRAY.with(|slot| {
        if let Some(tray) = slot.borrow().as_ref() {
            let i18n = crate::app::i18n::I18n::new(locale);
            tray.show.set_text(i18n.__("显示主窗口"));
            tray.apps.set_text(i18n.__("应用列表"));
            tray.providers.set_text(i18n.__("切换当前应用供应商"));
            tray.quit.set_text(i18n.__("退出识矩"));
            tray.status.set_text(tray_status_label(locale, tray.shutting_down, tray.background));
        }
    });
}

/// Uses only the application snapshot; no disk or SDK request runs on the UI thread.
pub(crate) fn refresh_tray_state(
    locale: vw_config_types::ui::DesktopLocale,
    shutting_down: bool,
    background: bool,
) {
    TRAY.with(|slot| {
        if let Some(tray) = slot.borrow_mut().as_mut() {
            tray.shutting_down = shutting_down;
            tray.background = background;
            tray.status.set_text(tray_status_label(locale, shutting_down, background));
            tray.apps.set_enabled(!shutting_down);
            tray.show.set_enabled(!shutting_down);
            tray.quit.set_enabled(!shutting_down);
        }
    });
}

/// Reflect only the SDK-backed snapshot; plain items do not toggle optimistically.
pub(crate) fn refresh_tray_providers(app: &crate::app::App) {
    use vw_gateway_client::vw_api_types::app_providers::catalog::ProviderSelection;
    TRAY.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(tray) = slot.as_mut() else { return };
        let state = &app.app_providers;
        let enabled = !app.shutdown_requested && !state.loading && state.tray_switch.is_none();
        let scope = state.scope.as_ref().filter(|s| super::tray_switch::supports(s.app));
        let snapshot = scope.and(state.snapshot.as_ref());
        let signature = format!("{:?}:{:?}:{:?}:{enabled}", state.scope, state.generation, snapshot);
        if signature == tray.provider_signature { return }
        for (_, _, item) in tray.provider_rows.drain(..) {
            let _ = tray.providers.remove(&item);
        }
        tray.providers.set_enabled(enabled && snapshot.is_some());
        if let Some(snapshot) = snapshot {
            for (index, row) in snapshot.catalog.rows.iter().enumerate() {
                let checked = matches!(&snapshot.catalog.in_use, ProviderSelection::Selected { id } if id == &row.provider.id);
                let label = format!("{}{}", if checked { "✓ " } else { "" }, row.provider.name);
                let event_id = format!("shiju.tray.provider.{}.{}", state.generation, index);
                let item = MenuItem::with_id(event_id.clone(), label, enabled, None);
                if tray.providers.append(&item).is_ok() {
                    tray.provider_rows.push((event_id, row.provider.id.clone(), item));
                }
            }
        }
        tray.provider_signature = signature;
    });
}

fn tray_status_label(
    locale: vw_config_types::ui::DesktopLocale,
    shutting_down: bool,
    background: bool,
) -> &'static str {
    let i18n = crate::app::i18n::I18n::new(locale);
    if shutting_down {
        i18n.__("正在退出")
    } else if background {
        i18n.__("后台运行中")
    } else {
        i18n.__("运行中")
    }
}

fn legacy_launcher() -> Result<AutoLaunch, String> {
    let executable = std::env::current_exe().map_err(|err| err.to_string())?;
    let path = executable.to_str().ok_or("启动程序路径不是有效 UTF-8")?;
    let mut builder = AutoLaunchBuilder::new();
    builder.set_app_name(if cfg!(debug_assertions) { "ShiJu-dev" } else { "ShiJu" });
    builder.set_app_path(path);
    // 只注册当前用户，不尝试机器级注册或提升权限。
    #[cfg(windows)]
    builder.set_windows_enable_mode(auto_launch::WindowsEnableMode::CurrentUser);
    builder.build().map_err(|err| err.to_string())
}

// macOS 主应用登录项必须由当前 .app 自身注册，不能指向另一个已安装版本。
#[cfg(target_os = "macos")]
fn macos_app_bundle(executable: &std::path::Path) -> Option<&std::path::Path> {
    let macos = executable.parent()?;
    let contents = macos.parent()?;
    let bundle = contents.parent()?;
    (macos.file_name()? == "MacOS"
        && contents.file_name()? == "Contents"
        && bundle.extension()? == "app"
        && contents.join("Info.plist").is_file())
    .then_some(bundle)
}

fn launcher() -> Result<AutoLaunch, String> {
    #[cfg(target_os = "macos")]
    {
        let executable = std::env::current_exe().map_err(|err| err.to_string())?;
        if macos_app_bundle(&executable).is_none() {
            return Err("macOS 开机启动需从打包后的 .app 运行；请打开识矩应用后再启用".into());
        }
        return AutoLaunchBuilder::new()
            .set_macos_launch_mode(auto_launch::MacOSLaunchMode::SMAppService)
            .build()
            .map_err(|err| err.to_string());
    }
    #[cfg(not(target_os = "macos"))]
    legacy_launcher()
}

pub(crate) fn launch_at_login_enabled() -> bool {
    read_launch_at_login().unwrap_or_else(|error| {
        tracing::warn!(%error, "failed to read login launch state");
        false
    })
}

pub(crate) fn read_launch_at_login() -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    if macos_app_bundle(&std::env::current_exe().map_err(|err| err.to_string())?).is_none() {
        // 保留旧项的可关闭入口，避免升级后遗留开发目录的启动代理。
        return legacy_launcher()?.is_enabled().map_err(|err| err.to_string());
    }
    launcher()?.is_enabled().map_err(|err| err.to_string())
}

pub(crate) fn set_launch_at_login(enabled: bool) -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    if !enabled {
        remove_legacy_login_item()?;
        if std::env::current_exe().ok().is_some_and(|path| macos_app_bundle(&path).is_none()) {
            return Ok(false);
        }
    }
    let launcher = launcher()?;
    // 已处于目标状态时不重复注册，避免系统拒绝重复操作。
    if launcher.is_enabled().map_err(|err| err.to_string())? != enabled {
        if enabled { launcher.enable() } else { launcher.disable() }
            .map_err(|err| err.to_string())?;
    }
    let actual = launcher.is_enabled().map_err(|err| err.to_string())?;
    if actual != enabled {
        return Err("登录项状态与请求不一致，请在系统设置 → 通用 → 登录项中检查识矩及其授权".into());
    }
    #[cfg(target_os = "macos")]
    if enabled {
        remove_legacy_login_item()?;
    }
    Ok(actual)
}

#[cfg(target_os = "macos")]
fn remove_legacy_login_item() -> Result<(), String> {
    let legacy = legacy_launcher()?;
    if legacy.is_enabled().map_err(|err| err.to_string())? {
        legacy.disable().map_err(|err| err.to_string())?;
    }
    Ok(())
}

fn preference_path() -> Result<PathBuf, String> {
    crate::app::project_dirs()
        .map(|dirs| dirs.config_dir().join("desktop-lifecycle.json"))
        .ok_or_else(|| "无法定位本机配置目录".into())
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct Preferences {
    close_to_tray: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self { close_to_tray: true }
    }
}

pub(crate) fn close_to_tray_enabled() -> bool {
    let result = (|| read_close_to_tray(&preference_path()?))();
    result.unwrap_or_else(|error: String| {
        tracing::warn!(%error, "failed to read close-to-tray preference");
        false
    })
}

fn read_close_to_tray(path: &std::path::Path) -> Result<bool, String> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Preferences::default().close_to_tray);
        }
        Err(error) => return Err(error.to_string()),
    };
    serde_json::from_slice::<Preferences>(&bytes)
        .map(|prefs| prefs.close_to_tray)
        .map_err(|err| err.to_string())
}

pub(crate) fn save_close_to_tray(enabled: bool) -> Result<(), String> {
    write_close_to_tray(&preference_path()?, enabled)
}

fn write_close_to_tray(path: &std::path::Path, enabled: bool) -> Result<(), String> {
    let parent = path.parent().ok_or("本机配置目录无效")?;
    std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|err| err.to_string())?;
    serde_json::to_writer(file.as_file_mut(), &Preferences { close_to_tray: enabled })
        .map_err(|err| err.to_string())?;
    file.as_file().sync_all().map_err(|err| err.to_string())?;
    file.persist(path).map_err(|err| err.to_string())?;
    Ok(())
}

pub(crate) fn ensure_tray(locale: vw_config_types::ui::DesktopLocale) -> Result<(), String> {
    TRAY.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_some() {
            return Ok(());
        }
        // 测试工作线程没有原生事件循环，禁止在其上调用 AppKit。
        #[cfg(all(test, target_os = "macos"))]
        if unsafe { libc::pthread_main_np() } == 0 {
            return Err("托盘需要原生 UI 主线程".into());
        }
        let i18n = crate::app::i18n::I18n::new(locale);
        let menu = Menu::new();
        let status = MenuItem::new(tray_status_label(locale, false, false), false, None);
        let apps = Submenu::new(i18n.__("应用列表"), true);
        for (_, event_id, label) in APP_PAGES {
            apps.append(&MenuItem::with_id(*event_id, *label, true, None))
                .map_err(|err| err.to_string())?;
        }
        menu.append(&apps).map_err(|err| err.to_string())?;
        let providers = Submenu::new(i18n.__("切换当前应用供应商"), false);
        menu.append(&providers).map_err(|err| err.to_string())?;
        let show = MenuItem::with_id(SHOW_ID, i18n.__("显示主窗口"), true, None);
        let quit = MenuItem::with_id(QUIT_ID, i18n.__("退出识矩"), true, None);
        menu.append(&status).map_err(|err| err.to_string())?;
        menu.append(&show).map_err(|err| err.to_string())?;
        menu.append(&quit).map_err(|err| err.to_string())?;
        let image = image::load_from_memory(include_bytes!(
            "../../../../../assets/logo/icons.iconset/icon_32x32.png"
        ))
        .map_err(|err| err.to_string())?
        .into_rgba8();
        let (width, height) = image.dimensions();
        let icon =
            Icon::from_rgba(image.into_raw(), width, height).map_err(|err| err.to_string())?;
        *slot = Some(LocalizedTray {
            _icon: TrayIconBuilder::new()
                .with_tooltip("ShiJu")
                .with_menu(Box::new(menu))
                // Keep the panel as a native menu: AppKit anchors it to the status
                // item on its screen and owns edge fitting and dismissal (Esc/outside click).
                .with_menu_on_left_click(true)
                .with_icon(icon)
                .build()
                .map_err(|err| err.to_string())?,
            apps,
            providers,
            provider_rows: Vec::new(),
            provider_signature: String::new(),
            show,
            quit,
            status,
            shutting_down: false,
            background: false,
        });
        Ok(())
    })
}

pub(crate) fn remove_tray() {
    TRAY.with(|slot| {
        slot.borrow_mut().take();
    });
}

pub(crate) fn poll_tray() -> Vec<GeneralSettingsMessage> {
    MenuEvent::receiver()
        .try_iter()
        // Bound work per tick even if native menu events keep arriving.
        .take(32)
        .filter_map(|event| match event.id.as_ref() {
            SHOW_ID => Some(GeneralSettingsMessage::TrayShow),
            QUIT_ID => Some(GeneralSettingsMessage::TrayQuit),
            id if APP_PAGES.iter().any(|(_, event_id, _)| *event_id == id) => {
                APP_PAGES.iter().find(|(_, event_id, _)| *event_id == id)
                    .map(|(app, _, _)| GeneralSettingsMessage::TrayOpenApp(*app))
            }
            id => TRAY.with(|slot| {
                slot.borrow().as_ref()?.provider_rows.iter()
                    .find(|(event_id, _, _)| event_id == id)
                    .map(|(_, provider, _)| GeneralSettingsMessage::TrayProviderSwitch(provider.clone()))
            }),
        })
        .collect()
}

#[cfg(test)]
#[path = "native_tests.rs"]
mod tests;
