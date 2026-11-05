"""Google model wire normalization; discovery and tool execution remain in Gemini."""
import json


def generate_message(fixture, body):
    messages = [{"role": "system", "content": json.dumps(body.get("systemInstruction", {}))}]
    pending = {}
    for content in body.get("contents", []):
        for part in content.get("parts", []):
            if "functionCall" in part:
                call = part["functionCall"]
                pending[call["name"]] = call.get("id") or next(
                    (key for key in reversed(fixture.calls)
                     if fixture.gemini_calls.get(key) == call["name"]), "uncorrelated")
            if "functionResponse" in part:
                result = part["functionResponse"]
                messages.append({"role": "tool", "tool_call_id": result.get("id") or pending.get(result["name"]),
                                 "content": json.dumps(result.get("response", {}), ensure_ascii=False)})
            if "text" in part:
                messages.append({"role": "assistant" if content.get("role") == "model" else "user", "content": part["text"]})
    tools = [{"name": tool["name"], "parameters": tool.get("parameters", tool.get("parametersJsonSchema", {})),
              "description": tool.get("description", "")}
             for group in body.get("tools", []) for tool in group.get("functionDeclarations", [])]
    message, finish = fixture.response({"messages": messages, "tools": tools})
    if message.get("tool_calls"):
        call = message["tool_calls"][0]
        fixture.gemini_calls[call["id"]] = call["function"]["name"]
    return message, finish


def generate_wire(message):
    if message.get("tool_calls"):
        call = message["tool_calls"][0]
        parts = [{"functionCall": {"id": call["id"], "name": call["function"]["name"],
                                   "args": json.loads(call["function"]["arguments"])}}]
    else:
        parts = [{"text": message["content"]}]
    value = {"candidates": [{"content": {"role": "model", "parts": parts}, "finishReason": "STOP", "index": 0}],
             "usageMetadata": {"promptTokenCount": 1, "candidatesTokenCount": 1, "totalTokenCount": 2}}
    return value, ("data: " + json.dumps(value) + "\n\n").encode()
