//! This module provides the StepfunAiStepPlan provider, wrapping OpenAI Chat Completions for StepfunAiStepPlan requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    StepfunAiStepPlanProviderSettings,
    StepfunAiStepPlanProviderSettingsBuilder,
    "StepfunAiStepPlan",
    "https://api.stepfun.ai/step_plan/v1",
    "STEPFUN_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    StepfunAiStepPlan,
    StepfunAiStepPlanBuilder,
    StepfunAiStepPlanProviderSettings,
    "stepfun-ai-step-plan"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(StepfunAiStepPlan);
