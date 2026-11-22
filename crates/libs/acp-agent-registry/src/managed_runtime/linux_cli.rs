//! Fixed upstream Linux executables for native accounts and models.
use super::{Stage, root};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs, io,
    io::Read,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::Duration,
};

const CORUST: &str =
    include_str!("../../../../../patches/acp-model-source/corust-agent/linux-x64.json");
const CORUST_INSTALL: &str =
    include_str!("../../../../../patches/acp-model-source/corust-agent/install_linux.py");

#[derive(Deserialize)]
struct Manifest {
    version: String,
    directory: Option<String>,
    #[serde(alias = "entry")]
    bin: String,
    args: Vec<String>,
    archive_entry: Option<String>,
    targets: BTreeMap<String, Target>,
}

#[derive(Deserialize)]
struct Target {
    #[serde(alias = "archive", alias = "artifact_url")]
    url: String,
    archive_sha256: Option<String>,
    #[serde(alias = "sha256")]
    executable_sha256: String,
}

#[derive(Deserialize)]
struct CorustManifest {
    version: String,
    target: String,
    artifact_url: String,
    archive_sha256: String,
    bin: String,
    files: BTreeMap<String, String>,
}

struct Artifact {
    directory: String,
    bin: String,
    args: Vec<String>,
    archive_entry: Option<String>,
    url: String,
    archive_sha256: Option<String>,
    executable_sha256: String,
}

fn contract(agent: &str, arch: &str) -> Result<Option<Artifact>, String> {
    if !matches!(arch, "x86_64" | "aarch64") {
        return Ok(None);
    }
    if agent == "corust-agent" {
        let manifest: CorustManifest =
            serde_json::from_str(CORUST).map_err(|_| "Invalid Corust Linux CLI contract")?;
        if manifest.target != format!("{arch}-unknown-linux-gnu") {
            return Ok(None);
        }
        return Ok(Some(Artifact {
            directory: format!("corust-agent-{}-linux-x64-native", manifest.version),
            executable_sha256: manifest
                .files
                .get(&manifest.bin)
                .ok_or("Missing Corust Linux executable digest")?
                .clone(),
            archive_entry: Some(manifest.bin.clone()),
            bin: manifest.bin,
            args: vec![],
            url: manifest.artifact_url,
            archive_sha256: Some(manifest.archive_sha256),
        }));
    }
    let (manifest, target) = match agent {
        "omp" => (super::omp::MANIFEST, format!("{arch}-unknown-linux-gnu")),
        "stakpak" => (super::stakpak::LINUX_CLI_CONTRACT, arch.into()),
        "vtcode" => (super::vtcode::LINUX_CLI_CONTRACT, arch.into()),
        _ => return Ok(None),
    };
    let mut manifest: Manifest =
        serde_json::from_str(manifest).map_err(|_| "Invalid fixed Linux CLI contract")?;
    let target = manifest.targets.remove(&target).ok_or("Missing fixed Linux CLI target")?;
    if agent == "omp" {
        manifest.directory = Some(format!("omp-{}-linux-cli-{arch}", manifest.version));
    }
    if agent == "stakpak" {
        manifest.archive_entry = Some(manifest.bin.clone());
    }
    Ok(Some(Artifact {
        directory: manifest.directory.ok_or("Missing Linux CLI installation directory")?,
        bin: manifest.bin,
        args: manifest.args,
        archive_entry: manifest.archive_entry,
        url: target.url,
        archive_sha256: target.archive_sha256,
        executable_sha256: target.executable_sha256,
    }))
}

