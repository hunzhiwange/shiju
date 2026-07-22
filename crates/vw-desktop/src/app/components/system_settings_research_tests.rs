use super::*;
use iced::Element;
use iced::advanced::widget::Tree;
use iced::widget::text;
use vw_config_types::automation::ResearchTrigger;

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn trigger_label_maps_all_trigger_variants() {
    assert_eq!(trigger_label(ResearchTrigger::Never), "从不");
    assert_eq!(trigger_label(ResearchTrigger::Always), "总是");
    assert_eq!(trigger_label(ResearchTrigger::Keywords), "关键词");
    assert_eq!(trigger_label(ResearchTrigger::Length), "长度");
    assert_eq!(trigger_label(ResearchTrigger::Question), "问句");
}

#[test]
fn parse_trigger_label_maps_known_labels_and_defaults_to_never() {
    assert_eq!(parse_trigger_label("总是"), ResearchTrigger::Always);
    assert_eq!(parse_trigger_label("关键词"), ResearchTrigger::Keywords);
    assert_eq!(parse_trigger_label("长度"), ResearchTrigger::Length);
    assert_eq!(parse_trigger_label("问句"), ResearchTrigger::Question);
    assert_eq!(parse_trigger_label("从不"), ResearchTrigger::Never);
    assert_eq!(parse_trigger_label("unknown"), ResearchTrigger::Never);
}

#[test]
fn parse_trigger_label_defaults_non_exact_inputs_to_never() {
    for value in ["", " 总是", "总是 ", "always", "未知"] {
        assert_eq!(parse_trigger_label(value), ResearchTrigger::Never);
    }
}

#[test]
fn field_and_text_rows_accept_research_controls() {
    let field: Element<'_, Message> =
        field_row("触发方式", "定义进入 Research 阶段的条件。", text("从不"));
    let text: Element<'_, Message> = text_row(
        "关键词",
        "仅在触发方式为关键词时生效。",
        "find",
        "search",
        |_| Message::GatewayHealthTick,
    );

    drop(field);
    drop(text);
}

#[test]
fn view_builds_every_trigger_and_boolean_state() {
    let mut app = test_app();

    for trigger in [
        ResearchTrigger::Never,
        ResearchTrigger::Always,
        ResearchTrigger::Keywords,
        ResearchTrigger::Length,
        ResearchTrigger::Question,
    ] {
        for enabled in [false, true] {
            for show_progress in [false, true] {
                app.research_settings.trigger = trigger;
                app.research_settings.enabled = enabled;
                app.research_settings.show_progress = show_progress;
                keep_element(view(&app));
            }
        }
    }
}

#[test]
fn view_builds_slider_boundaries() {
    let mut app = test_app();

    for (min_message_length, max_iterations) in [(1, 1), (10_000, 100)] {
        app.research_settings.min_message_length = min_message_length;
        app.research_settings.max_iterations = max_iterations;
        keep_element(view(&app));
    }
}

#[test]
fn view_builds_empty_and_unicode_text_inputs() {
    let mut app = test_app();

    for (keywords, prompt_prefix) in [
        ("", ""),
        ("find, search\ninvestigate", "Collect evidence before answering."),
        ("检索, 调研, α", "先核对资料，再回答。"),
    ] {
        app.research_settings.keywords_input = keywords.to_string();
        app.research_settings.system_prompt_prefix = prompt_prefix.to_string();
        keep_element(view(&app));
    }
}

#[test]
fn overlays_return_dialog_or_build_help_modal() {
    let mut app = test_app();
    keep_element(view_overlays(&app, text("dialog").into()));

    app.research_settings.show_help_modal = true;
    keep_element(view_overlays(&app, text("dialog").into()));
}

#[test]
fn view_keeps_keyword_hint_between_two_settings_panels() {
    let app = test_app();
    let element = view(&app);
    let tree = Tree::new(element.as_widget());

    assert_eq!(tree.children.len(), 6);
    assert_eq!(tree.children[2].children.len(), 5);
    assert_eq!(tree.children[5].children.len(), 7);
}
