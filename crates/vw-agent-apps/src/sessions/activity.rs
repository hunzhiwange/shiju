//! Conservative execution-host activity gate for destructive history operations.
use std::io;
use vw_api_types::external_session::ExternalSessionIdentity;

/// Called inside the host lifecycle guard. An application process may hold a
/// session without keeping its transcript open, so reject the whole application
/// while any matching command is present. Probe errors never establish inactivity.
pub fn is_active(identity: &ExternalSessionIdentity) -> io::Result<bool> {
    use vw_api_types::external_session::SessionProvider::*;
    let names: &[&str] = match identity.provider {
        Claude => &["claude"],
        Codex => &["codex"],
        Gemini => &["gemini"],
        Opencode => &["opencode"],
        Openclaw => &["openclaw"],
        Hermes => &["hermes"],
        Grokbuild => &["grok"],
        Pi => &["pi", "pi-coding-agent"],
        Omp => &["omp", "oh-my-pi"],
        Kimi => &["kimi"],
        _ => return Err(io::Error::new(io::ErrorKind::Unsupported, "activity probe unsupported")),
    };
    let table = tokio::runtime::Handle::current()
        .block_on(crate::aggregation::stale_clients::process_table())
        .ok_or_else(|| io::Error::other("host activity unavailable"))?;
    if table.trim().is_empty() {
        return Err(io::Error::other("empty host activity observation"));
    }
    for row in table.lines() {
        let mut fields = row.split_whitespace();
        if fields.next().and_then(|s| s.parse::<u32>().ok()).is_none() || fields.next().is_none() {
            return Err(io::Error::other("invalid host activity observation"));
        }
        for argument in fields {
            // Includes interpreter package paths and wrapper commands. Deliberate
            // overmatching errs toward refusal, never exposing raw arguments.
            if argument
                .split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
                .any(|part| names.iter().any(|name| part.eq_ignore_ascii_case(name)))
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
