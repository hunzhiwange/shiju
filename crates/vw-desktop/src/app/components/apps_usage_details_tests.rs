use chrono::{DateTime, Utc};
use iced::{
    Element, Event, Font, Rectangle, Renderer, Size,
    advanced::{clipboard, renderer::Headless, widget::operation::Operation},
    mouse,
};
use iced_runtime::user_interface::{Cache, UserInterface};
use serde_json::{Value, json};
use vw_gateway_client::vw_api_types::apps_usage::UsageSnapshot;

use super::apps_usage_details::view;
use crate::app::{Message, message::ViewMessage};

const LINK: &str = "https://platform.deepseek.com/usage";

fn snapshot(source: &str, data: Value) -> UsageSnapshot {
    serde_json::from_value(json!({
        "stale": false,
        "result": {
            "source": { "kind": source, "id": "fixture" },
            "queriedAt": 1735689600000_i64,
            "credentialState": "valid",
            "outcome": { "status": "success", "data": data }
        }
    }))
    .unwrap()
}

#[derive(Default)]
struct Texts(Vec<(String, Rectangle)>);

impl Operation for Texts {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
        children(self);
    }

    fn text(&mut self, _: Option<&iced::advanced::widget::Id>, bounds: Rectangle, value: &str) {
        self.0.push((value.into(), bounds));
    }
}

struct Scene<'a> {
    ui: UserInterface<'a, Message, iced::Theme, Renderer>,
    texts: Texts,
}

impl<'a> Scene<'a> {
    fn new(element: Element<'a, Message>, renderer: &mut Renderer) -> Self {
        let mut ui = UserInterface::build(element, Size::new(640.0, 700.0), Cache::new(), renderer);
        let mut texts = Texts::default();
        ui.operate(renderer, &mut texts);
        Self { ui, texts }
    }

    fn bounds(&self, label: &str) -> Option<Rectangle> {
        self.texts.0.iter().find(|(value, _)| value == label).map(|(_, bounds)| *bounds)
    }

    fn contains(&self, fragment: &str) -> bool {
        self.texts.0.iter().any(|(value, _)| value.contains(fragment))
    }

    fn send(&mut self, renderer: &mut Renderer, label: &str, events: &[Event]) -> Vec<Message> {
        let point = self.bounds(label).expect(label).center();
        let mut messages = Vec::new();
        self.ui.update(
            events,
            mouse::Cursor::Available(point),
            renderer,
            &mut clipboard::Null,
            &mut messages,
        );
        messages.retain(|message| !matches!(message, Message::None));
        messages
    }
}

async fn renderer() -> Renderer {
    static FONTS: std::sync::Once = std::sync::Once::new();
    FONTS.call_once(|| {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    });
    Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
        .await
        .expect("tiny-skia renderer")
}

fn now() -> DateTime<Utc> {
    "2025-01-01T00:00:00Z".parse().unwrap()
}

fn wallet(source: &str, url: &str) -> UsageSnapshot {
    snapshot(
        source,
        json!({
            "kind": "balance",
            "value": [{
                "currency": "CNY",
                "remaining": -3,
                "total": 20,
                "used": 23,
                "granted": 0,
                "toppedUp": 20,
                "detailUrl": url
            }]
        }),
    )
}

#[tokio::test]
async fn usage_usage_details_returns_expected_result() {
    let mut renderer = renderer().await;
    let official = snapshot(
        "official",
        json!({
            "kind": "official_subscription",
            "value": {
                "windows": [
                    { "name": "five_hour", "planLabel": "短时", "utilization": 25,
                      "resetsAt": "2025-01-01T05:00:00Z" },
                    { "name": "seven_day", "planLabel": "每周", "remaining": 15, "unit": "requests" }
                ],
                "creditsBalance": 100,
                "resetCredits": [null, "2025-01-03T02:00:00Z",
                    "2024-12-30T00:00:00Z", "2025-01-03T01:00:00Z"]
            }
        }),
    );
    let mut closed = Scene::new(
        view(&official, &now(), false, Message::CopyCode("toggle".into())),
        &mut renderer,
    );
    assert!(!closed.contains("短时"));
    let click = [
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
    ];
    assert!(matches!(
        closed.send(&mut renderer, "展开额度明细", &click).as_slice(),
        [Message::CopyCode(value)] if value == "toggle"
    ));
    let opened = Scene::new(view(&official, &now(), true, Message::None), &mut renderer);
    for visible in [
        "官方额度",
        "fixture",
        "2025-01-01 00:00:00",
        "短时",
        "75%",
        "每周",
        "15 requests",
        "2025-01-01 05:00:00",
        "2025-01-03 · 2 次",
        "永不过期 · 1 次",
        "Credits",
        "估算",
    ] {
        assert!(opened.contains(visible), "missing {visible}");
    }
    assert!(!opened.contains("2024-12-30"));

    for (source, label) in
        [("provider", "来源 · 供应商查询"), ("estimate", "来源 · 估算"), ("billing", "来源 · 账单")]
    {
        let wallet = wallet(source, LINK);
        let mut opened = Scene::new(view(&wallet, &now(), true, Message::None), &mut renderer);
        for visible in [label, "余额明细", "剩余 -3 CNY", "赠金 0 CNY", "充值 20 CNY"] {
            assert!(opened.contains(visible), "missing {visible}");
        }
        let link = "在浏览器核对额度";
        if source != "provider" {
            assert!(opened.bounds(link).is_none());
            continue;
        }
        assert!(
            opened
                .send(
                    &mut renderer,
                    link,
                    &[Event::Mouse(mouse::Event::CursorMoved {
                        position: opened.bounds(link).unwrap().center(),
                    })]
                )
                .is_empty()
        );
        assert!(matches!(
            opened.send(&mut renderer, link, &click).as_slice(),
            [Message::View(ViewMessage::OpenUrlExternal(url))] if url == LINK
        ));
    }
}

#[tokio::test]
async fn usage_usage_details_rejects_invalid_input() {
    let mut renderer = renderer().await;
    for url in [
        "http://platform.deepseek.com/usage",
        "https://user@platform.deepseek.com/usage",
        "https:///platform.deepseek.com/usage",
        "https://platform.deepseek.com/\nusage",
        "https://platform.deepseek.com\\@other.invalid/usage",
        "file:///tmp/usage",
        "javascript:alert(1)",
    ] {
        let snapshot = wallet("provider", url);
        let scene = Scene::new(view(&snapshot, &now(), true, Message::None), &mut renderer);
        assert!(scene.bounds("在浏览器核对额度").is_none(), "{url:?}");
        assert!(scene.contains("剩余 -3 CNY"));
    }
}
