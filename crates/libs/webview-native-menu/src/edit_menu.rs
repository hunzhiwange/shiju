//! macOS 原生主菜单：Edit / Browser / Window / Help。
//!
//! wry 在 child webview 上会把 `performKeyEquivalent:` 返回 NO，转而依赖
//! `NSApplication.mainMenu` 的加速键。没有 Edit 菜单时复制粘贴快捷键无效。
//! Browser 菜单转发到事件循环；Window / Help 提供标准窗口控制与关于面板入口。

use std::sync::Mutex;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{NSApplication, NSEventModifierFlags, NSMenu, NSMenuItem};
use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol, NSString};
use tao::event_loop::EventLoopProxy;

use webview_core::events::UserEvent;

static MENU_PROXY: Mutex<Option<EventLoopProxy<UserEvent>>> = Mutex::new(None);
static CURRENT_URL: Mutex<String> = Mutex::new(String::new());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MenuAction {
    pub(crate) title: &'static str,
    pub(crate) selector: &'static str,
    pub(crate) key: &'static str,
    pub(crate) shift: bool,
    pub(crate) control: bool,
}

/// Edit 菜单动作列表（供测试与安装共用）。
pub(crate) const EDIT_MENU_ACTIONS: &[MenuAction] = &[
    MenuAction { title: "撤销", selector: "undo:", key: "z", shift: false, control: false },
    MenuAction { title: "重做", selector: "redo:", key: "z", shift: true, control: false },
    MenuAction { title: "剪切", selector: "cut:", key: "x", shift: false, control: false },
    MenuAction { title: "复制", selector: "copy:", key: "c", shift: false, control: false },
    MenuAction { title: "粘贴", selector: "paste:", key: "v", shift: false, control: false },
    MenuAction { title: "全选", selector: "selectAll:", key: "a", shift: false, control: false },
];

/// Browser 菜单动作列表。
pub(crate) const BROWSER_MENU_ACTIONS: &[MenuAction] = &[
    MenuAction {
        title: "后退", selector: "browserBack:", key: "[", shift: false, control: false
    },
    MenuAction {
        title: "前进",
        selector: "browserForward:",
        key: "]",
        shift: false,
        control: false,
    },
    MenuAction {
        title: "刷新",
        selector: "browserReload:",
        key: "r",
        shift: false,
        control: false,
    },
    MenuAction {
        title: "在外部浏览器中打开",
        selector: "browserOpenExternal:",
        key: "",
        shift: false,
        control: false,
    },
    MenuAction {
        title: "关闭页面",
        selector: "browserClose:",
        key: "",
        shift: false,
        control: false,
    },
];

/// Window 菜单动作列表。
pub(crate) const WINDOW_MENU_ACTIONS: &[MenuAction] = &[
    MenuAction {
        title: "最小化",
        selector: "performMiniaturize:",
        key: "m",
        shift: false,
        control: false,
    },
    MenuAction { title: "缩放", selector: "performZoom:", key: "", shift: false, control: false },
    MenuAction {
        title: "进入全屏幕",
        selector: "toggleFullScreen:",
        key: "f",
        shift: false,
        control: true,
    },
];

/// Help 菜单动作列表。
pub(crate) const HELP_MENU_ACTIONS: &[MenuAction] = &[MenuAction {
    title: "关于 Vibe Window WebView",
    selector: "orderFrontStandardAboutPanel:",
    key: "",
    shift: false,
    control: false,
}];

#[derive(Debug, Default)]
struct MenuTargetIvars;

