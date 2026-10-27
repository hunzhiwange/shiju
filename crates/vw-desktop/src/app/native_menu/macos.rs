//! macOS 系统菜单栏安装与事件桥接。

use std::sync::Mutex;

use iced::Subscription;
use iced::futures::SinkExt;
use muda::accelerator::{Accelerator, Code, Modifiers};
use muda::{AboutMetadata, CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use once_cell::sync::Lazy;

use super::actions::{
    NativeMenuSnapshot, help_section, menu_item_enabled, menu_sections, parse_window_tab_menu_id,
    window_tab_menu_id,
};
use crate::app::Message;
use crate::app::components::top_bar::menu_actions::NativeMenuAction;
use crate::app::components::top_bar::menus::MenuEntry;
use crate::app::message::view::ViewMessage;

static INSTALLED_SNAPSHOT: Lazy<Mutex<Option<NativeMenuSnapshot>>> = Lazy::new(|| Mutex::new(None));

/// 按快照安装或重建原生菜单。
pub(crate) fn install_or_sync(snapshot: NativeMenuSnapshot) {
    // Unit tests drive App::update on harness workers without an AppKit event loop.
    // Explicit native-window tests still run this path on the real main thread.
    #[cfg(test)]
    // SAFETY: pthread_main_np takes no pointers and only reports the current thread.
    if unsafe { libc::pthread_main_np() } == 0 {
        return;
    }

    let Ok(mut installed) = INSTALLED_SNAPSHOT.lock() else {
        return;
    };
    if installed.as_ref() == Some(&snapshot) {
        return;
    }

    let Some((menu, window_menu, help_menu)) = build_menu(&snapshot) else {
        return;
    };
    // NSApp 会持有主菜单；Menu 本身不是 Send，不能放进跨线程静态存储。
    menu.init_for_nsapp();
    // 注册为系统窗口/帮助菜单后，macOS 会自动注入填充、居中、移动与调整大小、
    // 全屏幕平铺等标准项（以及窗口搜索框）。
    window_menu.set_as_windows_menu_for_nsapp();
    help_menu.set_as_help_menu_for_nsapp();
    std::mem::forget(menu);
    *installed = Some(snapshot);
}

pub(crate) fn subscription() -> Subscription<Message> {
    Subscription::run(native_menu_events)
}

fn native_menu_events() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(
        32,
        |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
            loop {
                let received = tokio::task::spawn_blocking(|| MenuEvent::receiver().recv()).await;
                let Ok(Ok(event)) = received else {
                    break;
                };
                let message = if let Some(tab_id) = parse_window_tab_menu_id(event.id.as_ref()) {
                    Message::View(ViewMessage::TabSelected(tab_id.to_string()))
                } else if let Some(action) = NativeMenuAction::from_id(event.id.as_ref()) {
                    Message::NativeMenu(action)
                } else {
                    continue;
                };
                if output.try_send(message.clone()).is_err() {
                    let _ = output.send(message).await;
                }
            }
        },
    )
}

fn build_menu(snapshot: &NativeMenuSnapshot) -> Option<(Menu, Submenu, Submenu)> {
    let menu = Menu::new();

    let app_menu = Submenu::new("ShiJu", true);
    app_menu
        .append(&PredefinedMenuItem::about(
            Some("关于识矩"),
            Some(AboutMetadata {
                name: Some("识矩".to_string()),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
                ..Default::default()
            }),
        ))
        .ok()?;
    app_menu.append(&PredefinedMenuItem::separator()).ok()?;
    app_menu.append(&PredefinedMenuItem::hide(Some("隐藏识矩"))).ok()?;
    app_menu.append(&PredefinedMenuItem::hide_others(Some("隐藏其他"))).ok()?;
    app_menu.append(&PredefinedMenuItem::show_all(Some("显示全部"))).ok()?;
    app_menu.append(&PredefinedMenuItem::separator()).ok()?;
    app_menu.append(&PredefinedMenuItem::quit(Some("退出识矩"))).ok()?;
    menu.append(&app_menu).ok()?;

    for (title, entries) in menu_sections(snapshot) {
        let submenu = Submenu::new(title, true);
        append_entries(&submenu, &entries, snapshot)?;
        menu.append(&submenu).ok()?;
    }

    let window_menu = build_window_menu(snapshot)?;
    menu.append(&window_menu).ok()?;

    let (help_title, help_entries) = help_section();
    let help_menu = Submenu::new(help_title, true);
    append_entries(&help_menu, &help_entries, snapshot)?;
    menu.append(&help_menu).ok()?;

    Some((menu, window_menu, help_menu))
}

