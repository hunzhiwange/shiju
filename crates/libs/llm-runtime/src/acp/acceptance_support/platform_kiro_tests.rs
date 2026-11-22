use std::{path::Path, process::Command};

#[test]
#[ignore = "requires a Linux shiju gateway and a reviewed Kiro installation"]
fn acp_platform_kiro_contract() {
    let gateway = std::env::var_os("SHIJU_ACP_KIRO_GATEWAY_BIN")
        .expect("set SHIJU_ACP_KIRO_GATEWAY_BIN to the Linux shiju gateway binary");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let output = tempfile::tempdir().unwrap();
    let report = output.path().join("kiro.json");
    let status = Command::new("python3")
        .current_dir(&root)
        .arg(root.join("scripts/verify_acp_skill_coexistence.py"))
        .args(["--agent", "kiro", "--gateway-bin"])
        .arg(gateway)
        .arg("--output")
        .arg(&report)
        .status()
        .unwrap();
    assert!(status.success(), "Kiro Linux gateway probe exited with {status}");
    let report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap();
    let artifact = &report["artifact"];
    assert_eq!(artifact["arch"], std::env::consts::ARCH);
    assert_eq!(artifact["version"], "3.0.0");
    assert_eq!(artifact["protocol"], "responses");
    assert_eq!(artifact["model_path"], "/v1/responses");
    assert_eq!(report["turns"].as_array().unwrap().len(), 4);
    println!("Kiro gateway artifact: {artifact}");
}
