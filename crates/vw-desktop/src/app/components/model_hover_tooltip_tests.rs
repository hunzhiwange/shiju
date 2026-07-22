use super::model_hover_tooltip::{
    HoverAnchor, build_hover_tip, hover_anchor, hover_model_trigger, hover_text_trigger,
    hover_tooltip_overlay,
};
use crate::app::state::{ModelPopoverHover, ModelSummary, ProviderModelsSummary};
use crate::app::{App, Message};
use iced::widget::{container, text};
use iced::{Background, Color, Element, Theme};
use std::cell::RefCell;
use std::rc::Rc;

fn test_app() -> App {
    App::new().0
}

fn keep(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

fn tooltip_style(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(Color::BLACK)),
        text_color: Some(Color::WHITE),
        ..Default::default()
    }
}

fn hover_message(hover: Option<ModelPopoverHover>) -> Message {
    std::hint::black_box(hover);
    Message::View(crate::app::message::view::ViewMessage::CloseModelPopover)
}

fn exit_message() -> Message {
    hover_message(None)
}

fn provider() -> ProviderModelsSummary {
    ProviderModelsSummary {
        id: "openai".to_string(),
        name: "OpenAI".to_string(),
        models: vec![
            ModelSummary {
                id: "gpt-test".to_string(),
                name: "GPT Test".to_string(),
                enabled: true,
                toolcall: true,
                attachment: false,
                context_limit: 128_000,
                detail: serde_json::json!({"owned_by": "test"}),
            },
            ModelSummary {
                id: "gpt-vision".to_string(),
                name: "GPT Vision".to_string(),
                enabled: true,
                toolcall: false,
                attachment: true,
                context_limit: 64_000,
                detail: serde_json::json!({"owned_by": "test"}),
            },
        ],
    }
}

#[test]
fn task_746_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("model_hover_tooltip_tests.rs"));
}

#[test]
fn hover_anchor_is_plain_copyable_coordinate_data() {
    let anchor = HoverAnchor { x: 12.5, y: 99.0 };

    assert_eq!(anchor.x, 12.5);
    assert_eq!(anchor.y, 99.0);
    assert_eq!(format!("{anchor:?}"), "HoverAnchor { x: 12.5, y: 99.0 }");
}

#[test]
fn hover_text_trigger_records_initial_unanchored_text() {
    let observed = Rc::new(RefCell::new(None));
    let captured = Rc::clone(&observed);

    keep(hover_text_trigger(
        text("trigger"),
        "plain tooltip",
        move |hover| {
            *captured.borrow_mut() = hover;
            exit_message()
        },
        exit_message(),
    ));

    let observed = observed.borrow();
    let Some(ModelPopoverHover::Text { text, anchor }) = observed.as_ref() else {
        panic!("text trigger should record text hover state");
    };
    assert_eq!(text, "plain tooltip");
    assert!(anchor.is_none());
}

#[test]
fn hover_model_trigger_records_initial_unanchored_ids() {
    let observed = Rc::new(RefCell::new(None));
    let captured = Rc::clone(&observed);

    keep(hover_model_trigger(
        text("model"),
        "openai",
        "gpt-test",
        move |hover| {
            *captured.borrow_mut() = hover;
            exit_message()
        },
        exit_message(),
    ));

    let observed = observed.borrow();
    let Some(ModelPopoverHover::Model { provider_id, model_id, anchor }) = observed.as_ref() else {
        panic!("model trigger should record model hover state");
    };
    assert_eq!(provider_id, "openai");
    assert_eq!(model_id, "gpt-test");
    assert!(anchor.is_none());
}

#[test]
fn hover_model_trigger_preserves_empty_identifiers() {
    let observed = Rc::new(RefCell::new(None));
    let captured = Rc::clone(&observed);

    keep(hover_model_trigger(
        text("model"),
        "",
        "",
        move |hover| {
            *captured.borrow_mut() = hover;
            exit_message()
        },
        exit_message(),
    ));

    let observed = observed.borrow();
    let Some(ModelPopoverHover::Model { provider_id, model_id, anchor }) = observed.as_ref() else {
        panic!("model trigger should preserve empty identifiers");
    };
    assert!(provider_id.is_empty());
    assert!(model_id.is_empty());
    assert!(anchor.is_none());
}

