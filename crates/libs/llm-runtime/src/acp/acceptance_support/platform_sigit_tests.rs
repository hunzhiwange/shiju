use serde_json::{Value, json};
use std::{path::Path, process::Command};

#[test]
#[ignore = "requires a Linux gateway and a private, reviewed siGit build"]
fn acp_platform_sigit_contract() {
    let gateway = std::env::var_os("SHIJU_ACP_SIGIT_GATEWAY_BIN")
        .expect("set SHIJU_ACP_SIGIT_GATEWAY_BIN to the current Linux shiju binary");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("sigit-platform.json");
    let status = Command::new("python3")
        .arg(root.join("scripts/verify_acp_skill_coexistence.py"))
        .args(["--agent", "sigit", "--gateway-bin"])
        .arg(gateway)
        .arg("--output")
        .arg(&output)
        .status()
        .unwrap();
    let evidence: Value = serde_json::from_slice(&std::fs::read(output).unwrap()).unwrap();
    eprintln!("siGit gateway probe: {evidence}");
    assert!(status.success(), "siGit platform probe failed");
    assert_eq!(evidence["passed"], true);
    assert_eq!(evidence["platform"], format!("Linux-{}", std::env::consts::ARCH));

    let contract: Value = serde_json::from_str(include_str!(
        "../../../../../../patches/acp-model-source/sigit/linux.json"
    ))
    .unwrap();
    let artifact = &evidence["artifact"];
    for field in [
        "version",
        "revision",
        "entry",
        "source_sha256",
        "linux_patch_sha256",
        "linux_mcp_patch_sha256",
    ] {
        assert_eq!(artifact[field], contract[field], "{field}");
    }
    assert_eq!(artifact["elf_machine"], contract["targets"][std::env::consts::ARCH]["elf_machine"]);
    assert_eq!(artifact["model_protocol"], "POST /v1/chat/completions");

    let turns = evidence["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 4);
    for (turn, phase) in turns.iter().zip(["baseline", "mounted", "removed", "rejected"]) {
        assert_eq!(turn["phase"], phase);
        assert_eq!(turn["source"], "session_model");
        assert_eq!(turn["model"], "coexistence-fixture/deepseek-v4.1-flash-1017");
        assert_eq!(turn["done"], 1);
        assert_eq!(turn["errors"], 0);
        let mounted = matches!(phase, "mounted" | "rejected");
        let calls = turn["calls"].as_object().unwrap();
        for name in ["native", "native_read"] {
            assert!(calls.contains_key(name), "{phase}: missing {name}");
        }
        for name in ["selected", "selected_read", "mcp"] {
            assert_eq!(calls.contains_key(name), mounted, "{phase}: {name}");
        }
        assert_eq!(turn["model_requests"], calls.len() as u64 + 1);
        assert_eq!(turn["mcp_calls"], u64::from(phase == "mounted"));
        assert_eq!(
            turn["mcp_decisions"],
            match phase {
                "mounted" => json!(["once"]),
                "rejected" => json!(["reject"]),
                _ => json!([]),
            }
        );
    }
}
