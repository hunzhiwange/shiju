use super::detect::{
    BalanceProvider, CodingPlanProvider, NativeUsageCapability, UsageDetectionError,
    ZhipuPlanSelection, detect_native_usage,
};

#[test]
fn usage_detect_returns_expected_result() {
    use NativeUsageCapability::{Balance, CodingPlan};

    for (url, capability) in [
        ("https://API.DEEPSEEK.COM:443/v1", Balance(BalanceProvider::DeepSeek)),
        ("https://api.stepfun.ai/v1", Balance(BalanceProvider::StepFun)),
        ("https://api.stepfun.com/v1", Balance(BalanceProvider::StepFun)),
        ("https://api.siliconflow.cn/v1", Balance(BalanceProvider::SiliconFlowCn)),
        ("https://api.siliconflow.com/v1", Balance(BalanceProvider::SiliconFlowGlobal)),
        ("https://openrouter.ai/api/v1", Balance(BalanceProvider::OpenRouter)),
        ("https://api.novita.ai/v3/openai", Balance(BalanceProvider::Novita)),
        ("https://api.kimi.com/coding", CodingPlan(CodingPlanProvider::Kimi)),
        ("https://api.kimi.ai/coding/v1", CodingPlan(CodingPlanProvider::Kimi)),
        ("https://open.bigmodel.cn/api/anthropic", CodingPlan(CodingPlanProvider::ZhipuCn)),
        ("https://api.z.ai/api/paas/v4", CodingPlan(CodingPlanProvider::ZhipuGlobal)),
        ("https://api.minimaxi.com/v1", CodingPlan(CodingPlanProvider::MiniMaxCn)),
        ("https://api.minimax.cn/anthropic", CodingPlan(CodingPlanProvider::MiniMaxCn)),
        ("https://api.minimax.io/v1", CodingPlan(CodingPlanProvider::MiniMaxGlobal)),
        ("https://edge.api.minimax.io/v1", CodingPlan(CodingPlanProvider::MiniMaxGlobal)),
        ("https://zenmux.ai/api/v1", CodingPlan(CodingPlanProvider::ZenMux)),
        (
            "https://ark.cn-beijing.volces.com/api/plan/v3",
            CodingPlan(CodingPlanProvider::Volcengine),
        ),
        (
            "https://ark.cn-shanghai.volces.com/api/coding",
            CodingPlan(CodingPlanProvider::Volcengine),
        ),
        ("https://opencode.ai/zen/go", CodingPlan(CodingPlanProvider::OpenCodeGo)),
        ("https://opencode.ai/zen/go/v1", CodingPlan(CodingPlanProvider::OpenCodeGo)),
        ("https://opencode.ai/inference/go/anthropic", CodingPlan(CodingPlanProvider::OpenCodeGo)),
        ("https://opencode.ai/inference/go/v1", CodingPlan(CodingPlanProvider::OpenCodeGo)),
        ("https://api.commandcode.ai/provider", CodingPlan(CodingPlanProvider::CommandCode)),
        ("https://api.commandcode.ai/provider/v1", CodingPlan(CodingPlanProvider::CommandCode)),
    ] {
        assert_eq!(
            detect_native_usage(url, ZhipuPlanSelection::Auto),
            Ok(Some(capability)),
            "{url}"
        );
    }
}

#[test]
fn usage_detect_rejects_invalid_input() {
    for url in [
        "https://api.deepseek.com.evil.example/v1",
        "https://notopenrouter.ai/api/v1",
        "https://api.minimax.cn.evil.example/v1",
        "https://fakeapi.minimax.io/v1",
        "https://evil.example/api.minimax.io/v1",
        "https://api.z.ai.evil.example",
        "https://evil.example/?url=https://api.stepfun.ai/v1",
        "https://evil.example/#https://api.kimi.com/coding",
        "https://zenmux.example/v1",
        "https://api.kimi.com/coding-backup",
        "https://api.kimi.com/v1?path=/coding",
        "https://api.kimi.com/other/coding",
        "https://api.kimi.com/coding%2Fv1",
        "https://ark.cn-beijing.volces.com/api/v3",
        "https://ark.cn-beijing.volces.com/api/compatible",
        "https://ark.cn-beijing.volces.com/api/planner",
        "https://ark.cn-beijing.volces.com.evil.example/api/plan",
        "https://other.cn-beijing.volces.com/api/plan",
        "https://opencode.ai/zen/v1",
        "https://opencode.ai/inference/v1",
        "https://opencode.ai/zen/google",
        "https://api.commandcode.ai/providers",
        "https://commandcode.ai/provider",
    ] {
        assert_eq!(detect_native_usage(url, ZhipuPlanSelection::Auto), Ok(None), "{url}");
    }

    for url in [
        "api.deepseek.com/v1",
        "ftp://api.deepseek.com/v1",
        "https://api.deepseek.com@evil.example/v1",
        "https://user:secret@api.deepseek.com/v1",
    ] {
        assert_eq!(
            detect_native_usage(url, ZhipuPlanSelection::Auto),
            Err(UsageDetectionError::InvalidBaseUrl)
        );
    }
}

#[test]
fn usage_detect_requires_explicit_team_selection_and_disables_volcengine_by_default() {
    let zhipu = "https://open.bigmodel.cn/api/coding";
    assert_eq!(
        detect_native_usage(zhipu, ZhipuPlanSelection::Auto),
        Ok(Some(NativeUsageCapability::CodingPlan(CodingPlanProvider::ZhipuCn)))
    );
    assert_eq!(
        detect_native_usage(zhipu, ZhipuPlanSelection::Team),
        Ok(Some(NativeUsageCapability::CodingPlan(CodingPlanProvider::ZhipuTeam)))
    );
    for url in [
        "https://api.z.ai/api/paas/v4",
        "https://open.bigmodel.cn.evil.example/api/coding",
        "https://api.deepseek.com/v1",
    ] {
        assert_eq!(
            detect_native_usage(url, ZhipuPlanSelection::Team),
            Err(UsageDetectionError::IncompatibleZhipuTeam)
        );
    }

    assert!(CodingPlanProvider::Kimi.enabled_by_default());
    assert!(!CodingPlanProvider::Volcengine.enabled_by_default());
}
