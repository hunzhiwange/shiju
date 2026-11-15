//! 终端交互式权限确认提示。

use std::io::{self, IsTerminal as _};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionPromptOptions {
    pub prompt: String,
    pub header: Option<String>,
    pub details: Option<String>,
}

pub fn can_prompt_for_permission() -> bool {
    io::stdin().is_terminal() && io::stderr().is_terminal()
}

fn prompt_for_permission_with_io<R, W>(
    options: &PermissionPromptOptions,
    reader: &mut R,
    writer: &mut W,
) -> io::Result<bool>
where
    R: io::BufRead,
    W: io::Write,
{
    if let Some(header) = options.header.as_deref() {
        writeln!(writer)?;
        writeln!(writer, "{header}")?;
    }
    if let Some(details) = options.details.as_deref()
        && !details.trim().is_empty()
    {
        writeln!(writer, "{details}")?;
    }
    write!(writer, "{}", options.prompt)?;
    writer.flush()?;

    let mut answer = String::new();
    reader.read_line(&mut answer)?;
    let normalized = answer.trim().to_ascii_lowercase();
    Ok(matches!(normalized.as_str(), "y" | "yes"))
}

pub fn prompt_for_permission(options: &PermissionPromptOptions) -> io::Result<bool> {
    if !can_prompt_for_permission() {
        return Ok(false);
    }

    let stdin = io::stdin();
    let mut stdin = stdin.lock();
    let mut stderr = io::stderr().lock();
    prompt_for_permission_with_io(options, &mut stdin, &mut stderr)
}

#[cfg(test)]
#[path = "permission_prompt_tests.rs"]
mod permission_prompt_tests;
