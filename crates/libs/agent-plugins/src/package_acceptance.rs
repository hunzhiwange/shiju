//! Package lifecycle contracts driven by real application execution ports.
//! These functions are host-invoked acceptance drivers, not standalone domain tests.
pub mod skill;
pub mod platform;
use std::{path::Path, fs};
use sha2::{Digest, Sha256};
fn public_original(fixtures: &Path, root: &Path, name: &str) -> std::path::PathBuf {
    let archive = fs::File::open(fixtures.join("public").join(format!("{name}.tar.gz"))).unwrap();
    tar::Archive::new(flate2::read::GzDecoder::new(archive)).unpack(root).unwrap();
    root.join(name)
}
fn digest(root: &Path) -> Vec<(String, String)> {
    fn collect(root: &Path, path: &Path, files: &mut Vec<(String, String)>) {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() { collect(root, &entry.path(), files); }
            else if entry.file_type().unwrap().is_file() {
                files.push((entry.path().strip_prefix(root).unwrap().to_string_lossy().into_owned(), hex::encode(Sha256::digest(fs::read(entry.path()).unwrap()))));
            }
        }
    }
    let mut files = Vec::new();
    collect(root, root, &mut files);
    files.sort();
    files
}
