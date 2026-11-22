use std::{path::Path, process::Command};

#[test]
#[ignore = "requires a non-root Linux container, a shiju gateway binary, Node.js and npm access"]
fn acp_platform_codebuddy_contract() {
    let gateway = std::env::var_os("SHIJU_ACP_CODEBUDDY_GATEWAY_BIN")
        .expect("set SHIJU_ACP_CODEBUDDY_GATEWAY_BIN to the current Linux gateway binary");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let output = Command::new("python3")
        .arg(root.join("scripts/acp_skill_coexistence/codebuddy_linux.py"))
        .arg(gateway)
        .output()
        .expect("launch the CodeBuddy gateway probe");
    assert!(
        output.status.success(),
        "CodeBuddy gateway probe failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    print!("{}", String::from_utf8_lossy(&output.stdout));
}
