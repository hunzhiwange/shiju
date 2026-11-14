//! Custom terminals are privileged execution-host settings, never request templates.
use crate::apps::launch::{request::LaunchMaterial, terminal_session::TerminalSession};
use crate::sessions::resume::ResumePlan;
use std::{
    path::PathBuf,
    process::{Command, Stdio},
};
use vw_api_types::external_session::ExternalSessionIdentity;

/// Non-wire capability created only after authorizing the execution host's
/// terminal settings. Ordinary launch/resume authorization does not grant the
/// right to change these settings. Do not construct this from request fields,
/// display commands, user preferences or imported session metadata.
///
/// Templates are argv arrays, not shell text. `{command}` must be one whole
/// argument and expands to the private PTY attachment program followed by its
/// arguments. `{cwd}` expands to the attachment directory as one argument.
/// Quoted, embedded and unknown placeholders are rejected, never interpolated.
pub struct AuthorizedCustomTerminal {
    program: PathBuf,
    arguments: Vec<String>,
}

impl AuthorizedCustomTerminal {
    /// The caller must load these values from separately authorized host
    /// settings under the same target-host guard used for resume execution.
    /// No API for accepting or persisting templates is provided here.
    pub fn from_authorized_host_settings(
        program: PathBuf,
        arguments: Vec<String>,
    ) -> Result<Self, String> {
        if program.as_os_str().is_empty() || arguments.is_empty() {
            return Err("custom terminal command template is empty".into());
        }
        if !program.is_absolute()
            || program.to_str().is_none_or(|value| value.chars().any(char::is_control))
        {
            return Err("custom terminal program must be an absolute host path".into());
        }
        let mut commands = 0;
        for argument in &arguments {
            if argument.chars().any(char::is_control) {
                return Err("invalid custom terminal argument".into());
            }
            match argument.as_str() {
                "{command}" => commands += 1,
                "{cwd}" => {}
                value if value.contains(['{', '}']) => {
                    return Err("unsupported custom terminal placeholder".into());
                }
                _ => {}
            }
        }
        if commands != 1 {
            return Err("custom terminal template requires exactly one {command} argument".into());
        }
        Ok(Self { program, arguments })
    }

    fn launch_attachment(
        &self,
        home: &std::path::Path,
        material: &LaunchMaterial,
    ) -> Result<(), String> {
        if material.program != std::path::Path::new("/bin/sh")
            || material.args.len() != 1
            || !material.cwd.is_absolute()
            || std::path::Path::new(&material.args[0]).parent() != Some(material.cwd.as_path())
        {
            return Err("invalid custom terminal attachment material".into());
        }
        let mut command = Command::new(&self.program);
        command.env_clear().env("HOME", home).env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin");
        command.current_dir(&material.cwd);
        for argument in &self.arguments {
            match argument.as_str() {
                "{command}" => {
                    command.arg(&material.program).args(&material.args);
                }
                "{cwd}" => {
                    command.arg(&material.cwd);
                }
                value => {
                    command.arg(value);
                }
            }
        }
        let status = command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|_| "cannot launch custom terminal")?;
        if !status.success() {
            return Err(format!("custom terminal launch failed ({status})"));
        }
        Ok(())
    }
}

/// Execute under the host authorization guard with a freshly rebuilt plan.
/// There is deliberately no string command or per-request template parameter.
pub fn launch_resume(
    plan: &ResumePlan,
    terminal: &AuthorizedCustomTerminal,
    resolve_agy: impl FnOnce(&ExternalSessionIdentity, &mut Command) -> Result<(), String>,
) -> Result<(), String> {
    let mut command = plan.command_with_credentials(resolve_agy)?;
    let home = command
        .get_envs()
        .find(|(key, _)| *key == "HOME")
        .and_then(|(_, value)| value)
        .map(PathBuf::from)
        .ok_or("resume execution home is missing")?;
    let session = TerminalSession::connect_with(std::path::Path::new("/tmp"), |_, material| {
        terminal.launch_attachment(&home, material)
    })?;
    let status = session.run(&mut command, || {}).map_err(|_| "native resume process failed")?;
    if !status.success() {
        return Err(format!("native resume process exited unsuccessfully ({status})"));
    }
    Ok(())
}
