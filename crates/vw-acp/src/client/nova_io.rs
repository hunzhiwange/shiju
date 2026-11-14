//! Normalize Nova 1.1.65 permission callbacks and unsuccessful turn outcomes.
use super::*;

pub(super) struct NovaReader<R> {
    inner: R,
    enabled: bool,
    input: Vec<u8>,
    output: Vec<u8>,
    offset: usize,
}
impl<R> NovaReader<R> {
    pub(super) fn new(inner: R, enabled: bool) -> Self {
        Self { inner, enabled, input: Vec::new(), output: Vec::new(), offset: 0 }
    }
}
impl<R: AsyncRead + Unpin> AsyncRead for NovaReader<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        out: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if !self.enabled {
            return Pin::new(&mut self.inner).poll_read(cx, out);
        }
        if out.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }
        loop {
            if self.offset < self.output.len() {
                let end = (self.offset + out.remaining()).min(self.output.len());
                out.put_slice(&self.output[self.offset..end]);
                self.offset = end;
                return Poll::Ready(Ok(()));
            }
            if let Some(end) = self.input.iter().position(|b| *b == b'\n') {
                let frame: Vec<_> = self.input.drain(..=end).collect();
                self.output = frame;
                let mut value: Value =
                    serde_json::from_slice(&self.output).map_err(std::io::Error::other)?;
                if value.get("id").is_some() && value["method"] == "permission/request" {
                    value["method"] = json!("session/request_permission");
                    self.output = serde_json::to_vec(&value).map_err(std::io::Error::other)?;
                    self.output.push(b'\n');
                }
                let status = value["result"]["_meta"]["compass"]["turnOutcome"]["status"].as_str();
                if status == Some("queued") {
                    return Poll::Ready(Err(std::io::Error::other(
                        "Nova queued a turn instead of executing it",
                    )));
                }
                if status == Some("failed") {
                    value.as_object_mut().expect("RPC result object").remove("result");
                    value["error"] =
                        json!({"code":-32000,"message":"Nova did not complete the requested turn"});
                    self.output = serde_json::to_vec(&value).map_err(std::io::Error::other)?;
                    self.output.push(b'\n');
                }
                self.offset = 0;
                continue;
            }
            let mut bytes = [0; 8192];
            let mut buffer = ReadBuf::new(&mut bytes);
            match Pin::new(&mut self.inner).poll_read(cx, &mut buffer) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                Poll::Ready(Ok(())) => {
                    if buffer.filled().is_empty() {
                        return if self.input.is_empty() {
                            Poll::Ready(Ok(()))
                        } else {
                            Poll::Ready(Err(std::io::Error::new(
                                std::io::ErrorKind::UnexpectedEof,
                                "incomplete Nova frame",
                            )))
                        };
                    }
                    self.input.extend_from_slice(buffer.filled());
                    if self.input.len() > 16 * 1024 * 1024 {
                        return Poll::Ready(Err(std::io::Error::other("Nova frame exceeds limit")));
                    }
                }
            }
        }
    }
}
