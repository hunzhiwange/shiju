use crate::{model_routes::validate_routes, state::ModelRoute};

#[test]
fn model_routes_validate_required_fields() {
    let mut routes = Vec::new();
    routes = vec![ModelRoute {
        pattern: "  ".to_string(),
        provider: "openai".to_string(),
        model: "gpt-5".to_string(),
        priority_input: "1".to_string(),
    }];
    assert!(validate_routes(&routes).unwrap_err().contains("pattern"));

    routes[0].pattern = "chat".to_string();
    routes[0].priority_input = "oops".to_string();
    assert!(validate_routes(&routes).unwrap_err().contains("priority"));
}
