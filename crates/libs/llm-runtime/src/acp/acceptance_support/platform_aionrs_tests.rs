use serde_json::{Value, json};
use std::{path::Path, process::Command};

#[test]
#[ignore = "requires a Linux gateway and a fixed aionrs candidate in an isolated container"]
fn acp_platform_aionrs_contract() {
    let gateway = std::env::var_os("SHIJU_ACP_AIONRS_GATEWAY_BIN")
        .expect("provide the absolute path to the Linux shiju gateway");
    assert!(Path::new(&gateway).is_absolute());
    let root = tempfile::tempdir().unwrap();
    let report = root.path().join("report.json");
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../scripts/verify_acp_skill_coexistence.py");
    let status = Command::new("python3")
        .arg(script)
        .args(["--agent", "aionrs", "--gateway-bin"])
        .arg(gateway)
        .arg("--output")
        .arg(&report)
        .status()
        .expect("could not start the aionrs gateway probe");
    let evidence: Value = serde_json::from_slice(
        &std::fs::read(report).expect("the gateway probe did not produce a report"),
    )
    .unwrap();
    assert!(status.success() && evidence["passed"] == true, "{evidence:#}");
    assert_eq!(evidence["platform"], format!("Linux-{}", std::env::consts::ARCH));
    assert_eq!(evidence["artifact"]["version"], "0.2.11");

    let turns = evidence["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 4);
    for (turn, (phase, calls, decisions)) in turns.iter().zip([
        ("baseline", 0, json!([])),
        ("mounted", 1, json!(["once"])),
        ("removed", 0, json!([])),
        ("rejected", 0, json!(["reject"])),
    ]) {
        assert_eq!(turn["phase"], phase);
        assert_eq!(turn["source"], "session_model");
        assert_eq!(turn["mcp_calls"], calls);
        assert_eq!(turn["mcp_decisions"], decisions);
        assert!(turn["calls"]["native_read"].is_string());
        let mounted = matches!(phase, "mounted" | "rejected");
        assert_eq!(turn["calls"]["selected_read"].is_string(), mounted);
        assert_eq!(turn["calls"]["mcp"].is_string(), mounted);
    }
    eprintln!(
        "aionrs artifact: {}; observed model protocol: POST /v1/chat/completions",
        evidence["artifact"]
    );
}
