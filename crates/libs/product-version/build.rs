use std::{env, fs, path::PathBuf};

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("crate directory"))
        .join("../../../Cargo.toml");
    println!("cargo:rerun-if-changed={}", manifest.display());

    let contents = fs::read_to_string(&manifest).expect("read workspace manifest");
    let workspace: toml::Table = contents.parse().expect("parse workspace manifest");
    let version = workspace
        .get("workspace")
        .and_then(|workspace| workspace.get("package"))
        .and_then(|package| package.get("version"))
        .and_then(toml::Value::as_str)
        .expect("workspace.package.version must be a string");
    println!("cargo:rustc-env=SHIJU_PRODUCT_VERSION={version}");
}
