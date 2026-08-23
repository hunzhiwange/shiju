//! 验证网关账户数据向桌面模型转换时的字段规范化与凭据脱敏约束。
//! fixture 刻意混入首尾空白和可识别密钥，用于同时覆盖清洗结果与调试输出边界。

use super::*;

#[test]
fn account_gateway_dtos_map_without_exposing_credentials() {
    let challenge_dto: vw_gateway_client::ShijuLoginChallengeDto =
        serde_json::from_value(serde_json::json!({
            "state": "state-secret-value",
            "authorization_url":
                "https://auth.shiju.cloud/plugin/login?state=state-secret-value",
            "poll_interval_seconds": 2
        }))
        .expect("challenge dto");
    let challenge = LoginChallenge::try_from(challenge_dto).expect("desktop challenge");

    assert_eq!(challenge.state(), "state-secret-value");
    assert_eq!(challenge.poll_interval(), std::time::Duration::from_secs(2));
    assert_eq!(
        challenge.authorization_url().as_str(),
        "https://auth.shiju.cloud/plugin/login?state=state-secret-value"
    );
    let challenge_debug = format!("{challenge:?}");
    assert!(!challenge_debug.contains("state-secret-value"));
    assert!(!challenge_debug.contains("auth.shiju.cloud"));

    let credential =
        vw_gateway_client::ShijuCredential::new("gateway-secret-value").expect("credential");
    let poll = LoginPoll::try_from(vw_gateway_client::ShijuLoginPollDto::Approved {
        credential: credential.clone(),
    })
    .expect("approved poll");
    let LoginPoll::Approved(skey) = poll else {
        panic!("expected approved poll");
    };
    assert_eq!(skey.expose(), "gateway-secret-value");
    assert!(!format!("{skey:?}").contains("gateway-secret-value"));

    let profile = vw_gateway_client::ShijuAccountProfileDto {
        company_name: " 示例公司 ".to_string(),
        company_status: " enabled ".to_string(),
        name: " 测试用户 ".to_string(),
        account_name: " tester ".to_string(),
        user_status: " active ".to_string(),
        departments: vec![" AI创新组 ".to_string(), " ".to_string()],
        roles: vec![" 默认角色 ".to_string()],
    };
    let verified = VerifiedAccount::from_gateway_profile(
        profile,
        SecretSkey::new(credential.expose()).expect("desktop credential"),
    )
    .expect("verified account");

    assert_eq!(verified.identity().company_name(), "示例公司");
    assert_eq!(verified.identity().status(), "enabled");
    let profile = verified.profile().expect("profile");
    assert_eq!(profile.name(), "测试用户");
    assert_eq!(profile.account_name(), "tester");
    assert_eq!(profile.departments(), ["AI创新组"]);
    assert_eq!(profile.roles(), ["默认角色"]);
    assert!(!format!("{verified:?}").contains("gateway-secret-value"));
}
