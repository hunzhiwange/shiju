//! Preserve startup terminal input while filtering completed terminal-color replies.
//! 过滤完整终端颜色响应，同时保留启动阶段收到的用户输入。

const MAX_REPLAYED_OSC_BYTES: usize = 1024;

/// Preserve input and bracketed paste while omitting completed terminal-color replies.
/// 保留普通输入和括号粘贴，同时丢弃已完成的终端颜色响应。
///
/// Keep bounded incomplete OSC prefixes so unread continuations stay parser-framed.
/// 有界保留未完成的 OSC 前缀，使尚未读取的后续字节仍能由解析器按帧处理。
pub(super) fn startup_replay_input(input: &[u8]) -> Vec<u8> {
    const PASTE_START: &[u8] = b"\x1b[200~";
    const PASTE_END: &[u8] = b"\x1b[201~";

    let mut replay = Vec::with_capacity(input.len());
    let mut cursor = 0;
    let mut inside_paste = false;
    // 即使颜色响应夹在括号粘贴中也会过滤，但粘贴起止标记及其他载荷必须原样回放。
    while cursor < input.len() {
        if inside_paste {
            if input[cursor..].starts_with(PASTE_END) {
                replay.extend_from_slice(PASTE_END);
                cursor += PASTE_END.len();
                inside_paste = false;
            } else if let Some((slot, _)) = [(10, b"\x1b]10;"), (11, b"\x1b]11;")]
                .into_iter()
                .find(|(_, prefix)| input[cursor..].starts_with(*prefix))
                && let Some((end, terminator_len)) = super::osc_payload_end(
                    &input[cursor + 2..input.len().min(cursor + MAX_REPLAYED_OSC_BYTES)],
                )
                && let response = &input[cursor..cursor + 2 + end + terminator_len]
                && !response.windows(PASTE_END.len()).any(|window| window == PASTE_END)
                && super::parse_osc_color(response, slot).is_some()
            {
                cursor += 2 + end + terminator_len;
            } else {
                replay.push(input[cursor]);
                cursor += 1;
            }
            continue;
        }

        if input[cursor..].starts_with(PASTE_START) {
            replay.extend_from_slice(PASTE_START);
            cursor += PASTE_START.len();
            inside_paste = true;
            continue;
        }

        if input[cursor..].starts_with(b"\x1b]") {
            // 完整 OSC 在启动探测阶段已消费；不完整前缀最多保留固定长度，避免无界缓存。
            if let Some((end, terminator_len)) = super::osc_payload_end(&input[cursor + 2..]) {
                cursor += 2 + end + terminator_len;
                continue;
            }
            let end = input.len().min(cursor + MAX_REPLAYED_OSC_BYTES);
            replay.extend_from_slice(&input[cursor..end]);
            break;
        }

        replay.push(input[cursor]);
        cursor += 1;
    }
    replay
}

#[cfg(test)]
#[path = "startup_replay_tests.rs"]
mod tests;
