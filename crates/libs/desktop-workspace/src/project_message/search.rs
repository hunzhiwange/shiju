use desktop_file_tree::state::FindInFolderMatch;
use regex::RegexBuilder;
fn is_word_byte(b: u8) -> bool { b.is_ascii_alphanumeric() || b == b'_' }
pub fn search(scope_path: String, query: String, case_sensitive: bool, whole_word: bool, use_regex: bool, files: Vec<String>) -> (Vec<FindInFolderMatch>, Option<desktop_i18n::input::Feedback>, bool) {
const MAX_FIND_RESULTS: usize = 3000;
                let mut out = Vec::<FindInFolderMatch>::new();
                let mut error: Option<desktop_i18n::input::Feedback> = None;
                let mut limit_reached = false;
                let mut compiled_regex = None;

                if use_regex {
                    let mut pattern = query.clone();
                    if whole_word {
                        pattern = format!(r"\b(?:{})\b", pattern);
                    }
                    let mut builder = RegexBuilder::new(&pattern);
                    builder.case_insensitive(!case_sensitive);
                    match builder.build() {
                        Ok(re) => compiled_regex = Some(re),
                        Err(e) => {
                            error = Some(desktop_i18n::input::Feedback::new(
                                "正则表达式错误: %s",
                                vec![e.to_string().into()],
                            ));
                        }
                    }
                }

                if error.is_none() {
                    let scope = std::path::Path::new(&scope_path);
                    let scope_prefix = scope.to_string_lossy().to_string();
                    let plain_query =
                        if case_sensitive { query.clone() } else { query.to_ascii_lowercase() };

                    'files: for path in files {
                        if !(path == scope_prefix
                            || path.starts_with(&(scope_prefix.clone() + "/")))
                        {
                            continue;
                        }
                        let Ok(content) = std::fs::read_to_string(&path) else {
                            continue;
                        };

                        for (li, line) in content.lines().enumerate() {
                            if let Some(re) = &compiled_regex {
                                for m in re.find_iter(line) {
                                    out.push(FindInFolderMatch {
                                        path: path.clone(),
                                        line: li + 1,
                                        column: m.start() + 1,
                                        preview: line.trim().to_string(),
                                        match_len: m.end().saturating_sub(m.start()),
                                    });
                                    if out.len() >= MAX_FIND_RESULTS {
                                        limit_reached = true;
                                        break 'files;
                                    }
                                }
                                continue;
                            }

                            if plain_query.is_empty() {
                                continue;
                            }

                            let (line_cmp, needle) = if case_sensitive {
                                (line.to_string(), plain_query.as_str())
                            } else {
                                (line.to_ascii_lowercase(), plain_query.as_str())
                            };

                            let mut start = 0usize;
                            while start <= line_cmp.len() {
                                let Some(pos) = line_cmp[start..].find(needle) else {
                                    break;
                                };
                                let st = start + pos;
                                let ed = st + needle.len();

                                if whole_word {
                                    let bytes = line_cmp.as_bytes();
                                    let left_ok = st == 0 || !is_word_byte(bytes[st - 1]);
                                    let right_ok = ed >= bytes.len() || !is_word_byte(bytes[ed]);
                                    if !(left_ok && right_ok) {
                                        start = st + needle.len();
                                        continue;
                                    }
                                }

                                out.push(FindInFolderMatch {
                                    path: path.clone(),
                                    line: li + 1,
                                    column: st + 1,
                                    preview: line.trim().to_string(),
                                    match_len: needle.len(),
                                });
                                if out.len() >= MAX_FIND_RESULTS {
                                    limit_reached = true;
                                    break 'files;
                                }
                                start = st + needle.len();
                            }
                        }
                    }
                }

                (out, error, limit_reached)
}

#[cfg(test)]
#[path = "search_tests.rs"]
mod search_tests;
