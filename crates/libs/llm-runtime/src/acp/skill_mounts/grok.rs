//! Grok Build's private session-model TOML accepts additional native skill roots.
use agent_config::schema::AcpAgentConfig;

use super::MountedSkills;

pub(super) fn configure(
    config: &mut AcpAgentConfig,
    view: Option<&MountedSkills>,
) -> Result<(), super::Error> {
    let Some(view) = view else {
        return Ok(());
    };
    configure_selected(config, &view.0).map_err(super::super::to_api_error)
}

#[cfg(not(any(
    all(target_os = "macos", target_arch = "aarch64"),
    all(
        target_os = "linux",
        target_env = "gnu",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
)))]
fn configure_selected(
    _: &mut AcpAgentConfig,
    _: &agent_skills::session_mounts::SessionSkillView,
) -> anyhow::Result<()> {
    anyhow::bail!("Grok Build skill mounts require a reviewed platform")
}

#[cfg(any(
    all(target_os = "macos", target_arch = "aarch64"),
    all(
        target_os = "linux",
        target_env = "gnu",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
fn configure_selected(
    config: &mut AcpAgentConfig,
    view: &agent_skills::session_mounts::SessionSkillView,
) -> anyhow::Result<()> {
    use sha2::{Digest, Sha256};
    use std::{
        io::{Read, Write},
        path::Path,
    };

    view.validate()?;
    let [isolated, script, template, _, executable, agent, stdio] = config.args.as_mut_slice()
    else {
        anyhow::bail!("Grok Build skill mounts require the managed native session_model entry");
    };
    let template_path = Path::new(template);
    anyhow::ensure!(
        config.command == "/usr/bin/python3"
            && isolated == "-I"
            && template_path.file_name() == Some("grok-model.toml".as_ref())
            && Path::new(script) == template_path.with_file_name("grok-projection.py")
            && agent == "agent"
            && stdio == "stdio",
        "Grok Build skill mounts require the managed native session_model entry"
    );
    let entry = Path::new(executable);
    anyhow::ensure!(entry.is_absolute(), "Grok Build executable must be absolute");
    let entry = entry.canonicalize()?;
    anyhow::ensure!(entry.is_file(), "invalid Grok Build executable");
    let mut digest = Sha256::new();
    let mut binary = std::fs::File::open(&entry)?;
    let mut buffer = [0; 64 * 1024];
    loop {
        let count = binary.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    anyhow::ensure!(
        hex::encode(digest.finalize()) == reviewed_entry_digest()?,
        "Grok Build skill mounts require the reviewed native 1.0.77 executable"
    );

    let source = std::fs::read_to_string(template_path)?;
    let contents = append_skill_root(&source, view.root())?;
    let directory =
        template_path.parent().ok_or_else(|| anyhow::anyhow!("invalid Grok config path"))?;
    let mut file = tempfile::NamedTempFile::new_in(directory)?;
    file.write_all(contents.as_bytes())?;
    let (_, path) = file.keep()?;
    *template = path
        .into_os_string()
        .into_string()
        .map_err(|_| anyhow::anyhow!("Grok Build config path requires UTF-8"))?;
    *executable = entry
        .into_os_string()
        .into_string()
        .map_err(|_| anyhow::anyhow!("Grok Build executable path requires UTF-8"))?;
    Ok(())
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn reviewed_entry_digest() -> anyhow::Result<String> {
    Ok("197c66a63e5ac46e07450b0ca21d141ce665050a412180f185dadfae1c7a5b2a".into())
}

#[cfg(all(
    target_os = "linux",
    target_env = "gnu",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn reviewed_entry_digest() -> anyhow::Result<String> {
    let manifest: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../patches/acp-model-source/grok/linux-artifacts.json"
    ))?;
    manifest["targets"][std::env::consts::ARCH]["sha256"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| anyhow::anyhow!("missing reviewed Grok Build executable digest"))
}

#[cfg(any(
    test,
    all(target_os = "macos", target_arch = "aarch64"),
    all(
        target_os = "linux",
        target_env = "gnu",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
fn append_skill_root(source: &str, root: &std::path::Path) -> anyhow::Result<String> {
    let root = root.to_str().ok_or_else(|| anyhow::anyhow!("Grok skill root requires UTF-8"))?;
    let mut config = source.parse::<toml::Table>()?;
    let skills = config
        .entry("skills")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()))
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("invalid Grok skills table"))?;
    let roots = skills
        .entry("paths")
        .or_insert_with(|| toml::Value::Array(Vec::new()))
        .as_array_mut()
        .ok_or_else(|| anyhow::anyhow!("invalid Grok skills paths"))?;
    let root = toml::Value::String(root.into());
    if !roots.contains(&root) {
        roots.push(root);
    }
    Ok(toml::to_string(&config)?)
}

#[cfg(test)]
#[path = "grok_tests.rs"]
mod tests;
