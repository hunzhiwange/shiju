use tao::event::{Event, WindowEvent};
use tao::event_loop::ControlFlow;
use webview_core::events::UserEvent;
use webview_core::navigation::{ExternalTarget, is_safe_navigation_url};
use webview_launch::DEFAULT_TARGET;
use webview_toolbar::{toolbar_address_script, window_regions};
use webview_browser_views::to_wry_rect;

pub(crate) struct BrowserEvents {
    pub(crate) window: tao::window::Window,
    pub(crate) webview: wry::WebView,
    pub(crate) toolbar_webview: Option<wry::WebView>,
    pub(crate) popups: webview_popup::PopupWindows,
    pub(crate) external: webview_external_protocol::ExternalProtocols,
    pub(crate) proxy: tao::event_loop::EventLoopProxy<UserEvent>,
    pub(crate) embedded_mode: bool,
    pub(crate) independent_mode: bool,
    pub(crate) current_url: String,
    pub(crate) browser_cleared: bool,
}

impl BrowserEvents {
    pub(crate) fn handle(&mut self, event: Event<'_, UserEvent>, control_flow: &mut ControlFlow) {
        let Self {
            window, webview, toolbar_webview, popups, external, proxy,
            embedded_mode, independent_mode, current_url, browser_cleared,
        } = self;
        match event {
            Event::UserEvent(UserEvent::Resize(w, h)) => {
                window.set_inner_size(tao::dpi::LogicalSize::new(w as f64, h as f64));
            }
            Event::UserEvent(UserEvent::Move(x, y)) => {
                window.set_outer_position(tao::dpi::LogicalPosition::new(x as f64, y as f64));
            }
            Event::UserEvent(UserEvent::OpenUrl(u)) => {
                if is_safe_navigation_url(&u) {
                    *browser_cleared = false;
                    let _ = webview.load_url(&u);
                } else if let Some(target) = ExternalTarget::parse(&u) {
                    external.request(target, None, proxy);
                    if let Some(toolbar) = toolbar_webview.as_ref()
                        && let Ok(script) = toolbar_address_script(current_url)
                    {
                        let _ = toolbar.evaluate_script(&script);
                    }
                }
            }
            Event::UserEvent(UserEvent::PopupClosed(id)) => popups.close(id),
            Event::UserEvent(UserEvent::OpenExternal(u)) => {
                if is_safe_navigation_url(&u) {
                    if open::that(u).is_err() {
                        webview_external_protocol::show_browser_open_error(window);
                    }
                } else if let Some(target) = ExternalTarget::parse(&u) {
                    external.request(target, None, proxy);
                    if let Some(toolbar) = toolbar_webview.as_ref()
                        && let Ok(script) = toolbar_address_script(current_url)
                    {
                        let _ = toolbar.evaluate_script(&script);
                    }
                }
            }
            Event::UserEvent(UserEvent::ConfirmExternal) => {
                if let Some((target, popup, confirmation)) = external.take() {
                    if let Some(id) = popup {
                        popups.show_external_confirmation(id, target, confirmation);
                    } else if let Some(parent) = webview_external_protocol::main_dialog_parent(window) {
                        webview_external_protocol::show_confirmation(parent, target, confirmation);
                    }
                }
            }
            Event::UserEvent(UserEvent::NavigationChanged(u)) => {
                if *browser_cleared {
                    current_url.clear();
                } else {
                    current_url.clone_from(&u);
                    if *embedded_mode {
                        crate::control_pipe::emit_navigation_url(&u);
                    }
                    if let Some(toolbar) = toolbar_webview.as_ref()
                        && let Ok(script) = toolbar_address_script(&u)
                    {
                        let _ = toolbar.evaluate_script(&script);
                    }
                }
                #[cfg(target_os = "macos")]
                webview_native_menu::set_current_url(current_url);
            }
            Event::UserEvent(UserEvent::ToolbarReady) => {
                if let Some(toolbar) = toolbar_webview.as_ref()
                    && let Ok(script) = toolbar_address_script(current_url)
                {
                    let _ = toolbar.evaluate_script(&script);
                }
            }
            Event::UserEvent(UserEvent::Back) => {
                if !*browser_cleared {
                    let _ = webview.evaluate_script("history.back()");
                }
            }
            Event::UserEvent(UserEvent::Forward) => {
                if !*browser_cleared {
                    let _ = webview.evaluate_script("history.forward()");
                }
            }
            Event::UserEvent(UserEvent::Reload) => {
                if !*browser_cleared {
                    let _ = webview.reload();
                }
            }
            Event::UserEvent(UserEvent::SetAlwaysOnTop(always_on_top)) => {
                window.set_always_on_top(always_on_top);
            }
            Event::UserEvent(UserEvent::Show) => {
                window.set_visible(true);
            }
            Event::UserEvent(UserEvent::Hide) => {
                window.set_visible(false);
            }
            Event::UserEvent(UserEvent::Close) => {
                *control_flow = ControlFlow::Exit;
            }
            Event::UserEvent(UserEvent::ClearBrowser) => {
                *browser_cleared = true;
                current_url.clear();
                #[cfg(target_os = "macos")]
                webview_native_menu::set_current_url("");
                let _ = webview.load_url(DEFAULT_TARGET);
                if let Some(toolbar) = toolbar_webview.as_ref()
                    && let Ok(script) = toolbar_address_script("")
                {
                    let _ = toolbar.evaluate_script(&script);
                }
            }
            Event::WindowEvent { window_id, event: WindowEvent::Resized(size), .. }
                if window_id == window.id() && *independent_mode =>
            {
                let size = size.to_logical::<u32>(window.scale_factor());
                let (toolbar_region, content_region) =
                    window_regions(size.width, size.height, true);
                if let (Some(toolbar), Some(toolbar_region)) =
                    (toolbar_webview.as_ref(), toolbar_region)
                {
                    let _ = toolbar.set_bounds(to_wry_rect(toolbar_region));
                }
                let _ = webview.set_bounds(to_wry_rect(content_region));
            }
            Event::WindowEvent { window_id, event: WindowEvent::CloseRequested, .. }
                if window_id == window.id() =>
            {
                *control_flow = ControlFlow::Exit;
            }
            _ => (),
        }
    }
}
