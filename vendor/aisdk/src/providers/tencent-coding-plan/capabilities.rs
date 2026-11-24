//! Capabilities for tencent_coding_plan models.
//!
//! This module defines model types and their capabilities for tencent_coding_plan providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::tencent_coding_plan::TencentCodingPlan;

model_capabilities! {
    provider: TencentCodingPlan,
    models: {
        TcCodeLatest {
            model_name: "tc-code-latest",
            constructor_name: tc_code_latest,
            display_name: "Auto",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
