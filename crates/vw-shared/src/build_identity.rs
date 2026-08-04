//! Executable build identity helpers shared by runtime and desktop bootstrap.

use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

/// Returns the SHA-256 digest of an executable file.
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

    Ok(format!("{:x}", hasher.finalize()))
}

/// Returns the SHA-256 digest of the current process executable.
pub fn current_executable_build_id() -> io::Result<String> {
    executable_build_id(std::env::current_exe()?)
}
