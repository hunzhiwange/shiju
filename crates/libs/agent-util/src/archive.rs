//! 提供跨平台压缩包解压工具。
//! 实现将平台差异局部化，并在 wasm32 这类不支持执行解压命令的目标上显式返回错误。

use std::io;
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
use std::process::Command;

/// 执行 extract_zip 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
#[cfg(target_arch = "wasm32")]
pub fn extract_zip(zip_path: impl AsRef<Path>, dest_dir: impl AsRef<Path>) -> io::Result<()> {
    let _ = zip_path;
    let _ = dest_dir;
    Err(io::Error::new(io::ErrorKind::Unsupported, "extract zip not supported on wasm32"))
}

/// 执行 extract_zip 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
#[cfg(not(target_arch = "wasm32"))]
pub fn extract_zip(zip_path: impl AsRef<Path>, dest_dir: impl AsRef<Path>) -> io::Result<()> {
    let zip_path = zip_path.as_ref();
    let dest_dir = dest_dir.as_ref();

    #[cfg(windows)]
    {
        let zip = zip_path.to_string_lossy().replace('\'', "''");
        let dest = dest_dir.to_string_lossy().replace('\'', "''");
        let cmd = format!(
            "$global:ProgressPreference = 'SilentlyContinue'; Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
            zip, dest
        );
        let status = Command::new("powershell")
            .arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-Command")
            .arg(cmd)
            .status()?;
        if status.success() {
            return Ok(());
        }
        return Err(io::Error::other("extract zip failed"));
    }

    #[cfg(not(windows))]
    {
        let status = Command::new("unzip")
            .arg("-o")
            .arg("-q")
            .arg(zip_path)
            .arg("-d")
            .arg(dest_dir)
            .status()?;
        if status.success() {
            return Ok(());
        }
        Err(io::Error::other("extract zip failed"))
    }
}
