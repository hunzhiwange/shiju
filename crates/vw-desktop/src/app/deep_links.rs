//! Process-lifetime inbox. Reading never consumes a link; consumers acknowledge its ID
//! only after preview handoff or explicit rejection. URLs may contain secrets: never log them.
use std::collections::VecDeque;
use std::io;
use std::sync::Mutex;

#[derive(Clone)]
pub struct PendingLink {
    pub id: u64,
    pub url: String,
}

#[derive(Default)]
struct Inbox {
    next_id: u64,
    pending: VecDeque<PendingLink>,
    notify: bool,
}

static INBOX: Mutex<Inbox> = Mutex::new(Inbox {
    next_id: 0,
    pending: VecDeque::new(),
    notify: false,
});

impl Inbox {
    fn receive(&mut self, url: &str) -> io::Result<()> {
        if url.len() > 16 * 1024 || url.chars().any(char::is_control) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "启动链接无效"));
        }
        // LaunchServices and argv may deliver the same launch. Coalesce while pending,
        // but allow an intentional new launch after acknowledgement.
        if self.pending.iter().any(|link| link.url == url) {
            return Ok(());
        }
        let id = self.next_id.checked_add(1).ok_or_else(|| io::Error::other("链接编号已耗尽"))?;
        self.next_id = id;
        self.pending.push_back(PendingLink { id, url: url.to_owned() });
        self.notify = true;
        Ok(())
    }

    fn acknowledge(&mut self, id: u64) -> bool {
        let Some(index) = self.pending.iter().position(|link| link.id == id) else { return false };
        self.pending.remove(index);
        true
    }
}

/// Accept argv from the initial instance and subsequent instance deliveries.
/// Returns whether non-link arguments remain unsupported; never executes URL content.
pub(crate) fn receive_arguments(args: &[String]) -> io::Result<bool> {
    let mut inbox = INBOX.lock().map_err(|_| io::Error::other("链接队列不可用"))?;
    let mut unsupported = false;
    for arg in args {
        if arg.starts_with("ccswitch://") || arg.starts_with("shiju://") {
            inbox.receive(arg)?;
        } else if !arg.starts_with("-psn_") {
            unsupported = true;
        }
    }
    Ok(unsupported)
}

/// Snapshot for the preview owner; no destructive drain on a UI tick.
pub fn pending() -> io::Result<Vec<PendingLink>> {
    let inbox = INBOX.lock().map_err(|_| io::Error::other("链接队列不可用"))?;
    Ok(inbox.pending.iter().cloned().collect())
}

pub fn acknowledge(id: u64) -> io::Result<bool> {
    let mut inbox = INBOX.lock().map_err(|_| io::Error::other("链接队列不可用"))?;
    Ok(inbox.acknowledge(id))
}

pub(crate) fn take_notification() -> bool {
    INBOX.lock().map(|mut inbox| std::mem::take(&mut inbox.notify)).unwrap_or(false)
}

#[path = "deep_links_macos.rs"]
mod native;
pub use native::install_url_handler;

#[cfg(test)]
#[path = "desktop_integration_38_tests.rs"]
mod tests;
