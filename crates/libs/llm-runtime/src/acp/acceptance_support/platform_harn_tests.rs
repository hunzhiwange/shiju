use std::{path::Path, process::Command};

#[test]
#[ignore = "requires a Linux shiju gateway binary and an isolated Harn v5 source build"]
fn acp_platform_harn_contract() {
    let gateway = std::env::var_os("SHIJU_ACP_HARN_GATEWAY_BIN")
        .expect("set SHIJU_ACP_HARN_GATEWAY_BIN to the Linux shiju gateway binary");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let status = Command::new("python3")
        .arg(root.join("scripts/acp_skill_coexistence/harn_linux.py"))
        .arg(gateway)
        .status()
        .unwrap();
    assert!(status.success(), "Harn Linux gateway probe exited with {status}");
}
