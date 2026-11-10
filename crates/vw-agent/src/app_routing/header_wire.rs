//! Preserve only approved request header names from the first HTTP/1 head.

use axum::http::{HeaderMap, HeaderName, HeaderValue, header};
use std::{collections::HashMap, io, time::Duration};
use tokio::{io::AsyncReadExt, net::TcpStream};

pub(super) const HEAD_TIMEOUT: Duration = Duration::from_secs(5);
// Match the server's HTTP/1 parser limits.
pub(super) const MAX_HEAD_BYTES: usize = 8192 + 4096 * 100;
pub(super) const MAX_HEADERS: usize = 100;

#[derive(Clone, Default)]
pub(super) struct HeaderWire {
    names: Vec<(HeaderName, String)>,
    conflicting_framing: bool,
}

impl HeaderWire {
    fn parse(prefix: &[u8]) -> Option<Self> {
        let mut headers = [httparse::EMPTY_HEADER; MAX_HEADERS];
        let mut request = httparse::Request::new(&mut headers);
        match request.parse(prefix) {
            Ok(httparse::Status::Partial) => return None,
            Err(_) => return Some(Self::default()), // Hyper will reject the malformed request.
            Ok(httparse::Status::Complete(_)) => {}
        }
        // Hyper removes Content-Length when it sees Transfer-Encoding.
        // Preserve the conflict so dispatch can reject ambiguous framing.
        let has =
            |name: &str| request.headers.iter().any(|field| field.name.eq_ignore_ascii_case(name));
        Some(Self {
            names: request
                .headers
                .iter()
                .filter_map(|field| {
                    let name = HeaderName::from_bytes(field.name.as_bytes()).ok()?;
                    is_allowed(&name).then(|| (name, field.name.to_owned()))
                })
                .collect(),
            conflicting_framing: has("content-length") && has("transfer-encoding"),
        })
    }

    pub(super) fn has_conflicting_framing(&self) -> bool {
        self.conflicting_framing
    }

    pub(super) fn retain_allowed(&mut self, incoming: &HeaderMap, prepared: &mut HeaderMap) {
        // A validated provider override keeps its value and can reuse the
        // spelling of a surviving client field.
        self.names.retain(|(name, _)| incoming.contains_key(name));
        for (name, _) in &self.names {
            if !prepared.contains_key(name) {
                for value in incoming.get_all(name) {
                    prepared.append(name.clone(), value.clone());
                }
            }
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    pub(super) fn write(&self, headers: &HeaderMap, head: &mut Vec<u8>) {
        let mut values: HashMap<_, _> =
            headers.keys().map(|name| (name, headers.get_all(name).iter())).collect();
        for (name, spelling) in &self.names {
            if let Some(value) = values.get_mut(name).and_then(Iterator::next) {
                write_field(head, spelling, value);
            }
        }
        for name in headers.keys() {
            for value in values.remove(name).into_iter().flatten() {
                write_field(head, name.as_str(), value);
            }
        }
    }
}

fn is_allowed(name: &HeaderName) -> bool {
    matches!(*name, header::ACCEPT | header::ACCEPT_LANGUAGE | header::USER_AGENT)
}

fn write_field(head: &mut Vec<u8>, name: &str, value: &HeaderValue) {
    head.extend_from_slice(name.as_bytes());
    head.extend_from_slice(b": ");
    head.extend_from_slice(value.as_bytes());
    head.extend_from_slice(b"\r\n");
}

pub(super) async fn capture(mut socket: TcpStream) -> io::Result<(TcpStream, Vec<u8>, HeaderWire)> {
    let mut prefix = Vec::new();
    let mut chunk = [0; 4096];
    let mut wire = HeaderWire::default();
    while prefix.len() < MAX_HEAD_BYTES {
        let capacity = chunk.len().min(MAX_HEAD_BYTES - prefix.len());
        let read = socket.read(&mut chunk[..capacity]).await?;
        if read == 0 {
            break;
        }
        let scan_from = prefix.len().saturating_sub(3);
        prefix.extend_from_slice(&chunk[..read]);
        let suffix = &prefix[scan_from..];
        if (suffix.windows(2).any(|bytes| bytes == b"\n\n")
            || suffix.windows(3).any(|bytes| bytes == b"\n\r\n"))
            && let Some(parsed) = HeaderWire::parse(&prefix)
        {
            wire = parsed;
            break;
        }
    }
    Ok((socket, prefix, wire))
}

pub(super) fn strip_hop_headers(headers: &mut HeaderMap) {
    let nominations: Vec<HeaderName> = headers
        .get_all(header::CONNECTION)
        .iter()
        .flat_map(|value| value.as_bytes().split(|&byte| byte == b','))
        .filter_map(|name| HeaderName::from_bytes(name.trim_ascii()).ok())
        .collect();
    for name in nominations {
        headers.remove(name);
    }
    for name in [
        "connection",
        "keep-alive",
        "proxy-authenticate",
        "proxy-authorization",
        "proxy-connection",
        "te",
        "trailer",
        "trailers",
        "transfer-encoding",
        "upgrade",
    ] {
        headers.remove(name);
    }
}

pub(super) fn strip_rebuilt_entity_headers(headers: &mut HeaderMap) {
    for name in [
        "content-encoding",
        "content-length",
        "content-md5",
        "content-range",
        "content-digest",
        "digest",
        "repr-digest",
    ] {
        headers.remove(name);
    }
}
