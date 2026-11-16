//! Native ACP rendering data.
use vw_api_acp_model::acp_session::*;
fn fixture_status() -> AcpAccessStatus {
    AcpAccessStatus {
        requested: false,
        backend: "fixture".into(),
        version: Some("1.0".into()),
        support: AcpAccessSupport::Unavailable,
        explanation: "沿用工作区工具审批规则。".into(),
    }
}

pub fn options() -> AcpSessionOptions {
    AcpSessionOptions {
                acp_model_source:
                    vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel,
                model_source_capabilities: Vec::new(),
                selected_model: None,
                verified_cli_release: None,
                verified_platform: None,
                access_status: Some(fixture_status()),
                connected: true,
                options: vec![AcpSessionOption {
                    kind: AcpOptionKind::Config { id: "reasoning_effort".into() },
                    name: "推理档位".into(),
                    current: "low".into(),
                    choices: vec![
                        AcpChoice { id: "low".into(), name: "低".into() },
                        AcpChoice {
                            id: "high".into(),
                            name: "高档位，使用较多推理预算来处理复杂问题和多个约束条件。此长选项用于检查中文折行和滚动区域的留白。".into(),
                        },
                    ],
                }],
            }
}
