//! Encode the validated Chat history without provider state or endpoint selection.
use super::*;
fn copy(chat: &Value, output: &mut Value, fields: &[(&str, &str)]) {
    for (source, target) in fields {
        if let Some(v) = chat.get(*source) {
            output[*target] = v.clone();
        }
    }
}
fn max(chat: &Value) -> Option<&Value> {
    chat.get("max_tokens").or_else(|| chat.get("max_completion_tokens"))
}
fn choice(chat: &Value) -> Result<Option<&Value>> {
    let Some(choice) = chat.get("tool_choice") else { return Ok(None) };
    match choice.as_str() {
        Some("auto" | "none" | "required") => {}
        Some(_) => return Err(UNSUPPORTED),
        None => {
            keys(choice, &["type", "function"])?;
            keys(&choice["function"], &["name"])?;
            if choice["type"] != "function" {
                return Err(UNSUPPORTED);
            }
            let name = string(&choice["function"], "name")?;
            if !definitions(chat)?.iter().any(|f| f["name"] == name) {
                return Err(INVALID);
            }
        }
    }
    Ok(Some(choice))
}
fn stop(chat: &Value) -> Result<Option<Vec<Value>>> {
    let Some(v) = chat.get("stop").filter(|v| !v.is_null()) else { return Ok(None) };
    let v = if v.is_string() { vec![v.clone()] } else { v.as_array().ok_or(INVALID)?.clone() };
    if v.len() > 4 || v.iter().any(|v| v.as_str().is_none_or(str::is_empty)) {
        return Err(INVALID);
    }
    Ok(Some(v))
}
fn format(chat: &Value) -> Result<Option<&Value>> {
    let Some(v) = chat.get("response_format") else { return Ok(None) };
    match string(v, "type")? {
        "text" => keys(v, &["type"])?,
        "json_object" => keys(v, &["type"])?,
        "json_schema" => {
            keys(v, &["type", "json_schema"])?;
            let s = &v["json_schema"];
            keys(s, &["name", "description", "schema", "strict"])?;
            string(s, "name")?;
            if !s["schema"].is_object() {
                return Err(INVALID);
            }
        }
        _ => return Err(UNSUPPORTED),
    }
    Ok(Some(v))
}
pub(super) fn messages(chat: &Value) -> Result<Value> {
    // Messages requires an explicit bound; transport fills the selected model's limit.
    let mut out =
        json!({"model":chat["model"],"messages":[],"max_tokens":max(chat).ok_or(INVALID)?});
    copy(
        chat,
        &mut out,
        &[("stream", "stream"), ("temperature", "temperature"), ("top_p", "top_p")],
    );
    let mut system = Vec::new();
    let mut messages = Vec::new();
    for m in chat["messages"].as_array().ok_or(INVALID)? {
        let role = string(m, "role")?;
        let t = text(&m["content"])?;
        if matches!(role, "system" | "developer") {
            system.push(t);
            continue;
        }
        let mut parts = Vec::new();
        if role == "tool" {
            parts.push(json!({"type":"tool_result","tool_use_id":m["tool_call_id"],"content":t}));
        } else {
            if !t.is_empty() {
                parts.push(json!({"type":"text","text":t}));
            }
            if let Some(calls) = m["tool_calls"].as_array() {
                for call in calls {
                    parts.push(json!({"type":"tool_use","id":call["id"],"name":call["function"]["name"],"input":arguments(call)?}));
                }
            }
        }
        let role = if role == "tool" { "user" } else { role };
        messages.push(json!({"role":role,"content":parts}));
    }
    out["messages"] = messages.into();
    if !system.is_empty() {
        out["system"] = system.join("\n\n").into();
    }
    let defs = definitions(chat)?;
    if !defs.is_empty() {
        out["tools"] = defs
            .into_iter()
            .map(|f| {
                let mut tool = json!({"name":f["name"],"input_schema":f["parameters"]});
                for k in ["description", "strict"] {
                    if let Some(v) = f.get(k) {
                        tool[k] = v.clone();
                    }
                }
                tool
            })
            .collect::<Vec<_>>()
            .into();
    }
    if let Some(c) = choice(chat)? {
        out["tool_choice"] = match c.as_str() {
            Some("auto") => json!({"type":"auto"}),
            Some("none") => json!({"type":"none"}),
            Some("required") => json!({"type":"any"}),
            _ => json!({"type":"tool","name":c["function"]["name"]}),
        };
    }
    if let Some(parallel) = chat.get("parallel_tool_calls") {
        if out.get("tool_choice").is_none() {
            out["tool_choice"] = json!({"type":"auto"});
        }
        out["tool_choice"]["disable_parallel_tool_use"] =
            json!(!parallel.as_bool().ok_or(INVALID)?);
    }
    if let Some(stops) = stop(chat)? {
        out["stop_sequences"] = stops.into();
    }
    if let Some(f) = format(chat)? {
        match f["type"].as_str() {
            Some("text") => {}
            Some("json_schema") => {
                out["output_config"] =
                    json!({"format":{"type":"json_schema","schema":f["json_schema"]["schema"]}})
            }
            _ => return Err(UNSUPPORTED),
        }
    }
    if chat["reasoning_effort"] == "none" {
        out["thinking"] = json!({"type":"disabled"});
    }
    Ok(out)
}
pub(super) fn responses(chat: &Value) -> Result<Value> {
    let mut out = json!({"model":chat["model"],"input":[],"store":false});
    copy(
        chat,
        &mut out,
        &[
            ("stream", "stream"),
            ("temperature", "temperature"),
            ("top_p", "top_p"),
            ("parallel_tool_calls", "parallel_tool_calls"),
        ],
    );
    if let Some(n) = max(chat) {
        out["max_output_tokens"] = n.clone();
    }
    if stop(chat)?.is_some_and(|a| !a.is_empty()) {
        return Err(UNSUPPORTED);
    }
    let mut input = Vec::new();
    for m in chat["messages"].as_array().ok_or(INVALID)? {
        let role = string(m, "role")?;
        let t = text(&m["content"])?;
        if role == "tool" {
            input.push(
                json!({"type":"function_call_output","call_id":m["tool_call_id"],"output":t}),
            );
            continue;
        }
        if !t.is_empty() {
            input.push(json!({"type":"message","role":role,"content":[{"type":if role=="assistant"{"output_text"}else{"input_text"},"text":t}]}));
        }
        if let Some(calls) = m["tool_calls"].as_array() {
            for call in calls {
                input.push(json!({"type":"function_call","call_id":call["id"],"name":call["function"]["name"],"arguments":call["function"]["arguments"]}));
            }
        }
    }
    out["input"] = input.into();
    let defs = definitions(chat)?;
    if !defs.is_empty() {
        out["tools"] = defs
            .into_iter()
            .map(|mut f| {
                f["type"] = "function".into();
                f
            })
            .collect::<Vec<_>>()
            .into();
    }
    if let Some(c) = choice(chat)? {
        out["tool_choice"] = if c.is_string() {
            c.clone()
        } else {
            json!({"type":"function","name":c["function"]["name"]})
        };
    }
    if let Some(f) = format(chat)? {
        out["text"] = json!({"format":if f["type"]=="json_schema"{let mut s=f["json_schema"].clone();s["type"]="json_schema".into();s}else{f.clone()}});
    }
    if chat["reasoning_effort"] == "none" {
        out["reasoning"] = json!({"effort":"none"});
    }
    Ok(out)
}
pub(super) fn gemini(chat: &Value) -> Result<Value> {
    let mut out = json!({"contents":[],"generationConfig":{}});
    let mut system = Vec::new();
    let mut contents = Vec::new();
    let mut names = BTreeMap::new();
    for m in chat["messages"].as_array().ok_or(INVALID)? {
        let role = string(m, "role")?;
        let t = text(&m["content"])?;
        if matches!(role, "system" | "developer") {
            system.push(t);
            continue;
        }
        let mut parts = Vec::new();
        if role == "tool" {
            let id = string(m, "tool_call_id")?;
            let name = names.remove(id).ok_or(INVALID)?;
            // Tool output is an opaque text value. Do not parse/reinterpret its JSON.
            parts.push(json!({"functionResponse":{"id":id,"name":name,"response":{"output":t}}}));
        } else {
            if !t.is_empty() {
                parts.push(json!({"text":t}));
            }
            if let Some(calls) = m["tool_calls"].as_array() {
                for call in calls {
                    names.insert(
                        string(call, "id")?.to_owned(),
                        string(&call["function"], "name")?.to_owned(),
                    );
                    parts.push(json!({"functionCall":{"id":call["id"],"name":call["function"]["name"],"args":arguments(call)?}}));
                }
            }
        }
        contents.push(json!({"role":if role=="assistant"{"model"}else{"user"},"parts":parts}));
    }
    out["contents"] = contents.into();
    if !system.is_empty() {
        out["systemInstruction"] = json!({"parts":[{"text":system.join("\n\n")} ]});
    }
    copy(chat, &mut out["generationConfig"], &[("temperature", "temperature"), ("top_p", "topP")]);
    if let Some(n) = max(chat) {
        out["generationConfig"]["maxOutputTokens"] = n.clone();
    }
    if let Some(stops) = stop(chat)? {
        out["generationConfig"]["stopSequences"] = stops.into();
    }
    let defs = definitions(chat)?;
    if !defs.is_empty() {
        let mut functions = Vec::new();
        for mut f in defs {
            if f.get("strict").is_some_and(|v| v == true) {
                return Err(UNSUPPORTED);
            }
            let object = f.as_object_mut().ok_or(INVALID)?;
            object.remove("strict");
            let schema = object.remove("parameters").ok_or(INVALID)?;
            object.insert("parametersJsonSchema".into(), schema);
            functions.push(f);
        }
        out["tools"] = json!([{"functionDeclarations":functions}]);
    }
    if chat.get("parallel_tool_calls").is_some_and(|v| v == false) {
        return Err(UNSUPPORTED);
    }
    if let Some(c) = choice(chat)? {
        out["toolConfig"] = json!({"functionCallingConfig":match c.as_str(){Some("auto")=>json!({"mode":"AUTO"}),Some("none")=>json!({"mode":"NONE"}),Some("required")=>json!({"mode":"ANY"}),_=>json!({"mode":"ANY","allowedFunctionNames":[c["function"]["name"]]})}});
    }
    if let Some(f) = format(chat)? {
        match f["type"].as_str() {
            Some("text") => {}
            Some("json_object") => {
                out["generationConfig"]["responseMimeType"] = "application/json".into()
            }
            Some("json_schema") => {
                out["generationConfig"]["responseMimeType"] = "application/json".into();
                out["generationConfig"]["responseJsonSchema"] = f["json_schema"]["schema"].clone();
            }
            _ => return Err(UNSUPPORTED),
        }
    }
    if chat["reasoning_effort"] == "none" {
        out["generationConfig"]["thinkingConfig"] =
            json!({"thinkingBudget":0,"includeThoughts":false});
    }
    Ok(out)
}
