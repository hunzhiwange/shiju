//! 运行时与桌面启动流程共享的可执行文件构建身份工具。

use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

/// 流式读取指定可执行文件，并返回其小写十六进制 SHA-256 摘要。
pub fn executable_build_id(path: impl AsRef<Path>) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hex::encode(hasher.finalize()))
}

/// 定位当前进程的可执行文件，并返回与 [`executable_build_id`] 相同格式的摘要。
pub fn current_executable_build_id() -> io::Result<String> {
    executable_build_id(std::env::current_exe()?)
}
