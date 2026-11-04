#!/usr/bin/env python3
"""Minimal newline-delimited JSON-RPC MCP demo server.

This intentionally uses only the Python standard library so the demo plugin can
run in a fresh checkout without installing extra packages.
"""

from __future__ import annotations

import json
import sys
from typing import Any


SERVER_INFO = {"name": "vibewindow-demo-plugin", "version": "0.1.0"}


def write_message(message: dict[str, Any]) -> None:
    sys.stdout.write(json.dumps(message, separators=(",", ":")) + "\n")
    sys.stdout.flush()


def result_response(message_id: Any, result: dict[str, Any]) -> dict[str, Any]:
    return {"jsonrpc": "2.0", "id": message_id, "result": result}


def error_response(message_id: Any, code: int, message: str) -> dict[str, Any]:
    return {"jsonrpc": "2.0", "id": message_id, "error": {"code": code, "message": message}}


def initialize_result() -> dict[str, Any]:
    return {
        "protocolVersion": "2025-06-18",
        "capabilities": {
            "tools": {},
            "resources": {},
            "prompts": {}
        },
        "serverInfo": SERVER_INFO,
        "instructions": "Demo MCP server packaged by vibewindow-demo-plugin."
    }


def tools_list_result() -> dict[str, Any]:
    return {
        "tools": [
            {
                "name": "demo_echo",
                "description": "Return the supplied message with demo metadata.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "message": {"type": "string"},
                        "uppercase": {"type": "boolean"}
                    },
                    "required": ["message"],
                    "additionalProperties": False
                }
            },
            {
                "name": "demo_sum",
                "description": "Return the sum of numeric values.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "values": {
                            "type": "array",
                            "items": {"type": "number"}
                        }
                    },
                    "required": ["values"],
                    "additionalProperties": False
                }
            }
        ]
    }


def call_tool(params: dict[str, Any]) -> dict[str, Any]:
    name = params.get("name")
    arguments = params.get("arguments") or {}

    if name == "demo_echo":
        message = str(arguments.get("message", ""))
        if arguments.get("uppercase"):
            message = message.upper()
        text = json.dumps(
            {
                "plugin": SERVER_INFO["name"],
                "tool": name,
                "message": message
            },
            ensure_ascii=False,
            sort_keys=True
        )
        return {"content": [{"type": "text", "text": text}]}

    if name == "demo_sum":
        values = arguments.get("values")
        if not isinstance(values, list) or not all(isinstance(value, (int, float)) for value in values):
            return {
                "isError": True,
                "content": [{"type": "text", "text": "values must be an array of numbers"}]
            }
        return {"content": [{"type": "text", "text": str(sum(values))}]}

    return {"isError": True, "content": [{"type": "text", "text": f"unknown tool: {name}"}]}


def resources_list_result() -> dict[str, Any]:
    return {
        "resources": [
            {
                "uri": "demo://plugin/summary",
                "name": "Demo plugin summary",
                "description": "Static summary resource from the demo plugin.",
                "mimeType": "text/plain"
            }
        ]
    }


def read_resource(params: dict[str, Any]) -> dict[str, Any]:
    uri = params.get("uri")
    if uri != "demo://plugin/summary":
        return {"contents": []}
    return {
        "contents": [
            {
                "uri": uri,
                "mimeType": "text/plain",
                "text": "vibewindow-demo-plugin exposes skills, MCP, hooks, and interface metadata."
            }
        ]
    }


def prompts_list_result() -> dict[str, Any]:
    return {
        "prompts": [
            {
                "name": "demo-plugin-tour",
                "description": "Explain the demo plugin component layout.",
                "arguments": []
            }
        ]
    }


def get_prompt(params: dict[str, Any]) -> dict[str, Any]:
    if params.get("name") != "demo-plugin-tour":
        return {"messages": []}
    return {
        "description": "Demo plugin tour",
        "messages": [
            {
                "role": "user",
                "content": {
                    "type": "text",
                    "text": "Describe how the VibeWindow demo plugin wires skills, MCP, hooks, and interface metadata."
                }
            }
        ]
    }


def handle_request(message: dict[str, Any]) -> dict[str, Any] | None:
    method = message.get("method")
    message_id = message.get("id")
    params = message.get("params") or {}

    if message_id is None:
        return None
    if method == "initialize":
        return result_response(message_id, initialize_result())
    if method == "ping":
        return result_response(message_id, {})
    if method == "tools/list":
        return result_response(message_id, tools_list_result())
    if method == "tools/call":
        return result_response(message_id, call_tool(params))
    if method == "resources/list":
        return result_response(message_id, resources_list_result())
    if method == "resources/read":
        return result_response(message_id, read_resource(params))
    if method == "prompts/list":
        return result_response(message_id, prompts_list_result())
    if method == "prompts/get":
        return result_response(message_id, get_prompt(params))
    return error_response(message_id, -32601, f"method not found: {method}")


def main() -> int:
    for line in sys.stdin:
        if not line.strip():
            continue
        try:
            message = json.loads(line)
            response = handle_request(message)
            if response is not None:
                write_message(response)
        except Exception as exc:
            write_message(error_response(None, -32603, f"internal error: {exc}"))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
