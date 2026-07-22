use super::*;
use crate::app::message::settings::{SettingsMessage, WebSearchMessage};
use crate::app::{App, Message};
use iced::Element;
use iced::advanced::widget::Tree;
use iced::widget::text;

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn provider_visibility_helpers_cover_every_supported_provider() {
    let cases = [
        ("duckduckgo", false, false, false),
        ("brave", true, true, true),
        ("serper", true, true, false),
        ("google", true, true, false),
        ("bing", true, true, false),
    ];

    for (provider, api_key, api_url, brave_api_key) in cases {
        assert_eq!(shows_api_key(provider), api_key, "provider={provider}");
        assert_eq!(shows_api_url(provider), api_url, "provider={provider}");
        assert_eq!(shows_brave_api_key(provider), brave_api_key, "provider={provider}");
    }
}

#[test]
fn unrecognized_providers_keep_generic_api_fields_visible() {
    for provider in ["", "DuckDuckGo", " duckduckgo", "duckduckgo ", "custom"] {
        assert!(shows_api_key(provider), "provider={provider:?}");
        assert!(shows_api_url(provider), "provider={provider:?}");
        assert!(!shows_brave_api_key(provider), "provider={provider:?}");
    }
}

#[test]
fn brave_api_key_visibility_requires_the_exact_provider_key() {
    assert!(shows_brave_api_key("brave"));

    for provider in ["", "BRAVE", "Brave", " brave", "brave ", "brave-search"] {
        assert!(!shows_brave_api_key(provider), "provider={provider:?}");
    }
}

#[test]
fn generic_api_key_and_url_visibility_stay_in_lockstep() {
    for provider in ["", "duckduckgo", "brave", "serper", "google", "bing", "custom"] {
        assert_eq!(shows_api_key(provider), shows_api_url(provider), "provider={provider:?}");
    }
}

#[test]
fn row_helpers_build_empty_and_populated_controls() {
    keep_element(field_row("", "", text("")));
    keep_element(field_row("标签", "说明", text("control")));

    for checked in [false, true] {
        keep_element(bool_row("启用", "说明", checked, "启用搜索", |value| {
            Message::Settings(SettingsMessage::WebSearch(WebSearchMessage::EnabledToggled(value)))
        }));
    }

    for value in ["", "key"] {
        keep_element(text_row("密钥", "说明", "placeholder", value, |value| {
            Message::Settings(SettingsMessage::WebSearch(WebSearchMessage::ApiKeyChanged(value)))
        }));
    }

    keep_element(hint_row(""));
    keep_element(hint_row("提示"));
}

#[test]
fn view_builds_supported_and_fallback_provider_layouts() {
    let mut app = test_app();

    for populated in [false, true] {
        for provider in ["duckduckgo", "brave", "serper", "google", "bing", "unknown"] {
            app.web_search_settings.provider = provider.to_string();
            app.web_search_settings.enabled = populated;
            app.web_search_settings.api_key_input =
                populated.then_some("api-key").unwrap_or("").into();
            app.web_search_settings.brave_api_key_input =
                populated.then_some("brave-key").unwrap_or("").into();
            app.web_search_settings.api_url_input =
                populated.then_some("https://search.example").unwrap_or("").into();
            app.web_search_settings.max_results_input =
                populated.then_some("10").unwrap_or("").into();
            app.web_search_settings.timeout_secs_input =
                populated.then_some("30").unwrap_or("").into();
            app.web_search_settings.user_agent =
                populated.then_some("VibeWindow/Test").unwrap_or("").into();
            keep_element(view(&app));
        }
    }
}

#[test]
fn provider_layouts_include_only_their_conditional_rows() {
    let mut app = test_app();

    for (provider, expected_rows) in [("duckduckgo", 8), ("brave", 12), ("serper", 10)] {
        app.web_search_settings.provider = provider.to_string();
        let element = view(&app);
        let tree = Tree::new(element.as_widget());

        assert_eq!(tree.children.len(), expected_rows, "provider={provider}");
    }
}

#[test]
fn overlays_return_dialog_or_help_modal() {
    let mut app = test_app();
    keep_element(view_overlays(&app, text("dialog").into()));

    app.web_search_settings.show_help_modal = true;
    keep_element(view_overlays(&app, text("dialog").into()));
}
