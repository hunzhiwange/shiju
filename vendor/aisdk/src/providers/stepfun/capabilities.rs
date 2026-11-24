//! Capabilities for stepfun models.
//!
//! This module defines model types and their capabilities for stepfun providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::stepfun::Stepfun;

model_capabilities! {
    provider: Stepfun,
    models: {
        Step35Flash {
            model_name: "step-3.5-flash",
            constructor_name: step_3_5_flash,
            display_name: "Step 3.5 Flash",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Step35Flash2603 {
            model_name: "step-3.5-flash-2603",
            constructor_name: step_3_5_flash_2603,
            display_name: "Step 3.5 Flash 2603",
            capabilities: [ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Step37Flash {
            model_name: "step-3.7-flash",
            constructor_name: step_3_7_flash,
            display_name: "Step 3.7 Flash",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        Step5Preview {
            model_name: "step-5-preview",
            constructor_name: step_5_preview,
            display_name: "Step 5 Preview",
            capabilities: [ImageInputSupport, ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport, VideoInputSupport]
        },
        StepTts2 {
            model_name: "step-tts-2",
            constructor_name: step_tts_2,
            display_name: "Step TTS 2",
            capabilities: [AudioOutputSupport, TextInputSupport]
        },
        Stepaudio25Asr {
            model_name: "stepaudio-2.5-asr",
            constructor_name: stepaudio_2_5_asr,
            display_name: "StepAudio 2.5 ASR",
            capabilities: [AudioInputSupport, ImageInputSupport, TextOutputSupport]
        },
        Stepaudio25Tts {
            model_name: "stepaudio-2.5-tts",
            constructor_name: stepaudio_2_5_tts,
            display_name: "StepAudio 2.5 TTS",
            capabilities: [AudioOutputSupport, TextInputSupport]
        },
    }
}
