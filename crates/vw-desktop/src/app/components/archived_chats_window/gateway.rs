//! Loopback-only HTTP boundary. All mutable data lives in a temporary directory.
use super::fixture;
use std::io::{Read, Write};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use vw_shared::session::info::Info;

pub(super) struct Gateway {
    pub port: u16,
    pub requests: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    _home: tempfile::TempDir,
}
impl Gateway {
    pub fn start() -> Self {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join("sessions.json");
        std::fs::write(&path, serde_json::to_vec(&fixture()).unwrap()).unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        let stop = Arc::new(AtomicBool::new(false));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stopped = stop.clone();
        let recorded = requests.clone();
        let thread = std::thread::spawn(move || {
            let mut fail_list = true;
            let mut fail_restore = true;
            let mut fail_batch = true;
            while !stopped.load(Ordering::Relaxed) {
                let (mut stream, _) = match listener.accept() {
                    Ok(value) => value,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(5));
                        continue;
                    }
                    Err(error) => panic!("fixture accept: {error}"),
                };
                stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
                let mut bytes = Vec::new();
                let header_end;
                loop {
                    let mut buffer = [0; 4096];
                    let count = stream.read(&mut buffer).unwrap();
                    assert!(count > 0, "incomplete fixture request");
                    bytes.extend_from_slice(&buffer[..count]);
                    if let Some(index) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        header_end = index + 4;
                        break;
                    }
                }
                let header = String::from_utf8(bytes[..header_end].to_vec()).unwrap();
                let length = header
                    .lines()
                    .find_map(|line| {
                        let (key, value) = line.split_once(':')?;
                        key.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                while bytes.len() < header_end + length {
                    let mut buffer = [0; 4096];
                    let count = stream.read(&mut buffer).unwrap();
                    assert!(count > 0);
                    bytes.extend_from_slice(&buffer[..count]);
                }
                let request = header.lines().next().unwrap().to_owned();
                recorded.lock().unwrap().push(request.clone());
                let mut parts = request.split_whitespace();
                let method = parts.next().unwrap();
                let uri = parts.next().unwrap();
                let route = uri.split('?').next().unwrap();
                let id =
                    route.strip_prefix("/v1/session/").unwrap_or("").split('/').next().unwrap();
                let mut rows: Vec<Info> =
                    serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
                let mut status = "200 OK";
                let body = if method == "GET" && route == "/v1/session" {
                    if std::mem::take(&mut fail_list) {
                        status = "503 Service Unavailable";
                        serde_json::json!({"error":"fixture list failure"})
                    } else {
                        serde_json::to_value(
                            rows.iter()
                                .filter(|row| {
                                    !uri.contains("projectID=batch") || row.project_id == "batch"
                                })
                                .collect::<Vec<_>>(),
                        )
                        .unwrap()
                    }
                } else if method == "PATCH" && route.starts_with("/v1/session/") {
                    if id == "chat-0000" && std::mem::take(&mut fail_restore) {
                        status = "503 Service Unavailable";
                        serde_json::json!({"error":"fixture restore failure"})
                    } else {
                        let patch: serde_json::Value =
                            serde_json::from_slice(&bytes[header_end..]).unwrap();
                        let row = rows.iter_mut().find(|row| row.id == id).unwrap();
                        row.time.archived =
                            patch["time"]["archived"].as_u64().filter(|value| *value != 0);
                        serde_json::to_value(row).unwrap()
                    }
                } else if method == "DELETE" && route.ends_with("/archived") {
                    if id == "chat-0004" && std::mem::take(&mut fail_batch) {
                        status = "503 Service Unavailable";
                        serde_json::json!({"error":"fixture batch failure"})
                    } else {
                        assert!(rows.iter().any(|row| row.id == id && row.time.archived.is_some()));
                        let mut ids = vec![id.to_owned()];
                        loop {
                            let before = ids.len();
                            for row in &rows {
                                if row.parent_id.as_ref().is_some_and(|parent| ids.contains(parent))
                                    && !ids.contains(&row.id)
                                {
                                    ids.push(row.id.clone());
                                }
                            }
                            if before == ids.len() {
                                break;
                            }
                        }
                        rows.retain(|row| !ids.contains(&row.id));
                        serde_json::json!({"deleted_ids":ids})
                    }
                } else if method == "GET" && route.starts_with("/v1/session/") {
                    serde_json::to_value(rows.iter().find(|row| row.id == id).unwrap()).unwrap()
                } else if (method == "GET" && route == "/v1/projects/batch")
                    || (method == "POST" && route == "/v1/projects/resolve")
                {
                    serde_json::json!({"project": {
                        "id":"batch", "name":"同名项目", "directory":"/isolated/batch", "display_path":"/isolated/batch",
                        "status":"ready", "created_at_ms":1, "updated_at_ms":2,
                        "git":{"is_repo":false,"has_uncommitted_changes":false}
                    }})
                } else {
                    status = "404 Not Found";
                    serde_json::json!({"error":"fixture route unavailable"})
                };
                std::fs::write(&path, serde_json::to_vec(&rows).unwrap()).unwrap();
                let body = serde_json::to_vec(&body).unwrap();
                write!(stream, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
                stream.write_all(&body).unwrap();
            }
        });
        Self { port, requests, stop, thread: Some(thread), _home: home }
    }
}
impl Drop for Gateway {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.take().unwrap().join().unwrap();
    }
}
