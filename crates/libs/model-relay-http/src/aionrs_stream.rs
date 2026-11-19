//! Preserve reasoning as reasoning when the native client receives Chat SSE.
use axum::body::{Body, Bytes};
use futures_util::StreamExt;
use model_protocol::{responses_chat_stream, sse_framing::Lines};
use std::io;

pub(super) fn separate_inline_reasoning(body: Body) -> Body {
    Body::from_stream(futures_util::stream::unfold(
        (body.into_data_stream(), Lines::default(), responses_chat_stream::State::default(), false),
        |(mut stream, mut lines, mut state, stopped)| async move {
            if stopped {
                return None;
            }
            loop {
                let next = stream.next().await;
                let eof = next.is_none();
                let result = match next {
                    Some(Ok(chunk)) => normalize(&mut lines, &mut state, &chunk),
                    Some(Err(_)) => Err(io::Error::other("aionrs upstream SSE read failed")),
                    None if lines.is_empty() => return None,
                    None => normalize(&mut lines, &mut state, b"\n"),
                };
                match result {
                    Ok(chunk) if chunk.is_empty() && !eof => continue,
                    Ok(chunk) => {
                        return Some((Ok(Bytes::from(chunk)), (stream, lines, state, eof)));
                    }
                    Err(error) => return Some((Err(error), (stream, lines, state, true))),
                }
            }
        },
    ))
}

fn normalize(
    lines: &mut Lines,
    state: &mut responses_chat_stream::State,
    bytes: &[u8],
) -> Result<String, io::Error> {
    let mut output = String::new();
    let lines = lines
        .push(bytes, 1024 * 1024)
        .map_err(|_| io::Error::other("invalid aionrs upstream SSE framing"))?;
    for line in lines {
        if let Some(payload) = line.strip_prefix("data:").map(str::trim)
            && !payload.is_empty()
            && payload != "[DONE]"
        {
            let mut chunk: serde_json::Value = serde_json::from_str(payload)?;
            if chunk.get("choices").is_some() {
                model_protocol::inline_think::normalize(&mut state.inline_think, &mut chunk)
                    .map_err(|_| io::Error::other("invalid aionrs Chat stream delta"))?;
            }
            output.push_str("data: ");
            output.push_str(&chunk.to_string());
        } else {
            output.push_str(&line);
        }
        output.push('\n');
    }
    Ok(output)
}

#[cfg(test)]
#[path = "aionrs_stream_tests.rs"]
mod tests;
