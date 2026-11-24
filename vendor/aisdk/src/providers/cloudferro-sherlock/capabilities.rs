//! Capabilities for cloudferro_sherlock models.
//!
//! This module defines model types and their capabilities for cloudferro_sherlock providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::cloudferro_sherlock::CloudferroSherlock;

model_capabilities! {
    provider: CloudferroSherlock,
    models: {
        MinimaxaiMinimaxM25 {
            model_name: "MiniMaxAI/MiniMax-M2.5",
            constructor_name: minimaxai_minimax_m2_5,
            display_name: "MiniMax-M2.5",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MetaLlamaLlama3370bInstruct {
            model_name: "meta-llama/Llama-3.3-70B-Instruct",
            constructor_name: meta_llama_llama_3_3_70b_instruct,
            display_name: "Llama 3.3 70B Instruct",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        OpenaiGptOss120b {
            model_name: "openai/gpt-oss-120b",
            constructor_name: openai_gpt_oss_120b,
            display_name: "OpenAI GPT OSS 120B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        SpeakleashBielik11bV30Instruct {
            model_name: "speakleash/Bielik-11B-v3.0-Instruct",
            constructor_name: speakleash_bielik_11b_v3_0_instruct,
            display_name: "Bielik 11B v3.0 Instruct",
            capabilities: [StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        ZaiOrgGlm53 {
            model_name: "zai-org/GLM-5.3",
            constructor_name: zai_org_glm_5_3,
            display_name: "GLM-5.3",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
