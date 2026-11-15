use std::borrow::Cow;
use super::markdown::markdown_code_fence;
pub fn normalize_display_text<'a>(s: &'a str) -> Cow<'a, str> {
    if s.is_empty()
        || (!s.contains('\r')
            && !s.contains("\n\n\n")
            && !s.contains(" \n")
            && !s.contains("\t\n")
            && !s.contains("\n \n")
            && !s.contains("\n\t\n"))
    {
        return Cow::Borrowed(s);
    }

    let mut out = String::with_capacity(s.len());
    let mut prev_blank = false;
    let mut open_fence = None;
    for raw_line in s.split('\n') {
        let line = raw_line.trim_end_matches('\r');
        let fence = markdown_code_fence(line);
        if open_fence.is_some() || fence.is_some() {
            match (open_fence, fence) {
                (None, Some((fence, _))) => open_fence = Some(fence),
                (Some(open), Some((close, suffix)))
                    if close.starts_with(open) && suffix.trim().is_empty() =>
                {
                    open_fence = None;
                }
                _ => {}
            }
            out.push_str(line);
            out.push('\n');
            prev_blank = false;
            continue;
        }

        let line = line.trim_end_matches([' ', '\t']);
        if line.is_empty() {
            if prev_blank {
                continue;
            }
            out.push('\n');
            prev_blank = true;
        } else {
            out.push_str(line);
            out.push('\n');
            prev_blank = false;
        }
    }
    if out.ends_with('\n') {
        out.pop();
    }
    Cow::Owned(out)
}
