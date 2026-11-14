//! Resident search citations; offsets are Unicode scalar indices, never UTF-8 bytes.
use serde_json::{Value, json};
use std::collections::HashSet;
#[path = "search_citations_markdown.rs"]
mod markdown;
use markdown::*;
pub(crate) fn text_with_url_citations(text: &str, annotations: &[Value]) -> String {
    struct Citation {
        start: Option<usize>,
        end: Option<usize>,
        raw_url: String,
        url: String,
        title: String,
    }

    let mut citations = Vec::new();
    for annotation in annotations {
        if annotation.get("type").and_then(Value::as_str) != Some("url_citation") {
            continue;
        }
        let Some(raw_url) = annotation.get("url").and_then(Value::as_str) else {
            continue;
        };
        let Some(url) = markdown_link_destination(raw_url) else {
            continue;
        };
        let title = annotation
            .get("title")
            .and_then(Value::as_str)
            .filter(|title| !title.trim().is_empty())
            .unwrap_or(raw_url)
            .to_string();
        let start = annotation
            .get("start_index")
            .and_then(Value::as_u64)
            .and_then(|index| usize::try_from(index).ok());
        let end = annotation
            .get("end_index")
            .and_then(Value::as_u64)
            .and_then(|index| usize::try_from(index).ok());
        citations.push(Citation { start, end, raw_url: raw_url.trim().to_string(), url, title });
    }

    if citations.is_empty() {
        return text.to_string();
    }

    let code = markdown_code_scan(text);
    let definitions = markdown_reference_definitions(text, &code.mask);
    let brackets = markdown_bracket_pairs(text, &code.mask);
    let reference_uses = markdown_reference_uses(text, &code.mask, &definitions, &brackets);
    let link_syntax =
        markdown_link_syntax_mask(text, &code.mask, &definitions, &reference_uses, &brackets);
    let mut linked_urls = citations
        .iter()
        .filter(|citation| {
            contains_markdown_link_to_url_with_context(
                text,
                &citation.raw_url,
                &citation.url,
                &code.mask,
                &definitions,
                &reference_uses,
                &brackets,
            )
        })
        .map(|citation| citation.url.clone())
        .collect::<HashSet<_>>();
    let mut ranged = Vec::new();
    let mut fallback = Vec::new();
    for citation in citations {
        let Some((start, end)) = citation.start.zip(citation.end) else {
            fallback.push(citation);
            continue;
        };
        let Some(start) = char_index_to_byte_offset(text, start) else {
            fallback.push(citation);
            continue;
        };
        let Some(end) = char_index_to_byte_offset(text, end) else {
            fallback.push(citation);
            continue;
        };
        if start >= end
            || text[start..end].trim().is_empty()
            || text[start..end]
                .chars()
                .any(|character| character.is_whitespace() && character != ' ')
            || code.mask[start..end].iter().any(|masked| *masked)
            || (link_syntax[start..end].iter().any(|masked| *masked)
                && !linked_urls.contains(&citation.url))
        {
            fallback.push(citation);
            continue;
        }
        ranged.push((start, end, citation));
    }
    ranged.sort_by_key(|(start, end, _)| (*start, *end));

    let mut rendered = String::with_capacity(text.len());
    let mut cursor = 0;
    for (start, end, citation) in ranged {
        if start < cursor {
            fallback.push(citation);
            continue;
        }
        rendered.push_str(&text[cursor..start]);
        let cited_text = &text[start..end];
        if linked_urls.contains(&citation.url) {
            rendered.push_str(cited_text);
            cursor = end;
            continue;
        }
        rendered.push('[');
        rendered.push_str(&markdown_link_label(cited_text));
        rendered.push_str("](");
        rendered.push_str(&citation.url);
        rendered.push(')');
        cursor = end;
        linked_urls.insert(citation.url);
    }
    rendered.push_str(&text[cursor..]);

    let mut fallback_links = Vec::new();
    for citation in fallback {
        if !linked_urls.insert(citation.url.clone()) {
            continue;
        }
        fallback_links.push(format!(
            "[{}]({})",
            markdown_link_label(&citation.title),
            citation.url
        ));
    }
    if !fallback_links.is_empty() {
        if !rendered.is_empty() {
            if let Some((marker, length)) = code.unterminated_fence {
                if !rendered.ends_with('\n') {
                    rendered.push('\n');
                }
                rendered.extend(std::iter::repeat_n(char::from(marker), length));
                rendered.push_str("\n\n");
            } else {
                rendered.push_str("\n\n");
            }
        }
        rendered.push_str("Sources: ");
        rendered.push_str(&fallback_links.join(", "));
    }
    rendered
}

