//! 构建桌面端编译期资源。
//! 构建脚本集中处理资源生成，避免运行时重复发现静态文件。

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn generate_app_state_struct(manifest_dir: &Path, out_dir: &Path) {
    let fragments = [
        "core_fields.rs",
        "chrome_fields.rs",
        "git_fields.rs",
        "settings_preview_fields.rs",
        "tool_fields.rs",
    ];
    let fragments_dir = manifest_dir.join("src/app/state/app_state");
    let mut generated = String::from("#[allow(dead_code)]\npub struct App {\n");

    for fragment in fragments {
        let path = fragments_dir.join(fragment);
        println!("cargo:rerun-if-changed={}", path.display());
        let content = fs::read_to_string(&path).expect("read app_state fragment");
        generated.push_str(&content);
        if !content.ends_with('\n') {
            generated.push('\n');
        }
    }

    generated.push_str("}\n");
    fs::write(out_dir.join("app_state_generated.rs"), generated)
        .expect("write app_state generated include");
}

fn png_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    if data.len() < 24 || &data[..8] != PNG_SIGNATURE {
        return None;
    }

    let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
    Some((width, height))
}

fn write_png_ico(png_path: &Path, ico_path: &Path) {
    let png = fs::read(png_path).expect("read windows icon png");
    let (width, height) = png_dimensions(&png).expect("windows icon source must be a png");
    let width_byte = if width >= 256 { 0 } else { width as u8 };
    let height_byte = if height >= 256 { 0 } else { height as u8 };

    let mut ico = Vec::with_capacity(22 + png.len());
    ico.extend_from_slice(&0u16.to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.push(width_byte);
    ico.push(height_byte);
    ico.push(0);
    ico.push(0);
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&32u16.to_le_bytes());
    ico.extend_from_slice(&(png.len() as u32).to_le_bytes());
    ico.extend_from_slice(&22u32.to_le_bytes());
    ico.extend_from_slice(&png);

    fs::write(ico_path, ico).expect("write generated windows ico");
}

fn rc_quoted_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\"")
}

fn try_compile_windows_resource(compiler: &str, rc_path: &Path, res_path: &Path) -> bool {
    // llvm-rc interprets absolute /Users paths as its /U option on Unix.
    let Ok(output) = Command::new(compiler)
        .current_dir(rc_path.parent().expect("resource directory"))
        .arg("/nologo")
        .arg("/fo")
        .arg(res_path.file_name().expect("resource output filename"))
        .arg(rc_path.file_name().expect("resource input filename"))
        .output()
    else {
        return false;
    };

    if output.status.success() {
        return true;
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("cargo:warning={compiler} failed while embedding Windows icon: {stderr}");
    false
}

fn embed_windows_icon(manifest_dir: &Path, out_dir: &Path) {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows")
        || env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc")
    {
        return;
    }

    println!("cargo:rerun-if-env-changed=PATH");

    let icon_png = manifest_dir.join("../../assets/logo/logo.png");
    println!("cargo:rerun-if-changed={}", icon_png.display());
    let app_manifest = manifest_dir.join("../../release/windows/shi-ju.manifest");
    println!("cargo:rerun-if-changed={}", app_manifest.display());

    let icon_ico = out_dir.join("VibeWindow.ico");
    let rc_path = out_dir.join("vibewindow-icon.rc");
    let res_path = out_dir.join("vibewindow-icon.res");

    write_png_ico(&icon_png, &icon_ico);
    // RT_MANIFEST (24), CREATEPROCESS_MANIFEST_RESOURCE_ID (1): the copied
    // update helper must start without requiring elevation before its handshake.
    fs::write(
        &rc_path,
        format!(
            "IDI_ICON1 ICON \"{}\"\n1 24 \"{}\"\n",
            rc_quoted_path(&icon_ico),
            rc_quoted_path(&app_manifest)
        ),
    )
    .expect("write windows application resources");

    for compiler in ["rc.exe", "rc", "llvm-rc.exe", "llvm-rc"] {
        if try_compile_windows_resource(compiler, &rc_path, &res_path) {
            println!("cargo:rustc-link-arg-bin=shi-ju={}", res_path.display());
            return;
        }
    }

    // Homebrew keeps LLVM off PATH because it is keg-only.
    if cfg!(target_os = "macos")
        && let Ok(output) = Command::new("brew").args(["--prefix", "llvm"]).output()
        && output.status.success()
    {
        let prefix = String::from_utf8_lossy(&output.stdout);
        let compiler = Path::new(prefix.trim()).join("bin/llvm-rc");
        if try_compile_windows_resource(&compiler.to_string_lossy(), &rc_path, &res_path) {
            println!("cargo:rustc-link-arg-bin=shi-ju={}", res_path.display());
            return;
        }
    }

    panic!(
        "Windows icon compilation requires rc.exe or llvm-rc; install LLVM and add its bin directory to PATH (see scripts/BUILDING.md)"
    );
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    embed_windows_icon(&manifest_dir, &out_dir);
    generate_app_state_struct(&manifest_dir, &out_dir);

}
