use super::{
    list_tool_specs_response, openai_tools_json, should_enable_question_tool, should_enable_search,
    should_use_apply_patch, spec_dtos, specs, specs_json,
};
use crate::app::agent::flag;
use serde_json::Value;

#[test]
fn search_is_enabled_for_vibewindow_prefix_or_exa_flag() {
    assert!(should_enable_search("vibewindow"));
    assert!(should_enable_search("vibewindow-custom"));
    assert_eq!(should_enable_search("openai"), *flag::VIBEWINDOW_ENABLE_EXA);
    assert_eq!(should_enable_search(""), *flag::VIBEWINDOW_ENABLE_EXA);
}

#[test]
fn apply_patch_is_enabled_for_any_model_identifier() {
    assert!(should_use_apply_patch(""));
    assert!(should_use_apply_patch("gpt-3.5-turbo"));
    assert!(should_use_apply_patch("claude-3-opus"));
}

#[test]
fn question_tool_is_limited_to_exact_interactive_client_names() {
    for client in ["app", "cli", "desktop"] {
        assert!(should_enable_question_tool(client), "{client} should be interactive");
    }

    for client in ["", "api", "webhook", "CLI", " cli "] {
        assert!(!should_enable_question_tool(client), "{client:?} should be rejected");
    }
}

#[test]
fn blank_models_use_the_same_default_tool_set() {
    let default_ids = specs(None).into_iter().map(|spec| spec.id).collect::<Vec<_>>();

    for model in [Some(""), Some("   \n")] {
        let ids = specs(model).into_iter().map(|spec| spec.id).collect::<Vec<_>>();
        assert_eq!(ids, default_ids);
    }
}

#[test]
fn dto_response_preserves_registry_order_and_fields() {
    let specs = specs(Some("openai/gpt-4"));
    let dtos = spec_dtos(Some("openai/gpt-4"));
    let response = list_tool_specs_response(Some("openai/gpt-4"));

    assert_eq!(dtos, response.items);
    assert_eq!(dtos.len(), specs.len());
    for (spec, dto) in specs.iter().zip(&dtos) {
        assert_eq!(dto.id.as_ref(), spec.id);
        assert_eq!(dto.display_name, spec.display_name);
        assert_eq!(dto.input_schema, spec.input_schema);
        assert_eq!(dto.strict, spec.strict);
    }
}

#[test]
fn json_views_expose_expected_shapes_and_sorted_openai_names() {
    let simplified = specs_json(Some("openai/gpt-4"));
    let simplified = simplified.as_array().expect("specs_json should return an array");
    assert!(!simplified.is_empty());
    assert!(simplified.iter().all(|item| {
        item.get("id").is_some()
            && item.get("description").is_some()
            && item.get("parameters").is_some()
    }));

    let openai = openai_tools_json(Some("openai/gpt-4"));
    let openai = openai.as_array().expect("openai_tools_json should return an array");
    let names = openai
        .iter()
        .map(|item| {
            assert_eq!(item["type"], Value::String("function".to_string()));
            item["function"]["name"].as_str().expect("function name should be a string").to_string()
        })
        .collect::<Vec<_>>();
    assert!(names.windows(2).all(|pair| pair[0] <= pair[1]));
}
