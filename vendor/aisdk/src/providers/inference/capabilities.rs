//! Capabilities for inference models.
//!
//! This module defines model types and their capabilities for inference providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::inference::Inference;

model_capabilities! {
    provider: Inference,
    models: {
        GoogleGemma3 {
            model_name: "google/gemma-3",
            constructor_name: google_gemma_3,
            display_name: "Google Gemma 3",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MetaLlama321bInstruct {
            model_name: "meta/llama-3.2-1b-instruct",
            constructor_name: meta_llama_3_2_1b_instruct,
            display_name: "Llama 3.2 1B Instruct",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        MetaLlama323bInstruct {
            model_name: "meta/llama-3.2-3b-instruct",
            constructor_name: meta_llama_3_2_3b_instruct,
            display_name: "Llama 3.2 3B Instruct",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
