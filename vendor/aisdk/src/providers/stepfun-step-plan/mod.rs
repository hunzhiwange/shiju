//! This module provides the StepfunStepPlan provider, wrapping OpenAI Chat Completions for StepfunStepPlan requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    StepfunStepPlanProviderSettings,
    StepfunStepPlanProviderSettingsBuilder,
    "StepfunStepPlan",
    "https://api.stepfun.com/step_plan/v1",
    "STEPFUN_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    StepfunStepPlan,
    StepfunStepPlanBuilder,
    StepfunStepPlanProviderSettings,
    "stepfun-step-plan"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(StepfunStepPlan);
