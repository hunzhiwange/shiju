use vw_api_types::apps_usage::UsageScriptPackage;

use super::config::{
    PreparedUsage, PreparedUsageScript, ProviderDefaults, ScriptParameters, UsageTemplate,
};
use super::extract::ExtractError;
use super::runtime::RuntimeError;

const FIXTURE_TOKEN: &str = "fixture-private-access-token";
const FIXTURE_KEY: &str = "fixture-private-api-key";

fn prepare(source: &str) -> PreparedUsageScript<'static> {
    let PreparedUsage::Script(script) = UsageTemplate::Custom
        .prepare(
            Some(source),
            ScriptParameters { access_token: Some(FIXTURE_TOKEN), ..ScriptParameters::default() },
            ProviderDefaults { api_key: Some(FIXTURE_KEY), ..ProviderDefaults::default() },
        )
        .unwrap()
    else {
        panic!("expected a prepared script");
    };
    script
}

#[tokio::test]
async fn usage_scripts_extract_returns_expected_result() {
    let script = prepare("({ extractor: (response) => response.packages })");
    let active = r#"{"planName":"Monthly","extra":"Renews soon","isValid":true,"total":10,"used":10.5,"remaining":-0.5,"unit":"USD"}"#;
    let invalid = r#"{"planName":"Expired","isValid":false,"invalidMessage":"Subscription ended","remaining":null}"#;
    let zero = r#"{"remaining":0}"#;
    let expected = [
        UsageScriptPackage {
            plan_name: Some("Monthly".into()),
            extra: Some("Renews soon".into()),
            is_valid: Some(true),
            invalid_message: None,
            total: Some(10.0.try_into().unwrap()),
            used: Some(10.5.try_into().unwrap()),
            remaining: Some((-0.5).try_into().unwrap()),
            unit: Some("USD".into()),
        },
        UsageScriptPackage {
            plan_name: Some("Expired".into()),
            extra: None,
            is_valid: Some(false),
            invalid_message: Some("Subscription ended".into()),
            total: None,
            used: None,
            remaining: None,
            unit: None,
        },
        UsageScriptPackage {
            plan_name: None,
            extra: None,
            is_valid: None,
            invalid_message: None,
            total: None,
            used: None,
            remaining: Some(0.0.try_into().unwrap()),
            unit: None,
        },
    ];

    assert_eq!(
        script.extract_packages(&format!(r#"{{"packages":{active}}}"#)).await.unwrap(),
        expected[..1]
    );
    assert_eq!(
        script
            .extract_packages(&format!(r#"{{"packages":[{active},{invalid},{zero}]}}"#))
            .await
            .unwrap(),
        expected
    );
}

#[tokio::test]
async fn usage_scripts_extract_rejects_invalid_input() {
    let script = prepare("({ extractor: (response) => response })");
    for (input, expected) in [
        ("[]", ExtractError::EmptyResult),
        ("{}", ExtractError::MissingValues),
        (r#"{"remaining":null,"unit":"USD"}"#, ExtractError::MissingValues),
        ("null", ExtractError::InvalidPackage),
        (r#"[{"remaining":0},false]"#, ExtractError::InvalidPackage),
        (r#"{"remaining":0,"isValid":"true"}"#, ExtractError::InvalidPackage),
        (r#"{"remaining":0,"total":"5"}"#, ExtractError::InvalidPackage),
        (r#"{"remaining":0,"unit":5}"#, ExtractError::InvalidPackage),
        (r#"{"remaining":0,"extra":["private payload"]}"#, ExtractError::InvalidPackage),
    ] {
        assert_eq!(script.extract_packages(input).await, Err(expected));
    }
    assert_eq!(
        script.extract_packages(FIXTURE_TOKEN).await,
        Err(ExtractError::Runtime(RuntimeError::InvalidResponse))
    );
    for number in ["NaN", "Infinity", "-Infinity"] {
        let script =
            prepare(&format!("({{ extractor: () => ({{ remaining: 0, total: {number} }}) }})"));
        assert_eq!(
            script.extract_packages("{}").await,
            Err(ExtractError::Runtime(RuntimeError::InvalidResult))
        );
    }
}

#[tokio::test]
async fn usage_scripts_extract_keeps_credentials_out_of_results_and_errors() {
    let script =
        prepare(r#"({ extractor: () => ({ remaining: 1, extra: "Bearer {{accessToken}}" }) })"#);
    let error = script.extract_packages("{}").await.unwrap_err();
    assert_eq!(error, ExtractError::SensitiveResult);
    assert!(!format!("{error:?} {error}").contains(FIXTURE_TOKEN));

    let script =
        prepare(r#"({ extractor: (response) => ({ remaining: 1, unit: response.unit }) })"#);
    let error =
        script.extract_packages(&format!(r#"{{"unit":"{FIXTURE_KEY}"}}"#)).await.unwrap_err();
    assert_eq!(error, ExtractError::SensitiveResult);
    assert!(!format!("{error:?} {error}").contains(FIXTURE_KEY));

    let script = prepare(r#"({ extractor: () => { throw Error("{{accessToken}}"); } })"#);
    let error = script.extract_packages("{}").await.unwrap_err();
    assert_eq!(error, ExtractError::Runtime(RuntimeError::InvalidResult));
    assert!(!format!("{error:?} {error}").contains(FIXTURE_TOKEN));
}
