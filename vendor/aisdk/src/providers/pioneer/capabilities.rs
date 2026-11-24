//! Capabilities for pioneer models.
//!
//! This module defines model types and their capabilities for pioneer providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::pioneer::Pioneer;

model_capabilities! {
    provider: Pioneer,
    models: {
        FastinoGliner25Decide {
            model_name: "fastino/GLiNER-2.5-Decide",
            constructor_name: fastino_gliner_2_5_decide,
            display_name: "GLiNER-2.5-Decide",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        FastinoGliguardLlmguardrails300m {
            model_name: "fastino/gliguard-LLMGuardrails-300M",
            constructor_name: fastino_gliguard_llmguardrails_300m,
            display_name: "GLiGuard LLM Guardrails 300M",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        FastinoGliguardPiiMulti {
            model_name: "fastino/gliguard-PII-multi",
            constructor_name: fastino_gliguard_pii_multi,
            display_name: "GLiNER2-Guardrails-PII-Multi",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        FastinoGliner2BaseV1 {
            model_name: "fastino/gliner2-base-v1",
            constructor_name: fastino_gliner2_base_v1,
            display_name: "GLiNER2 Base",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        FastinoGliner2LargeV1 {
            model_name: "fastino/gliner2-large-v1",
            constructor_name: fastino_gliner2_large_v1,
            display_name: "GLiNER2 Large",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        FastinoGliner2MultiLargeV1 {
            model_name: "fastino/gliner2-multi-large-v1",
            constructor_name: fastino_gliner2_multi_large_v1,
            display_name: "GLiNER2 Multi Large",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        FastinoGliner2MultiV1 {
            model_name: "fastino/gliner2-multi-v1",
            constructor_name: fastino_gliner2_multi_v1,
            display_name: "GLiNER2 Multi",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        FastinoGliner2PrivacyFilterPiiMulti {
            model_name: "fastino/gliner2-privacy-filter-PII-multi",
            constructor_name: fastino_gliner2_privacy_filter_pii_multi,
            display_name: "GLiNER2 Privacy Filter PII (Multi)",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        FastinoGliner25BaseV1 {
            model_name: "fastino/gliner2.5-base-v1",
            constructor_name: fastino_gliner2_5_base_v1,
            display_name: "GLiNER 2.5 Base",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        FastinoGliner25MultiV1 {
            model_name: "fastino/gliner2.5-multi-v1",
            constructor_name: fastino_gliner2_5_multi_v1,
            display_name: "GLiNER 2.5 Multi",
            capabilities: [TextInputSupport, TextOutputSupport]
        },
        NvidiaNvidiaNemotron35Lightning30bA3bBf16 {
            model_name: "nvidia/NVIDIA-Nemotron-3.5-Lightning-30B-A3B-BF16",
            constructor_name: nvidia_nvidia_nemotron_3_5_lightning_30b_a3b_bf16,
            display_name: "Nemotron 3.5 Lightning 30B A3B",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        NvidiaNvidiaNemotron35SuperVl120bA12bNvfp4 {
            model_name: "nvidia/NVIDIA-Nemotron-3.5-Super-VL-120B-A12B-NVFP4",
            constructor_name: nvidia_nvidia_nemotron_3_5_super_vl_120b_a12b_nvfp4,
            display_name: "Nemotron 3.5 Super VL 120B A12B",
            capabilities: [ImageInputSupport, ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
    }
}