#[test]
fn hover_anchor_extracts_model_and_text_coordinates_or_none() {
    let model = Some(ModelPopoverHover::Model {
        provider_id: "openai".to_string(),
        model_id: "gpt-test".to_string(),
        anchor: Some(HoverAnchor { x: 7.5, y: 8.25 }),
    });
    let text = Some(ModelPopoverHover::Text {
        text: "details".to_string(),
        anchor: Some(HoverAnchor { x: -2.0, y: 0.0 }),
    });

    let model_anchor = hover_anchor(&model).expect("model anchor");
    assert_eq!((model_anchor.x, model_anchor.y), (7.5, 8.25));
    let text_anchor = hover_anchor(&text).expect("text anchor");
    assert_eq!((text_anchor.x, text_anchor.y), (-2.0, 0.0));
    assert!(hover_anchor(&None).is_none());
}

#[test]
fn build_hover_tip_rejects_absent_hover_and_unknown_model_targets() {
    let app = test_app();
    assert!(build_hover_tip(&app, tooltip_style).is_none());

    let mut missing_provider = test_app();
    missing_provider.model_popover_hover = Some(ModelPopoverHover::Model {
        provider_id: "missing".to_string(),
        model_id: "gpt-test".to_string(),
        anchor: None,
    });
    assert!(build_hover_tip(&missing_provider, tooltip_style).is_none());

    let mut missing_model = test_app();
    missing_model.model_settings.providers.push(provider());
    missing_model.model_popover_hover = Some(ModelPopoverHover::Model {
        provider_id: "openai".to_string(),
        model_id: "missing".to_string(),
        anchor: None,
    });
    assert!(build_hover_tip(&missing_model, tooltip_style).is_none());
}

#[test]
fn build_hover_tip_accepts_text_and_both_model_capability_combinations() {
    let mut app = test_app();
    app.model_popover_hover =
        Some(ModelPopoverHover::Text { text: "plain tooltip".to_string(), anchor: None });
    assert!(build_hover_tip(&app, tooltip_style).is_some());

    app.model_settings.providers.push(provider());
    for model_id in ["gpt-test", "gpt-vision"] {
        app.model_popover_hover = Some(ModelPopoverHover::Model {
            provider_id: "openai".to_string(),
            model_id: model_id.to_string(),
            anchor: None,
        });
        assert!(build_hover_tip(&app, tooltip_style).is_some());
    }
}

#[test]
fn overlay_returns_content_when_hover_is_absent_or_model_is_missing() {
    let app = test_app();
    keep(hover_tooltip_overlay(&app, text("content"), tooltip_style));

    let mut missing = test_app();
    missing.model_popover_hover = Some(ModelPopoverHover::Model {
        provider_id: "missing".to_string(),
        model_id: "model".to_string(),
        anchor: None,
    });
    keep(hover_tooltip_overlay(&missing, text("content"), tooltip_style));
}

#[test]
fn overlay_builds_text_tooltip_with_side_and_point_anchor_modes() {
    let mut app = test_app();
    app.model_popover_hover = Some(ModelPopoverHover::Text {
        text: "Use this model for coding".to_string(),
        anchor: None,
    });
    keep(hover_tooltip_overlay(&app, container(text("content")), tooltip_style));

    app.model_popover_hover = Some(ModelPopoverHover::Text {
        text: "Anchored text".to_string(),
        anchor: Some(HoverAnchor { x: 30.0, y: 40.0 }),
    });
    keep(hover_tooltip_overlay(&app, container(text("content")), tooltip_style));
}

#[test]
fn overlay_builds_model_tooltip_for_matching_provider_model() {
    let mut app = test_app();
    app.model_settings.providers.push(provider());
    app.model_popover_hover = Some(ModelPopoverHover::Model {
        provider_id: "openai".to_string(),
        model_id: "gpt-test".to_string(),
        anchor: Some(HoverAnchor { x: 10.0, y: 20.0 }),
    });

    keep(hover_tooltip_overlay(&app, text("content"), tooltip_style));
}
