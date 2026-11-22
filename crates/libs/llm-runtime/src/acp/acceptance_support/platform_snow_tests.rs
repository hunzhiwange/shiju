use std::{path::Path, process::Command};

#[test]
#[ignore = "requires a non-root Linux container, a shiju gateway binary, Node.js, and npm access"]
fn acp_platform_snow_contract() {
    let gateway = std::env::var_os("SHIJU_ACP_SNOW_GATEWAY_BIN")
        .expect("set SHIJU_ACP_SNOW_GATEWAY_BIN to the current Linux shiju gateway binary");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");

    let status = Command::new("python3")
        .arg(root.join("scripts/acp_skill_coexistence/snow_linux.py"))
        .arg(gateway)
        .status()
        .unwrap();
    assert!(status.success(), "Snow Linux gateway probe exited with {status}");
}
