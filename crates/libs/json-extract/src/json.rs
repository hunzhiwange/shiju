/// 从一段文本中尽可能提取出所有合法的 JSON 值。
pub fn extract_json_values(input: &str) -> Vec<serde_json::Value> {
    let mut values = Vec::new();
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return values;
    }

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        values.push(value);
        return values;
    }

    let char_positions: Vec<(usize, char)> = trimmed.char_indices().collect();
    let mut idx = 0;

    while idx < char_positions.len() {
        let (byte_idx, ch) = char_positions[idx];

        if ch == '{' || ch == '[' {
            let slice = &trimmed[byte_idx..];
            let mut stream =
                serde_json::Deserializer::from_str(slice).into_iter::<serde_json::Value>();

            if let Some(Ok(value)) = stream.next() {
                let consumed = stream.byte_offset();
                values.push(value);
                let next_byte = byte_idx + consumed;
                while idx < char_positions.len() && char_positions[idx].0 < next_byte {
                    idx += 1;
                }
                continue;
            }
        }
        idx += 1;
    }

    values
}

#[cfg(test)]
#[path = "json_tests.rs"]
mod json_tests;
