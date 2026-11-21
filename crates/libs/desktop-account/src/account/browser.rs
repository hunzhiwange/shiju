//! 预先打开授权窗口，避免异步取得登录地址后被浏览器拦截。

use std::cell::RefCell;

thread_local! {
    static AUTHORIZATION_WINDOW: RefCell<Option<web_sys::Window>> = const { RefCell::new(None) };
}

pub fn prepare() -> bool {
    close();
    let Some(window) = open_isolated_window() else {
        return false;
    };
    AUTHORIZATION_WINDOW.with(|slot| *slot.borrow_mut() = Some(window));
    true
}

pub fn open_external(url: &str) -> bool {
    let Some(window) = open_isolated_window() else {
        return false;
    };
    if navigate_window(&window, url) {
        return true;
    }
    let _ = window.close();
    false
}

fn open_isolated_window() -> Option<web_sys::Window> {
    let window = web_sys::window()?.open_with_url_and_target("about:blank", "_blank").ok()??;
    // 授权站点不能通过 opener 操作应用页面。
    if js_sys::Reflect::set(window.as_ref(), &"opener".into(), &wasm_bindgen::JsValue::NULL)
        != Ok(true)
    {
        let _ = window.close();
        return None;
    }
    Some(window)
}

pub fn navigate(url: &str) -> bool {
    AUTHORIZATION_WINDOW.with(|slot| {
        // 进入跨站授权页后释放句柄；opener 已清除，浏览器不允许应用关闭该页。
        slot.borrow_mut().take().is_some_and(|window| navigate_window(&window, url))
    })
}

fn navigate_window(window: &web_sys::Window, url: &str) -> bool {
    !window.closed().unwrap_or(true)
        && window.open_with_url_and_target(url, "_self").ok().flatten().is_some()
}

pub fn close() {
    AUTHORIZATION_WINDOW.with(|slot| {
        if let Some(window) = slot.borrow_mut().take() {
            let _ = window.close();
        }
    });
}
