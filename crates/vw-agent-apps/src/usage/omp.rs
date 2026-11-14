//! Oh My Pi usage projection. The execution host supplies authorized files,
//! including nested task sessions, and the resolved Pi/OMP roots. Discovery and
//! ledger scope binding remain with their existing owners.
use std::{
    fs::{self, File},
    io::{self, BufRead, BufReader, Read},
    path::{Path, PathBuf},
};

use serde_json::Value;

use super::pi::{self, Flavor, PiUsageMessage, PiUsageSnapshot};

/// Bound the probe independently of the size of the first session record.
const TITLE_SLOT_PROBE_BYTES: u64 = 4096;

/// Read errors remain errors: an unreadable source must not silently move to
/// the other application's importer. Missing files can be retried by the host.
pub fn opens_with_title_slot(path: &Path) -> io::Result<bool> {
    let file = File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Expected a session file"));
    }
    let mut line = Vec::new();
    BufReader::new(file.take(TITLE_SLOT_PROBE_BYTES)).read_until(b'\n', &mut line)?;
    // A truncated probe cannot establish ownership, even if its JSON prefix
    // happens to be valid before padding or a later record.
    if line.last() != Some(&b'\n') { return Ok(false); }
    Ok(serde_json::from_slice::<Value>(&line)
        .is_ok_and(|value| value["type"] == "title"))
}

/// Compare canonical roots so aliases cannot let both importers claim a store.
/// Root resolution and project-context requirements are handled by the native
/// session providers; do not substitute process-global environment values here.
pub fn shares_root_with_pi(omp_roots: &[PathBuf], pi_roots: &[PathBuf]) -> io::Result<bool> {
    fn canonical_roots(roots: &[PathBuf]) -> io::Result<Vec<PathBuf>> {
        roots.iter().map(|root| match fs::canonicalize(root) {
            Ok(path) => Ok(path),
            // A not-yet-created absolute root still has a stable identity.
            Err(error) if error.kind() == io::ErrorKind::NotFound && root.is_absolute() => Ok(root.clone()),
            Err(error) => Err(error),
        }).collect()
    }
    let omp = canonical_roots(omp_roots)?;
    let pi = canonical_roots(pi_roots)?;
    Ok(omp.iter().any(|root| pi.contains(root)))
}

/// Dedicated stores retain all authorized files. In a shared store only title
/// slot files belong to OMP; Pi uses the exact complement via `pi::own_files`.
/// Filter each resolved store separately when an instance has multiple roots.
pub fn own_files(files: Vec<PathBuf>, shared_with_pi: bool) -> io::Result<Vec<PathBuf>> {
    select_files(files, shared_with_pi, true)
}

pub(crate) fn select_files(files: Vec<PathBuf>, shared: bool, omp: bool) -> io::Result<Vec<PathBuf>> {
    if !shared { return Ok(files); }
    let mut owned = Vec::new();
    for path in files {
        if opens_with_title_slot(&path)? == omp { owned.push(path); }
    }
    Ok(owned)
}

/// Reuse Pi's bounded parser and revision checks. OMP has no Pi first-import
/// cutoff or embedded subagent projection: task children carry their own usage.
pub fn parse_file(path: &Path) -> io::Result<PiUsageSnapshot> {
    pi::parse_file_for(path, Flavor::Omp)
}

pub fn parse_reader(reader: impl BufRead) -> io::Result<(Vec<PiUsageMessage>, bool)> {
    pi::parse_reader_for(reader, Flavor::Omp)
}
