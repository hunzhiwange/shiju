//! Capabilities for xiaomi_token_plan_sgp models.
//!
//! This module defines model types and their capabilities for xiaomi_token_plan_sgp providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::xiaomi_token_plan_sgp::XiaomiTokenPlanSgp;

model_capabilities! {
    provider: XiaomiTokenPlanSgp,
    models: {
        MimoV25Tts {
            model_name: "mimo-v2.5-tts",
            constructor_name: mimo_v2_5_tts,
            display_name: "MiMo-V2.5-TTS",
            capabilities: [AudioOutputSupport, TextInputSupport]
        },
        MimoV25TtsVoiceclone {
            model_name: "mimo-v2.5-tts-voiceclone",
            constructor_name: mimo_v2_5_tts_voiceclone,
            display_name: "MiMo-V2.5-TTS-VoiceClone",
            capabilities: [AudioOutputSupport, TextInputSupport]
        },
        MimoV25TtsVoicedesign {
            model_name: "mimo-v2.5-tts-voicedesign",
            constructor_name: mimo_v2_5_tts_voicedesign,
            display_name: "MiMo-V2.5-TTS-VoiceDesign",
            capabilities: [AudioOutputSupport, TextInputSupport]
        },
        MimoV26Flash {
            model_name: "mimo-v2.6-flash",
            constructor_name: mimo_v2_6_flash,
            display_name: "MiMo-V2.6-Flash",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        MimoV26Pro {
            model_name: "mimo-v2.6-pro",
            constructor_name: mimo_v2_6_pro,
            display_name: "MiMo-V2.6-Pro",
            capabilities: [AudioInputSupport, ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
    }
}
