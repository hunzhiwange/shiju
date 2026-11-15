//! 预先打开授权窗口，避免异步取得登录地址后被浏览器拦截。

use std::cell::RefCell;

thread_local! {
    static AUTHORIZATION_WINDOW: RefCell<Option<web_sys::Window>> = const { RefCell::new(None) };
}

pub(crate) fn prepare() -> bool {
    close();
    let Some(window) = web_sys::window()
        .and_then(|window| window.open_with_url_and_target("about:blank", "_blank").ok().flatten())
    else {
        return false;
    };
    // 授权站点不能通过 opener 操作应用页面。
    if js_sys::Reflect::set(window.as_ref(), &"opener".into(), &wasm_bindgen::JsValue::NULL)
        != Ok(true)
    {
        let _ = window.close();
        return false;
    }
    AUTHORIZATION_WINDOW.with(|slot| *slot.borrow_mut() = Some(window));
    true
}

pub(crate) fn navigate(url: &str) -> bool {
    AUTHORIZATION_WINDOW.with(|slot| {
        // 进入跨站授权页后释放句柄；opener 已清除，浏览器不允许应用关闭该页。
        slot.borrow_mut().take().is_some_and(|window| {
            !window.closed().unwrap_or(true)
                && window.open_with_url_and_target(url, "_self").ok().flatten().is_some()
        })
    })
}

pub(crate) fn close() {
    AUTHORIZATION_WINDOW.with(|slot| {
        if let Some(window) = slot.borrow_mut().take() {
            let _ = window.close();
        }
    });
}
