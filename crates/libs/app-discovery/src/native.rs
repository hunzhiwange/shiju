//! Read-only native executable identification.
use std::path::Path;

pub fn is_native_executable(path: &Path) -> bool {
    use std::io::Read;
    if !std::fs::symlink_metadata(path).is_ok_and(|m| m.is_file()) {
        return false;
    }
    let mut magic = [0; 4];
    std::fs::File::open(path).and_then(|mut f| f.read_exact(&mut magic)).is_ok()
        && (magic == [0x7f, b'E', b'L', b'F']
            || magic[..2] == *b"MZ"
            || matches!(
                magic,
                [0xcf, 0xfa, 0xed, 0xfe]
                    | [0xfe, 0xed, 0xfa, 0xcf]
                    | [0xce, 0xfa, 0xed, 0xfe]
                    | [0xfe, 0xed, 0xfa, 0xce]
                    | [0xca, 0xfe, 0xba, 0xbe]
            ))
}
