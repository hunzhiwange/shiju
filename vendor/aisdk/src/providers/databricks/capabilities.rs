//! Capabilities for databricks models.
//!
//! This module defines model types and their capabilities for databricks providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::databricks::Databricks;

model_capabilities! {
    provider: Databricks,
    models: {
        DatabricksClaudeHaiku45 {
            model_name: "databricks-claude-haiku-4-5",
            constructor_name: databricks_claude_haiku_4_5,
            display_name: "Claude Haiku 4.5 (latest)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DatabricksClaudeOpus41 {
            model_name: "databricks-claude-opus-4-1",
            constructor_name: databricks_claude_opus_4_1,
            display_name: "Claude Opus 4.1 (latest)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DatabricksClaudeOpus45 {
            model_name: "databricks-claude-opus-4-5",
            constructor_name: databricks_claude_opus_4_5,
            display_name: "Claude Opus 4.5 (latest)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DatabricksClaudeOpus46 {
            model_name: "databricks-claude-opus-4-6",
            constructor_name: databricks_claude_opus_4_6,
            display_name: "Claude Opus 4.6",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DatabricksClaudeOpus47 {
            model_name: "databricks-claude-opus-4-7",
            constructor_name: databricks_claude_opus_4_7,
            display_name: "Claude Opus 4.7",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DatabricksClaudeSonnet45 {
            model_name: "databricks-claude-sonnet-4-5",
            constructor_name: databricks_claude_sonnet_4_5,
            display_name: "Claude Sonnet 4.5 (latest)",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DatabricksClaudeSonnet46 {
            model_name: "databricks-claude-sonnet-4-6",
            constructor_name: databricks_claude_sonnet_4_6,
            display_name: "Claude Sonnet 4.6",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DatabricksGemini31FlashLite {
            model_name: "databricks-gemini-3-1-flash-lite",
            constructor_name: databricks_gemini_3_1_flash_lite,
            display_name: "Gemini 3.1 Flash Lite Preview",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        DatabricksGemini31Pro {
            model_name: "databricks-gemini-3-1-pro",
            constructor_name: databricks_gemini_3_1_pro,
            display_name: "Gemini 3.1 Pro Preview",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        DatabricksGemini35Flash {
            model_name: "databricks-gemini-3-5-flash",
            constructor_name: databricks_gemini_3_5_flash,
            display_name: "Gemini 3.5 Flash",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        DatabricksGemini3Flash {
            model_name: "databricks-gemini-3-flash",
            constructor_name: databricks_gemini_3_flash,
            display_name: "Gemini 3 Flash Preview",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        DatabricksGlm52 {
            model_name: "databricks-glm-5-2",
            constructor_name: databricks_glm_5_2,
            display_name: "GLM-5.2",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DatabricksGpt5 {
            model_name: "databricks-gpt-5",
            constructor_name: databricks_gpt_5,
            display_name: "GPT-5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DatabricksGpt51 {
            model_name: "databricks-gpt-5-1",
            constructor_name: databricks_gpt_5_1,
            display_name: "GPT-5.1",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DatabricksGpt52 {
            model_name: "databricks-gpt-5-2",
            constructor_name: databricks_gpt_5_2,
            display_name: "GPT-5.2",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DatabricksGpt54 {
            model_name: "databricks-gpt-5-4",
            constructor_name: databricks_gpt_5_4,
            display_name: "GPT-5.4",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DatabricksGpt54Mini {
            model_name: "databricks-gpt-5-4-mini",
            constructor_name: databricks_gpt_5_4_mini,
            display_name: "GPT-5.4 mini",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DatabricksGpt54Nano {
            model_name: "databricks-gpt-5-4-nano",
            constructor_name: databricks_gpt_5_4_nano,
            display_name: "GPT-5.4 nano",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DatabricksGpt55 {
            model_name: "databricks-gpt-5-5",
            constructor_name: databricks_gpt_5_5,
            display_name: "GPT-5.5",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DatabricksGpt56Luna {
            model_name: "databricks-gpt-5-6-luna",
            constructor_name: databricks_gpt_5_6_luna,
            display_name: "GPT-5.6 Luna",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DatabricksGpt56Sol {
            model_name: "databricks-gpt-5-6-sol",
            constructor_name: databricks_gpt_5_6_sol,
            display_name: "GPT-5.6 Sol",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DatabricksGpt56Terra {
            model_name: "databricks-gpt-5-6-terra",
            constructor_name: databricks_gpt_5_6_terra,
            display_name: "GPT-5.6 Terra",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DatabricksGpt5Mini {
            model_name: "databricks-gpt-5-mini",
            constructor_name: databricks_gpt_5_mini,
            display_name: "GPT-5 Mini",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DatabricksGpt5Nano {
            model_name: "databricks-gpt-5-nano",
            constructor_name: databricks_gpt_5_nano,
            display_name: "GPT-5 Nano",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DatabricksGptOss120b {
            model_name: "databricks-gpt-oss-120b",
            constructor_name: databricks_gpt_oss_120b,
            display_name: "GPT OSS 120B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        DatabricksGptOss20b {
            model_name: "databricks-gpt-oss-20b",
            constructor_name: databricks_gpt_oss_20b,
            display_name: "GPT OSS 20B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
