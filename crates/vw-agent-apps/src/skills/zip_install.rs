//! ZIP discovery keeps bounded extraction alive through each local transaction.
//! Callers authorize the archive and SSOT on the selected execution host.

use std::{fs, io, path::{Path, PathBuf}};
use serde::{Deserialize, Serialize};
use tempfile::TempDir;
use super::{archive::extract_local_zip, local_install::{LocalSkill, validate_directory}};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZipSkippedSkill {
    pub directory: String,
    pub existing_id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZipFailedSkill {
    pub directory: String,
    pub reason: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZipInstallResult {
    pub installed: Vec<LocalSkill>,
    pub skipped: Vec<ZipSkippedSkill>,
    pub failed: Vec<ZipFailedSkill>,
}

pub struct ZipSkill {
    pub directory: String,
    pub source: PathBuf,
}

pub struct ZipImport {
    _extracted: TempDir,
    pub skills: Vec<ZipSkill>,
}

impl ZipImport {
    /// Fully validate/extract before allowing any SSOT writes. An invalid skill
    /// name is retained for per-item rejection rather than aborting other items.
    pub fn prepare(archive: &Path) -> io::Result<Self> {
        if !archive.is_absolute() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "archive path must be absolute"));
        }
        let extracted = extract_local_zip(archive)?;
        let mut pending = vec![extracted.path().to_owned()];
        let mut skills = Vec::new();
        while let Some(path) = pending.pop() {
            if path.join("SKILL.md").is_file() {
                let directory = if path == extracted.path() {
                    archive.file_stem().and_then(|name| name.to_str())
                } else {
                    path.file_name().and_then(|name| name.to_str())
                }.unwrap_or("").to_owned();
                skills.push(ZipSkill { directory, source: path });
                // A skill owns its subtree; embedded examples are not installed.
                continue;
            }
            let mut children = Vec::new();
            for entry in fs::read_dir(&path)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    children.push(entry.path());
                }
            }
            children.sort();
            pending.extend(children.into_iter().rev());
        }
        if skills.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "ZIP contains no skills"));
        }
        Ok(Self { _extracted: extracted, skills })
    }
}

impl ZipSkill {
    pub fn validate(&self) -> Result<(), super::local_install::LocalInstallError> {
        validate_directory(&self.directory)
    }
}
