use super::*;

#[test]
fn lark_platform_maps_endpoints_and_labels() {
    assert_eq!(LarkPlatform::Lark.api_base(), LARK_BASE_URL);
    assert_eq!(LarkPlatform::Lark.ws_base(), LARK_WS_BASE_URL);
    assert_eq!(LarkPlatform::Feishu.api_base(), FEISHU_BASE_URL);
    assert_eq!(LarkPlatform::Feishu.ws_base(), FEISHU_WS_BASE_URL);
    assert_eq!(LarkPlatform::Lark.locale_header(), "en");
    assert_eq!(LarkPlatform::Feishu.locale_header(), "zh");
    assert_eq!(LarkPlatform::Lark.proxy_service_key(), "channel.lark");
    assert_eq!(LarkPlatform::Feishu.proxy_service_key(), "channel.feishu");
    assert_eq!(LarkPlatform::Lark.channel_name(), "lark");
    assert_eq!(LarkPlatform::Feishu.channel_name(), "feishu");
}

#[test]
fn lark_platform_supports_expected_value_traits() {
    let lark = LarkPlatform::Lark;
    let feishu = LarkPlatform::Feishu;
    let copied = lark;

    assert_eq!(lark, copied);
    assert_ne!(lark, feishu);
    assert_eq!(format!("{lark:?}"), "Lark");
    assert_eq!(format!("{feishu:?}"), "Feishu");
}