define_class!(
    // SAFETY: NSObject 无额外子类约束；MenuTarget 不实现 Drop。
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[name = "VwWebViewMenuTarget"]
    #[ivars = MenuTargetIvars]
    struct MenuTarget;

    unsafe impl NSObjectProtocol for MenuTarget {}

    impl MenuTarget {
        #[unsafe(method(browserBack:))]
        fn browser_back(&self, _sender: Option<&AnyObject>) {
            send_menu_event(UserEvent::Back);
        }

        #[unsafe(method(browserForward:))]
        fn browser_forward(&self, _sender: Option<&AnyObject>) {
            send_menu_event(UserEvent::Forward);
        }

        #[unsafe(method(browserReload:))]
        fn browser_reload(&self, _sender: Option<&AnyObject>) {
            send_menu_event(UserEvent::Reload);
        }

        #[unsafe(method(browserOpenExternal:))]
        fn browser_open_external(&self, _sender: Option<&AnyObject>) {
            let url = CURRENT_URL.lock().ok().map(|guard| guard.clone()).unwrap_or_default();
            if !url.trim().is_empty() {
                send_menu_event(UserEvent::OpenExternal(url));
            }
        }

        #[unsafe(method(browserClose:))]
        fn browser_close(&self, _sender: Option<&AnyObject>) {
            // 与工具栏关闭按钮一致：清空当前页面，而不是退出进程。
            send_menu_event(UserEvent::ClearBrowser);
        }
    }
);

impl MenuTarget {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(MenuTargetIvars);
        unsafe { msg_send![super(this), init] }
    }
}

/// 兼容旧调用名。
pub fn install_edit_menu(
    proxy: EventLoopProxy<UserEvent>,
    initial_url: &str,
    locale: config_ui::DesktopLocale,
) {
    install_native_menu(proxy, initial_url, locale);
}

/// 安装 macOS 主菜单。
fn install_native_menu(
    proxy: EventLoopProxy<UserEvent>,
    initial_url: &str,
    locale: config_ui::DesktopLocale,
) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };

    if let Ok(mut guard) = MENU_PROXY.lock() {
        *guard = Some(proxy);
    }
    set_current_url(initial_url);

    let i18n = desktop_i18n::I18n::new(locale);
    let app = NSApplication::sharedApplication(mtm);
    let menubar = NSMenu::new(mtm);
    let target = MenuTarget::new(mtm);

    // 首个菜单会被系统提升为应用菜单。
    let app_menu_item = NSMenuItem::new(mtm);
    menubar.addItem(&app_menu_item);
    let app_menu = NSMenu::new(mtm);
    app_menu_item.setSubmenu(Some(&app_menu));

    append_submenu(&menubar, "编辑", EDIT_MENU_ACTIONS, Some(2), None, mtm, i18n);
    append_browser_submenu(&menubar, &target, initial_url, mtm, i18n);
    let window_menu = append_submenu(&menubar, "窗口", WINDOW_MENU_ACTIONS, None, None, mtm, i18n);
    let help_menu = append_submenu(&menubar, "帮助", HELP_MENU_ACTIONS, None, None, mtm, i18n);

    // 保持 target 存活，菜单项会弱引用它作为 action target。
    thread_local! {
        static TARGET: std::cell::RefCell<Option<Retained<MenuTarget>>> = const { std::cell::RefCell::new(None) };
    }
    TARGET.with(|slot| *slot.borrow_mut() = Some(target));

    app.setMainMenu(Some(&menubar));
    // 注册后系统会注入 Fill / Center / Move & Resize / Tile 等标准窗口项。
    app.setWindowsMenu(Some(&window_menu));
    app.setHelpMenu(Some(&help_menu));
}

/// 同步当前页面地址，供「在外部浏览器中打开」使用。
pub fn set_current_url(url: &str) {
    if let Ok(mut guard) = CURRENT_URL.lock() {
        *guard = url.to_string();
    }
}

fn send_menu_event(event: UserEvent) {
    let Ok(guard) = MENU_PROXY.lock() else {
        return;
    };
    if let Some(proxy) = guard.as_ref() {
        let _ = proxy.send_event(event);
    }
}

