//! Linux Harn source-build identity. The caller verifies the receipt and digests.
use std::{io::Read, path::Path};

pub(super) fn verify_linux_engine(
    engine: &Path,
    receipt: &serde_json::Value,
) -> Result<(), String> {
    let (target, machine) = match std::env::consts::ARCH {
        "x86_64" => ("x86_64-unknown-linux-gnu", 62),
        "aarch64" => ("aarch64-unknown-linux-gnu", 183),
        _ => return Err("Harn Linux engine target is unsupported".into()),
    };
    if receipt["target"].as_str() != Some(target) {
        return Err("Harn Linux build receipt target mismatch".into());
    }

    let mut header = [0; 20];
    std::fs::File::open(engine)
        .and_then(|mut file| file.read_exact(&mut header))
        .map_err(|_| "Cannot read Harn Linux ELF header")?;
    if &header[..7] != b"\x7fELF\x02\x01\x01"
        || u16::from_le_bytes([header[18], header[19]]) != machine
    {
        return Err("Harn Linux ELF architecture mismatch".into());
    }
    Ok(())
}
