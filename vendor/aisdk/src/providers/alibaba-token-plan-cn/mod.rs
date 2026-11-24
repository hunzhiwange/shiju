//! This module provides the AlibabaTokenPlanCn provider, wrapping OpenAI Chat Completions for AlibabaTokenPlanCn requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    AlibabaTokenPlanCnProviderSettings,
    AlibabaTokenPlanCnProviderSettingsBuilder,
    "AlibabaTokenPlanCn",
    "https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1",
    "ALIBABA_TOKEN_PLAN_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    AlibabaTokenPlanCn,
    AlibabaTokenPlanCnBuilder,
    AlibabaTokenPlanCnProviderSettings,
    "alibaba-token-plan-cn"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(AlibabaTokenPlanCn);
