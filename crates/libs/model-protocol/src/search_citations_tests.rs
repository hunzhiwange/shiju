use super::search_citations::text_with_url_citations;
use serde_json::json;

#[test]
fn cc_route_045_search_citations_expected_behavior() {
    let annotations =
        json!([{"type":"url_citation","url":"https://example.com","start_index":2,"end_index":4}]);
    assert_eq!(
        text_with_url_citations("你好世界😀", annotations.as_array().unwrap()),
        "你好[世界](https://example.com)😀"
    );
}

#[test]
fn cc_route_045_existing_links_and_code_are_preserved() {
    for text in ["[source](https://example.com)", "[source][ref]\n\n[ref]: https://example.com"] {
        let annotations = json!([{"type":"url_citation","url":"https://example.com","start_index":1,"end_index":7}]);
        assert_eq!(text_with_url_citations(text, annotations.as_array().unwrap()), text);
    }
    let annotations = json!([{"type":"url_citation","url":"https://example.com","title":"source","start_index":4,"end_index":8}]);
    assert_eq!(
        text_with_url_citations("```\ncode\n```", annotations.as_array().unwrap()),
        "```\ncode\n```\n\nSources: [source](https://example.com)"
    );
}

#[test]
fn cc_route_045_search_metadata_is_enriched() {
    let body = json!({"id":"resp","status":"completed","output":[
        {"id":"search","type":"web_search_call","status":"completed","action":{"sources":[{"url":"https://example.com","page_age":"today"}]}},
        {"id":"msg","type":"message","role":"assistant","status":"completed","content":[{"type":"output_text","text":"你好","annotations":[{"type":"url_citation","url":"https://example.com","title":"Example","start_index":0,"end_index":2}]}]}
    ],"usage":{"input_tokens":1,"output_tokens":1}});
    let search = super::hosted_search::Search { name: Some("web_search".into()), max: None };
    let result = super::messages_responses_result::with_search(&body, "model", &search).unwrap();
    assert_eq!(result["content"][1]["content"][0]["title"], "Example");
    assert_eq!(result["content"][1]["content"][0]["page_age"], "today");
    assert_eq!(result["content"][2]["text"], "[你好](https://example.com)");
}

#[test]
fn cc_route_045_stream_waits_for_annotation_snapshot() {
    let mut decoder = super::messages_responses_stream::Decoder::new("model".into());
    decoder.frame(&json!({"type":"response.created","response":{"id":"r"}}).to_string()).unwrap();
    decoder
        .frame(
            &json!({"type":"response.output_item.added","output_index":0,
        "item":{"id":"m","type":"message","content":[]}})
            .to_string(),
        )
        .unwrap();
    let delta = decoder
        .frame(
            &json!({"type":"response.output_text.delta","output_index":0,
        "content_index":0,"delta":"你好"})
            .to_string(),
        )
        .unwrap();
    assert!(!delta.contains("text_delta"));
    let terminal = decoder
        .frame(
            &json!({"type":"response.completed","response":{
        "id":"r","status":"completed","output":[{"id":"m","type":"message","role":"assistant",
        "status":"completed","content":[{"type":"output_text","text":"你好","annotations":[
        {"type":"url_citation","url":"https://example.com","start_index":0,"end_index":2}]}]}],
        "usage":{"input_tokens":1,"output_tokens":1}}})
            .to_string(),
        )
        .unwrap();
    assert!(terminal.contains("[你好](https://example.com)"));
    assert!(decoder.done);
}
