//! This module provides the AlibabaCodingPlanCn provider, wrapping OpenAI Chat Completions for AlibabaCodingPlanCn requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    AlibabaCodingPlanCnProviderSettings,
    AlibabaCodingPlanCnProviderSettingsBuilder,
    "AlibabaCodingPlanCn",
    "https://coding.dashscope.aliyuncs.com/v1",
    "ALIBABA_CODING_PLAN_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    AlibabaCodingPlanCn,
    AlibabaCodingPlanCnBuilder,
    AlibabaCodingPlanCnProviderSettings,
    "alibaba-coding-plan-cn"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(AlibabaCodingPlanCn);
