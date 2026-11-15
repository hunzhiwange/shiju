use super::messages_gemini_request::convert;
use axum::http::StatusCode;
use serde_json::json;

#[test]
fn cc_route_047_messages_gemini_request_expected_behavior() {
    let body = json!({"model":"gemini-test","max_tokens":100,"system":"rules",
        "temperature":0.2,"top_p":0.8,"stop_sequences":["END"],
        "tools":[{"name":"lookup","input_schema":{"type":"object","properties":{}}}],
        "tool_choice":{"type":"tool","name":"lookup"},
        "messages":[{"role":"user","content":[{"type":"text","text":"see"},
            {"type":"image","source":{"type":"base64","media_type":"image/png","data":"aGk="}},
            {"type":"document","source":{"type":"base64","media_type":"application/pdf","data":"cGRm"}}]}]});
    let out = convert(&body).unwrap();
    assert_eq!(out["systemInstruction"]["parts"][0]["text"], "rules");
    assert_eq!(out["contents"][0]["role"], "user");
    assert_eq!(out["contents"][0]["parts"][0]["text"], "see");
    assert_eq!(out["contents"][0]["parts"][1]["inlineData"]["mimeType"], "image/png");
    assert_eq!(out["contents"][0]["parts"][2]["inlineData"]["mimeType"], "application/pdf");
    assert_eq!(out["generationConfig"]["maxOutputTokens"], 100);
    assert_eq!(out["generationConfig"]["stopSequences"], json!(["END"]));
    assert_eq!(out["toolConfig"]["functionCallingConfig"]["allowedFunctionNames"], json!(["lookup"]));
    assert!(out.get("model").is_none());
}

#[test]
fn cc_route_047_preserves_tool_correlation_and_followup_media() {
    let out = convert(&json!({"model":"gemini-test","max_tokens":100,"messages":[
        {"role":"assistant","content":[{"type":"tool_use","id":"call1","name":"lookup","input":{}}]},
        {"role":"user","content":[{"type":"tool_result","tool_use_id":"call1","content":"result"}]},
        {"role":"user","content":[{"type":"image","source":{"type":"base64","media_type":"image/png","data":"aGk="}}]}
    ]})).unwrap();
    assert_eq!(out["contents"][0]["role"], "model");
    assert_eq!(out["contents"][1]["parts"][0]["functionResponse"]["name"], "lookup");
    assert_eq!(out["contents"][2]["parts"][0]["inlineData"]["data"], "aGk=");
}

#[test]
fn cc_route_047_rejects_remote_media() {
    assert_eq!(convert(&json!({"model":"gemini-test","max_tokens":100,"messages":[
        {"role":"user","content":[{"type":"image","source":{"type":"url","url":"https://example.com/a.png"}}]}
    ]})).unwrap_err(), StatusCode::UNPROCESSABLE_ENTITY);
}
