use super::config::{
    AccountUsage, NativeTemplate, PreparedUsage, ProviderDefaults, ScriptParameter,
    ScriptParameters, TemplateError, UsageTemplate,
};
use crate::usage::detect::{ZhipuPlanSelection, detect_native_usage};

#[test]
fn usage_scripts_config_returns_expected_result() {
    let provider = ProviderDefaults {
        base_url: Some("https://provider.example/v1///"),
        api_key: Some("provider-key"),
    };
    for (id, template, path, header) in [
        ("general", UsageTemplate::General, "/user/balance", "Authorization"),
        ("newapi", UsageTemplate::NewApi, "/api/user/self", "New-Api-User"),
        ("sub2api", UsageTemplate::Sub2Api, "/v1/usage", "Authorization"),
    ] {
        let encoded = serde_json::to_string(&template).unwrap();
        assert_eq!(encoded, format!("\"{id}\""));
        assert_eq!(serde_json::from_str::<UsageTemplate>(&encoded).unwrap(), template);

        let PreparedUsage::Script(script) = template
            .prepare(
                None,
                ScriptParameters {
                    api_key: Some("  "),
                    access_token: Some("new-api-token"),
                    user_id: Some("123"),
                    ..Default::default()
                },
                provider,
            )
            .unwrap()
        else {
            panic!("expected a script for {id}");
        };
        assert_eq!(script.template(), template);
        assert_eq!(script.base_url(), Some("https://provider.example/v1"));
        assert!(script.source().contains(path));
        assert!(script.source().contains(header));
        assert!(script.source().contains(r#"method: "GET""#));
        assert!(!script.source().contains("{{"));
    }

    let PreparedUsage::Script(script) = UsageTemplate::Custom
        .prepare(
            Some(r#"["{{apiKey}}", "{{baseUrl}}", "{{accessToken}}", "{{userId}}"]"#),
            ScriptParameters {
                api_key: Some(" key\"'\\`\n\r\0${sideEffect()}{{userId}}\u{2028}😀 "),
                base_url: Some(" https://override.example/// "),
                access_token: Some("token"),
                user_id: Some("123"),
            },
            provider,
        )
        .unwrap()
    else {
        panic!("expected a custom script");
    };
    let values: [String; 4] = serde_json::from_str(script.source()).unwrap();
    assert_eq!(
        values,
        [
            "key\"'\\`\n\r\0${sideEffect()}{{userId}}\u{2028}😀",
            "https://override.example",
            "token",
            "123",
        ],
    );
}

#[test]
fn usage_scripts_config_rejects_invalid_input() {
    let result = UsageTemplate::NewApi.prepare(
        None,
        ScriptParameters::default(),
        ProviderDefaults {
            base_url: Some("https://private.example"),
            api_key: Some("private-key"),
        },
    );
    assert!(matches!(result, Err(TemplateError::MissingParameter(ScriptParameter::AccessToken))));
    assert!(matches!(
        UsageTemplate::Custom.prepare(
            Some(" \n"),
            ScriptParameters::default(),
            ProviderDefaults::default(),
        ),
        Err(TemplateError::EmptySource)
    ));
    assert!(serde_json::from_str::<UsageTemplate>(r#""unknown""#).is_err());
}

#[test]
fn usage_scripts_config_keeps_native_templates_out_of_javascript() {
    let detected_balance =
        detect_native_usage("https://api.deepseek.com", ZhipuPlanSelection::Auto).unwrap();
    let detected_plan =
        detect_native_usage("https://api.kimi.com/coding", ZhipuPlanSelection::Auto).unwrap();

    for (saved, account, detected, expected) in [
        (
            Some(UsageTemplate::Custom),
            Some(AccountUsage::GitHubCopilot),
            None,
            NativeTemplate::GitHubCopilot,
        ),
        (
            Some(UsageTemplate::Custom),
            Some(AccountUsage::OfficialSubscription),
            None,
            NativeTemplate::OfficialSubscription,
        ),
        (None, None, detected_balance, NativeTemplate::Balance),
        (None, None, detected_plan, NativeTemplate::CodingPlan),
        (Some(UsageTemplate::CodingPlan), None, None, NativeTemplate::CodingPlan),
    ] {
        let selected = UsageTemplate::select(saved, account, detected, false);
        assert!(matches!(
            selected.prepare(
                Some(r#"throw "{{apiKey}}";"#),
                ScriptParameters::default(),
                ProviderDefaults::default(),
            ),
            Ok(PreparedUsage::Native(native)) if native == expected
        ));
    }
    assert_eq!(
        UsageTemplate::select(Some(UsageTemplate::Sub2Api), None, detected_balance, false),
        UsageTemplate::Sub2Api
    );
    assert_eq!(UsageTemplate::select(None, None, detected_balance, true), UsageTemplate::NewApi);
    assert_eq!(UsageTemplate::select(None, None, None, false), UsageTemplate::General);
}
