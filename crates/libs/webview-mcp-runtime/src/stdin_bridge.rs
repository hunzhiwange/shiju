//! Forward bounded parent messages into the native event loop.
use std::io::{self, BufRead};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use tao::event_loop::EventLoopProxy;
use webview_mcp_policy::MAX_MESSAGE_BYTES;
use crate::runtime::AppEvent;

pub(crate) fn spawn(proxy: EventLoopProxy<AppEvent>, active: Arc<AtomicBool>) {
    let pipe_active = active;
    std::thread::spawn(move || {
        let mut input = io::stdin().lock();
        loop {
            let mut line = String::new();
            match std::io::Read::take(&mut input, (MAX_MESSAGE_BYTES + 1) as u64)
                .read_line(&mut line)
            {
                Ok(0) | Err(_) => break,
                Ok(_) if line.len() > MAX_MESSAGE_BYTES => break,
                Ok(_) => {}
            }
            let line = line.trim_end_matches(['\r', '\n']);
            if line == "close" {
                break;
            }
            if let Ok(value) = serde_json::from_str(line) {
                let _ = proxy.send_event(AppEvent::Deliver(value));
            }
        }
        pipe_active.store(false, Ordering::Release);
        let _ = proxy.send_event(AppEvent::Close);
    });
}
