//! Discord 令牌与用户 ID 相关工具模块
//!
//! 本模块提供用于 Discord Bot 令牌解析与用户 ID 提取的最小化、无外部依赖实现。
//! 主要用于从 Bot 令牌结构中安全地提取 Bot 用户 ID，以支持 Discord 通道的标识与校验。
//!
//! 核心功能：
//! - 轻量 Base64 解码（`base64_decode`）：仅支持本场景所需的编码形式。
//! - 从令牌提取 Bot 用户 ID（`bot_user_id_from_token`）：基于 Discord 令牌结构解析。
//!
//! 注意：本实现有意保持最小化，以避免引入额外依赖与复杂度，仅满足当前模块需要。

/// Base64 解码使用的标准字母表，遵循 RFC 4648。
const BASE64_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// 最小化 Base64 解码（无外部依赖）——仅需解码 Discord 令牌中的用户 ID 部分
///
/// 参数：
/// - `input`: 待解码的 Base64 字符串（可包含或不包含填充）。
///
/// 返回：
/// - `Some(String)`: 解码成功后的 UTF-8 字符串。
/// - `None`: 输入含非法字符或解码结果非合法 UTF-8 时返回。
///
/// 实现要点：
/// - 自动按需补充 `=` 填充到 4 的倍数。
/// - 逐块解码，正确处理填充字节。
#[allow(clippy::cast_possible_truncation)]
pub(super) fn base64_decode(input: &str) -> Option<String> {
    // 按长度对 4 取模，补齐 Base64 填充字符 "="
    let padded = match input.len() % 4 {
        2 => format!("{input}=="),
        3 => format!("{input}="),
        _ => input.to_string(),
    };

    let mut bytes = Vec::new();
    let chars: Vec<u8> = padded.bytes().collect();

    for chunk in chars.chunks(4) {
        // 长度不足一个完整块时提前结束
        if chunk.len() < 4 {
            break;
        }

        // 将每 4 个 Base64 字符映射到 6 位索引，"=" 视作 0
        let mut v = [0usize; 4];
        for (i, &b) in chunk.iter().enumerate() {
            if b == b'=' {
                v[i] = 0;
            } else {
                v[i] = BASE64_ALPHABET.iter().position(|&a| a == b)?;
            }
        }

        // 重组 4 个 6 位索引为 3 个 8 位字节，按填充位置决定是否写入
        bytes.push(((v[0] << 2) | (v[1] >> 4)) as u8);
        if chunk[2] != b'=' {
            bytes.push((((v[1] & 0xF) << 4) | (v[2] >> 2)) as u8);
        }
        if chunk[3] != b'=' {
            bytes.push((((v[2] & 0x3) << 6) | v[3]) as u8);
        }
    }

    // 将解码后的字节按 UTF-8 转换为字符串
    String::from_utf8(bytes).ok()
}

/// 从 Discord Bot 令牌中提取 Bot 用户 ID
///
/// 参数：
/// - `token`: Discord Bot 令牌字符串。
///
/// 返回：
/// - `Some(String)`: 成功提取的用户 ID 字符串。
/// - `None`: 令牌格式无效或解析失败时返回。
///
/// 说明：
/// - Discord Bot 令牌结构为：`base64(bot_user_id).timestamp.hmac`。
/// - 本函数解析第一段 Base64 编码部分，还原为用户 ID。
pub(super) fn bot_user_id_from_token(token: &str) -> Option<String> {
    // Discord bot tokens are base64(bot_user_id).timestamp.hmac
    let part = token.split('.').next()?;
    base64_decode(part)
}

#[cfg(test)]
#[path = "ids_tests.rs"]
mod ids_tests;