pub(crate) fn output_text_with_url_citations(block: &Value) -> Option<String> {
    let text = block.get("text").and_then(Value::as_str).filter(|text| !text.is_empty())?;
    let annotations =
        block.get("annotations").and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default();
    Some(text_with_url_citations(text, annotations))
}

fn collect_web_search_results_from_content(content: &Value, results: &mut Vec<Value>) {
    let Some(blocks) = content.as_array() else {
        return;
    };

    for block in blocks {
        let Some(annotations) = block.get("annotations").and_then(Value::as_array) else {
            continue;
        };
        for annotation in annotations {
            if annotation.get("type").and_then(Value::as_str) != Some("url_citation") {
                continue;
            }
            let Some(url) =
                annotation.get("url").and_then(Value::as_str).filter(|url| !url.is_empty())
            else {
                continue;
            };
            let title = annotation
                .get("title")
                .and_then(Value::as_str)
                .filter(|title| !title.is_empty())
                .unwrap_or(url);
            results.push(json!({
                "type": "web_search_result",
                "url": url,
                "title": title,
                "encrypted_content": "",
                "page_age": null
            }));
        }
    }
}

pub(crate) fn web_search_results_from_output_item(item: &Value) -> Vec<Value> {
    let mut results = Vec::new();
    if item.get("type").and_then(Value::as_str) == Some("message") {
        if let Some(content) = item.get("content") {
            collect_web_search_results_from_content(content, &mut results);
        }
    } else if item.get("type").and_then(Value::as_str) == Some("output_text") {
        collect_web_search_results_from_content(&json!([item]), &mut results);
    }

    let mut seen = HashSet::new();
    results.retain(|result| {
        result.get("url").and_then(Value::as_str).is_some_and(|url| seen.insert(url.to_string()))
    });
    results
}

pub(crate) fn web_search_results_from_action(item: &Value) -> Vec<Value> {
    let mut results = Vec::new();
    let Some(sources) = item.pointer("/action/sources").and_then(Value::as_array) else {
        return results;
    };

    for source in sources {
        let Some(url) = source.get("url").and_then(Value::as_str).filter(|url| !url.is_empty())
        else {
            continue;
        };
        let title = source
            .get("title")
            .and_then(Value::as_str)
            .filter(|title| !title.is_empty())
            .unwrap_or(url);
        let page_age = source
            .get("page_age")
            .filter(|page_age| page_age.is_string())
            .cloned()
            .unwrap_or(Value::Null);
        results.push(json!({
            "type": "web_search_result",
            "url": url,
            "title": title,
            "encrypted_content": source.get("encrypted_content").cloned().unwrap_or(json!("")),
            "page_age": page_age
        }));
    }

    let mut seen = HashSet::new();
    results.retain(|result| {
        result.get("url").and_then(Value::as_str).is_some_and(|url| seen.insert(url.to_string()))
    });
    results
}

pub(crate) fn merge_web_search_result_metadata(target: &mut [Value], candidates: &[Value]) {
    for candidate in candidates {
        let Some(url) = candidate.get("url").and_then(Value::as_str).filter(|url| !url.is_empty())
        else {
            continue;
        };
        let Some(existing) =
            target.iter_mut().find(|result| result.get("url").and_then(Value::as_str) == Some(url))
        else {
            continue;
        };
        let Some(existing_object) = existing.as_object_mut() else {
            continue;
        };

        if let Some(candidate_title) = candidate
            .get("title")
            .and_then(Value::as_str)
            .filter(|title| !title.is_empty() && *title != url)
        {
            let existing_title_is_placeholder = existing_object
                .get("title")
                .and_then(Value::as_str)
                .is_none_or(|title| title.is_empty() || title == url);
            if existing_title_is_placeholder {
                existing_object.insert("title".to_string(), json!(candidate_title));
            }
        }

        if let Some(candidate_page_age) = candidate
            .get("page_age")
            .and_then(Value::as_str)
            .filter(|page_age| !page_age.is_empty())
        {
            let existing_page_age_is_missing =
                existing_object.get("page_age").and_then(Value::as_str).is_none_or(str::is_empty);
            if existing_page_age_is_missing {
                existing_object.insert("page_age".to_string(), json!(candidate_page_age));
            }
        }
    }
}
