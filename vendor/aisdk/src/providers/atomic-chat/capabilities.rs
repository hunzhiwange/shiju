//! Capabilities for atomic_chat models.
//!
//! This module defines model types and their capabilities for atomic_chat providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::atomic_chat::AtomicChat;

model_capabilities! {
    provider: AtomicChat,
    models: {
        MetaLlama318bInstructGguf {
            model_name: "Meta-Llama-3_1-8B-Instruct-GGUF",
            constructor_name: meta_llama_3_1_8b_instruct_gguf,
            display_name: "Meta Llama 3.1 8B Instruct (GGUF)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Qwen359bMlx4bit {
            model_name: "Qwen3_5-9B-MLX-4bit",
            constructor_name: qwen3_5_9b_mlx_4bit,
            display_name: "Qwen 3.5 9B (MLX 4-bit)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Qwen359bQ4KM {
            model_name: "Qwen3_5-9B-Q4_K_M",
            constructor_name: qwen3_5_9b_q4_k_m,
            display_name: "Qwen 3.5 9B (Q4_K_M)",
            capabilities: [ImageInputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Gemma4E4bItIq4Xs {
            model_name: "gemma-4-E4B-it-IQ4_XS",
            constructor_name: gemma_4_e4b_it_iq4_xs,
            display_name: "Gemma 4 E4B Instruct (IQ4_XS)",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        Gemma4E4bItMlx4bit {
            model_name: "gemma-4-E4B-it-MLX-4bit",
            constructor_name: gemma_4_e4b_it_mlx_4bit,
            display_name: "Gemma 4 E4B Instruct (MLX 4-bit)",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
    }
}
