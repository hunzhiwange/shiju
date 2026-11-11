//! Web-only DOM bridge into the existing Iced input method event stream.
use crate::core::{Event, Rectangle, input_method, window};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use wasm_bindgen::{JsCast, JsValue, closure::Closure, prelude::wasm_bindgen};

#[wasm_bindgen(module = "/src/web_ime.js")]
extern "C" {
    type BrowserIme;

    #[wasm_bindgen(catch, js_name = createImeBridge)]
    fn create_ime_bridge(
        canvas: &web_sys::HtmlCanvasElement,
        callback: &js_sys::Function,
    ) -> Result<BrowserIme, JsValue>;

    #[wasm_bindgen(method)]
    fn update(
        this: &BrowserIme,
        enabled: bool,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        secure: bool,
    );

    #[wasm_bindgen(method, js_name = hasFocus)]
    fn has_focus(this: &BrowserIme) -> bool;

    #[wasm_bindgen(method)]
    fn dispose(this: &BrowserIme);
}

pub(crate) struct WebIme {
    browser: BrowserIme,
    pending: Rc<RefCell<Vec<Event>>>,
    // Retain the callback until JS listeners have been detached in Drop.
    _callback: Closure<dyn FnMut(String, String)>,
}

impl WebIme {
    pub(crate) fn new(
        raw: &Arc<winit::window::Window>,
    ) -> Result<Self, JsValue> {
        use winit::platform::web::WindowExtWebSys;
        let canvas = raw
            .canvas()
            .ok_or_else(|| JsValue::from_str("Web canvas unavailable"))?;
        let pending = Rc::new(RefCell::new(Vec::new()));
        let output = Rc::clone(&pending);
        let raw = Arc::downgrade(raw);
        let callback = Closure::new(move |kind: String, text: String| {
            let event = match kind.as_str() {
                "opened" => Event::InputMethod(input_method::Event::Opened),
                "closed" => Event::InputMethod(input_method::Event::Closed),
                "preedit" => {
                    let end = text.len();
                    Event::InputMethod(input_method::Event::Preedit(
                        text,
                        Some(end..end),
                    ))
                }
                "commit" => {
                    Event::InputMethod(input_method::Event::Commit(text))
                }
                "focus" => Event::Window(window::Event::Focused),
                "blur" => Event::Window(window::Event::Unfocused),
                _ => return,
            };
            output.borrow_mut().push(event);
            if let Some(raw) = raw.upgrade() {
                raw.request_redraw();
            }
        });
        let browser =
            create_ime_bridge(&canvas, callback.as_ref().unchecked_ref())?;
        Ok(Self {
            browser,
            pending,
            _callback: callback,
        })
    }

    pub(crate) fn update(
        &self,
        cursor: Option<Rectangle>,
        secure: bool,
        scale: f32,
    ) {
        let rect = cursor.unwrap_or_default();
        self.browser.update(
            cursor.is_some(),
            rect.x * scale,
            rect.y * scale,
            rect.width * scale,
            rect.height * scale,
            secure,
        );
    }

    pub(crate) fn has_focus(&self) -> bool {
        self.browser.has_focus()
    }

    pub(crate) fn take_events(&self) -> Vec<Event> {
        std::mem::take(&mut *self.pending.borrow_mut())
    }
}

impl Drop for WebIme {
    fn drop(&mut self) {
        self.browser.dispose();
    }
}