fn build_window_menu(snapshot: &NativeMenuSnapshot) -> Option<Submenu> {
    // 仅放置基础项；填充 / 居中 / 移动与调整大小 / 全屏幕平铺等由
    // `set_as_windows_menu_for_nsapp` 触发系统注入。
    let window_menu = Submenu::new("窗口", true);
    window_menu.append(&PredefinedMenuItem::minimize(Some("最小化"))).ok()?;
    window_menu.append(&PredefinedMenuItem::maximize(Some("缩放"))).ok()?;
    window_menu.append(&PredefinedMenuItem::fullscreen(Some("进入全屏幕"))).ok()?;
    window_menu.append(&PredefinedMenuItem::separator()).ok()?;
    window_menu.append(&PredefinedMenuItem::bring_all_to_front(Some("前置全部窗口"))).ok()?;

    if !snapshot.open_window_tabs.is_empty() {
        window_menu.append(&PredefinedMenuItem::separator()).ok()?;
        for tab in &snapshot.open_window_tabs {
            let item = CheckMenuItem::with_id(
                window_tab_menu_id(&tab.id),
                &tab.title,
                true,
                tab.active,
                None,
            );
            window_menu.append(&item).ok()?;
        }
    }

    Some(window_menu)
}

fn append_entries(
    submenu: &Submenu,
    entries: &[MenuEntry],
    snapshot: &NativeMenuSnapshot,
) -> Option<()> {
    for entry in entries {
        match entry {
            MenuEntry::Separator => {
                submenu.append(&PredefinedMenuItem::separator()).ok()?;
            }
            MenuEntry::Item { action, label, shortcut, .. } => {
                let Some(action) = *action else {
                    continue;
                };
                // 剪切/复制/粘贴不注册系统加速键：否则 macOS 会截获 ⌘X/⌘C/⌘V，
                // iced 自绘输入框与代码编辑器收不到按键。菜单点击仍走应用内路由。
                let accelerator = match action {
                    NativeMenuAction::EditCut
                    | NativeMenuAction::EditCopy
                    | NativeMenuAction::EditPaste => None,
                    _ => (*shortcut).and_then(parse_shortcut),
                };
                let enabled = menu_item_enabled(action, snapshot);
                let item = MenuItem::with_id(action.id(), *label, enabled, accelerator);
                submenu.append(&item).ok()?;
            }
        }
    }
    Some(())
}

pub(crate) fn parse_shortcut(shortcut: &str) -> Option<Accelerator> {
    let mut modifiers = Modifiers::empty();
    let mut key = shortcut;
    loop {
        if let Some(rest) = key.strip_prefix('⌘') {
            modifiers |= Modifiers::META;
            key = rest;
            continue;
        }
        if let Some(rest) = key.strip_prefix('⇧') {
            modifiers |= Modifiers::SHIFT;
            key = rest;
            continue;
        }
        if let Some(rest) = key.strip_prefix('⌥') {
            modifiers |= Modifiers::ALT;
            key = rest;
            continue;
        }
        if let Some(rest) = key.strip_prefix('⌃') {
            modifiers |= Modifiers::CONTROL;
            key = rest;
            continue;
        }
        break;
    }

    let code = match key {
        "A" | "a" => Code::KeyA,
        "C" | "c" => Code::KeyC,
        "F" | "f" => Code::KeyF,
        "R" | "r" => Code::KeyR,
        "S" | "s" => Code::KeyS,
        "V" | "v" => Code::KeyV,
        "X" | "x" => Code::KeyX,
        "Z" | "z" => Code::KeyZ,
        _ => return None,
    };

    Some(Accelerator::new(Some(modifiers), code))
}

#[cfg(test)]
#[path = "macos_tests.rs"]
mod macos_tests;
