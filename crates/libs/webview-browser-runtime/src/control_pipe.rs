use std::io::{self, BufRead};
use std::thread;
use tao::event_loop::EventLoopProxy;
use webview_core::events::UserEvent;
use webview_ipc::{navigation_event_line, parse_stdin_event};

pub(crate) fn spawn(proxy: &EventLoopProxy<UserEvent>, embedded_mode: bool) {
    let stdin_proxy = proxy.clone();
    thread::spawn(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines().map_while(Result::ok) {
            if let Some(event) = parse_stdin_event(&line) {
                let _ = stdin_proxy.send_event(event);
            }
        }
        // The desktop owns the embedded control pipe. Closing it ends the view's lifetime.
        if embedded_mode {
            let _ = stdin_proxy.send_event(UserEvent::Close);
        }
    });
}

pub(crate) fn emit_navigation_url(url: &str) {
    use std::io::Write;

    let Some(line) = navigation_event_line(url) else {
        return;
    };
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    let _ = writeln!(output, "{line}");
    let _ = output.flush();
}
