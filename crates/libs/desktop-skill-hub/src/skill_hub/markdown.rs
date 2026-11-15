/// 关闭代码语法高亮后解析公共内容，避免未知语言导致渲染失败。
pub fn parse_public_markdown(content: &str) -> Vec<iced::widget::markdown::Item> {
    let content = markdown_without_code_highlighting(content);
    iced::widget::markdown::parse(&content).collect()
}

/// 保留围栏代码内容，但移除开头语言标识以禁用语法高亮。
pub(super) fn markdown_without_code_highlighting(content: &str) -> String {
    let mut normalized = String::with_capacity(content.len());
    let mut open_fence = None;

    for line in content.split_inclusive('\n') {
        let body = line.trim_end_matches(['\r', '\n']);
        let line_ending = &line[body.len()..];

        if let Some((marker, minimum_length)) = open_fence {
            normalized.push_str(line);
            if closing_code_fence(body, marker, minimum_length) {
                open_fence = None;
            }
            continue;
        }

        let Some((indentation, marker, length)) = opening_code_fence(body) else {
            normalized.push_str(line);
            continue;
        };
        normalized.push_str(&body[..indentation]);
        normalized.extend(std::iter::repeat_n(marker, length));
        normalized.push_str(line_ending);
        open_fence = Some((marker, length));
    }

    normalized
}

fn opening_code_fence(line: &str) -> Option<(usize, char, usize)> {
    let indentation = line.bytes().take_while(|byte| *byte == b' ').count();
    if indentation > 3 {
        return None;
    }
    let rest = &line[indentation..];
    let marker = match rest.as_bytes().first() {
        Some(b'`') => '`',
        Some(b'~') => '~',
        _ => return None,
    };
    let length = rest.chars().take_while(|character| *character == marker).count();
    (length >= 3).then_some((indentation, marker, length))
}

fn closing_code_fence(line: &str, marker: char, minimum_length: usize) -> bool {
    let indentation = line.bytes().take_while(|byte| *byte == b' ').count();
    if indentation > 3 {
        return false;
    }
    let rest = &line[indentation..];
    let length = rest.chars().take_while(|character| *character == marker).count();
    length >= minimum_length && rest[length..].trim().is_empty()
}
