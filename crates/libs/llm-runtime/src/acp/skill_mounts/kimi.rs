//! Kimi 2.5.0 discovers extra roots through its private session-model TOML.
use agent_config::schema::AcpAgentConfig;
use sha2::{Digest, Sha256};
use std::{fs::File, io::Read, path::Path};

use super::{super::model_source::drivers::kimi::append_skill_root, MountedSkills};

const NATIVE_SHA256: &str = "d4dd02d40f18f3a784699fc37ae0c55d676a4704c37a3b91ab96541b0db673a1";

pub(super) fn configure(
    config: &mut AcpAgentConfig,
    view: Option<&MountedSkills>,
) -> Result<(), super::Error> {
    let Some(view) = view else {
        return Ok(());
    };
    configure_selected(config, &view.0).map_err(super::super::to_api_error)
}

fn configure_selected(
    config: &mut AcpAgentConfig,
    view: &agent_skills::session_mounts::SessionSkillView,
) -> anyhow::Result<()> {
    view.validate()?;
    let home = config.env.get("KIMI_CODE_HOME").ok_or_else(|| {
        anyhow::anyhow!("Kimi skill mounts require an owned session_model configuration")
    })?;
    let command = verify_entry(&config.command, &config.args)?;
    let command = command
        .into_os_string()
        .into_string()
        .map_err(|_| anyhow::anyhow!("Kimi executable requires a UTF-8 path"))?;
    append_skill_root(Path::new(home), view.root())?;
    config.command = command;
    Ok(())
}

fn verify_entry(command: &str, args: &[String]) -> anyhow::Result<std::path::PathBuf> {
    anyhow::ensure!(
        cfg!(all(target_os = "macos", target_arch = "aarch64")),
        "Kimi skill mounts require a reviewed platform"
    );
    let executable = Path::new(command);
    anyhow::ensure!(
        executable.is_absolute() && args == ["acp"],
        "Kimi skill mounts require the reviewed native ACP entry"
    );
    let executable = executable.canonicalize()?;
    anyhow::ensure!(executable.is_file(), "invalid Kimi executable");
    let mut hash = Sha256::new();
    let mut file = File::open(&executable)?;
    let mut buffer = [0; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    anyhow::ensure!(
        hex::encode(hash.finalize()) == NATIVE_SHA256,
        "Kimi skill mounts require the reviewed native 2.5.0 executable"
    );
    Ok(executable)
}

#[cfg(test)]
#[path = "kimi_tests.rs"]
mod tests;