fn append_browser_submenu(
    menubar: &NSMenu,
    target: &MenuTarget,
    initial_url: &str,
    mtm: MainThreadMarker,
    i18n: desktop_i18n::I18n,
) {
    let menu_item = NSMenuItem::new(mtm);
    menubar.addItem(&menu_item);
    let menu = NSMenu::new(mtm);
    menu.setTitle(&NSString::from_str(i18n.__("浏览器")));
    menu_item.setSubmenu(Some(&menu));

    let status_title = if initial_url.trim().is_empty() {
        i18n.__("页面：（空）").to_string()
    } else {
        i18n.format("页面：%s", &[initial_url.into()]).expect("valid source template")
    };
    let status = unsafe {
        menu.addItemWithTitle_action_keyEquivalent(
            &NSString::from_str(&status_title),
            None,
            &NSString::from_str(""),
        )
    };
    status.setEnabled(false);
    menu.addItem(&NSMenuItem::separatorItem(mtm));

    for (index, action) in BROWSER_MENU_ACTIONS.iter().enumerate() {
        if index == 3 {
            menu.addItem(&NSMenuItem::separatorItem(mtm));
        }
        add_targeted_menu_item(&menu, action, target, i18n);
    }
}

fn append_submenu(
    menubar: &NSMenu,
    title: &str,
    actions: &[MenuAction],
    separator_before_index: Option<usize>,
    target: Option<&MenuTarget>,
    mtm: MainThreadMarker,
    i18n: desktop_i18n::I18n,
) -> Retained<NSMenu> {
    let menu_item = NSMenuItem::new(mtm);
    menubar.addItem(&menu_item);
    let menu = NSMenu::new(mtm);
    menu.setTitle(&NSString::from_str(i18n.__(title)));
    menu_item.setSubmenu(Some(&menu));

    for (index, action) in actions.iter().enumerate() {
        if separator_before_index == Some(index) {
            menu.addItem(&NSMenuItem::separatorItem(mtm));
        }
        if let Some(target) = target {
            add_targeted_menu_item(&menu, action, target, i18n);
        } else {
            add_menu_item(&menu, action, i18n);
        }
    }
    menu
}

fn add_targeted_menu_item(
    menu: &NSMenu,
    action: &MenuAction,
    target: &MenuTarget,
    i18n: desktop_i18n::I18n,
) {
    let selector = match action.selector {
        "browserBack:" => sel!(browserBack:),
        "browserForward:" => sel!(browserForward:),
        "browserReload:" => sel!(browserReload:),
        "browserOpenExternal:" => sel!(browserOpenExternal:),
        "browserClose:" => sel!(browserClose:),
        _ => return,
    };

    let item = unsafe {
        menu.addItemWithTitle_action_keyEquivalent(
            &NSString::from_str(i18n.__(action.title)),
            Some(selector),
            &NSString::from_str(action.key),
        )
    };
    unsafe {
        // MenuTarget 是 NSObject 子类；setTarget 需要 &AnyObject。
        item.setTarget(Some(&*(target as *const MenuTarget).cast::<AnyObject>()));
    }
    apply_key_modifiers(&item, action);
}

fn add_menu_item(menu: &NSMenu, action: &MenuAction, i18n: desktop_i18n::I18n) {
    let selector = match action.selector {
        "undo:" => sel!(undo:),
        "redo:" => sel!(redo:),
        "cut:" => sel!(cut:),
        "copy:" => sel!(copy:),
        "paste:" => sel!(paste:),
        "selectAll:" => sel!(selectAll:),
        "performMiniaturize:" => sel!(performMiniaturize:),
        "performZoom:" => sel!(performZoom:),
        "toggleFullScreen:" => sel!(toggleFullScreen:),
        "orderFrontStandardAboutPanel:" => sel!(orderFrontStandardAboutPanel:),
        _ => return,
    };

    let item = unsafe {
        menu.addItemWithTitle_action_keyEquivalent(
            &NSString::from_str(i18n.__(action.title)),
            Some(selector),
            &NSString::from_str(action.key),
        )
    };
    apply_key_modifiers(&item, action);
}

fn apply_key_modifiers(item: &NSMenuItem, action: &MenuAction) {
    let mut modifiers = NSEventModifierFlags::Command;
    if action.shift {
        modifiers |= NSEventModifierFlags::Shift;
    }
    if action.control {
        modifiers |= NSEventModifierFlags::Control;
    }
    if action.key.is_empty() {
        item.setKeyEquivalentModifierMask(NSEventModifierFlags::empty());
    } else if action.shift || action.control {
        item.setKeyEquivalentModifierMask(modifiers);
    }
}