pub(super) fn discover(agent: &str, root: &Path) -> Result<Option<(String, Vec<String>)>, String> {
    match agent {
        "stakpak" => return super::stakpak::cli_local_command(root),
        "vtcode" => return super::vtcode::cli_local_command(root),
        _ => {}
    }
    let Some(artifact) = contract(agent, std::env::consts::ARCH)? else {
        return Ok(None);
    };
    let root = match root.canonicalize() {
        Ok(root) => root,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("Cannot resolve Linux ACP installation root".into()),
    };
    let directory = root.join(&artifact.directory);
    match directory.symlink_metadata() {
        Ok(metadata) if metadata.is_dir() => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        _ => return Err("Linux ACP installation directory is invalid".into()),
    }
    let executable = if agent == "omp" {
        super::omp::verify(&directory)?
    } else {
        let executable = directory.join(&artifact.bin);
        let metadata = executable.symlink_metadata().map_err(|_| "Missing fixed ACP executable")?;
        if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
            return Err("Fixed Linux ACP executable must be a regular executable".into());
        }
        let mut file =
            fs::File::open(&executable).map_err(|_| "Cannot read fixed ACP executable")?;
        let mut digest = Sha256::new();
        let mut buffer = [0; 64 * 1024];
        loop {
            let count = file.read(&mut buffer).map_err(|_| "Cannot hash fixed ACP executable")?;
            if count == 0 {
                break;
            }
            digest.update(&buffer[..count]);
        }
        if hex::encode(digest.finalize()) != artifact.executable_sha256 {
            return Err("Fixed Linux ACP executable checksum mismatch".into());
        }
        executable
    };
    let command = executable.to_str().ok_or("Linux ACP executable path is not Unicode")?;
    Ok(Some((command.into(), artifact.args)))
}

pub(super) fn verify_launch(
    agent: &str,
    root: &Path,
    command: &Path,
    args: &[String],
) -> Result<(), String> {
    let Some(artifact) = contract(agent, std::env::consts::ARCH)? else {
        return Ok(());
    };
    let relative_entry = Path::new(&artifact.directory).join(&artifact.bin);
    // Persisted absolute paths to the fixed installation must be reviewed again
    // on spawn, including a reconnect that no longer uses registry discovery.
    if command != root.join(&relative_entry)
        && !root.canonicalize().is_ok_and(|root| command == root.join(relative_entry))
    {
        return Ok(());
    }
    let (_, reviewed_args) =
        discover(agent, root)?.ok_or("Fixed Linux ACP executable is missing")?;
    if args != reviewed_args.as_slice() {
        return Err("Unexpected fixed Linux ACP arguments".into());
    }
    Ok(())
}

pub(super) async fn install(agent: &str) -> Result<(), String> {
    let artifact = contract(agent, std::env::consts::ARCH)?
        .ok_or("No fixed ACP executable for this Linux architecture")?;
    let root = root(agent)?;
    if discover(agent, &root)?.is_some() {
        return Ok(());
    }
    let stage = Stage::new(agent, &root)?;
    let staging_root = &stage.0;
    let runtime = staging_root.join(&artifact.directory);
    for (name, content) in [
        ("install.py", include_str!("install.py")),
        ("install_linux.py", CORUST_INSTALL),
        ("linux-x64.json", CORUST),
    ] {
        fs::write(staging_root.join(name), content)
            .map_err(|_| "Cannot stage embedded Linux ACP installer")?;
    }
    let script = staging_root.join("install.py");
    crate::install::run_installer(
        "python3",
        &[
            script.to_str().ok_or("Invalid installer path")?,
            staging_root.to_str().ok_or("Invalid installation stage path")?,
            runtime.to_str().ok_or("Invalid runtime path")?,
            agent,
            "--linux-cli",
            &artifact.url,
            artifact.archive_sha256.as_deref().unwrap_or_default(),
            artifact.archive_entry.as_deref().unwrap_or_default(),
            &artifact.bin,
            &artifact.executable_sha256,
        ],
        tokio::time::Instant::now() + Duration::from_secs(300),
    )
    .await?;
    discover(agent, staging_root)?.ok_or("Fixed ACP runtime was not staged")?;
    let destination: PathBuf = root.join(&artifact.directory);
    if fs::rename(&runtime, &destination).is_err() && discover(agent, &root)?.is_none() {
        return Err("Cannot atomically publish fixed ACP runtime".into());
    }
    discover(agent, &root)?.ok_or("Fixed ACP runtime was not installed")?;
    Ok(())
}

#[cfg(test)]
#[path = "linux_cli_tests.rs"]
mod tests;
