"""Observe the official caller's dispatch boundary; never copy its raw errors to diagnostics."""
import json
import os
import runpy
import sys
import uuid

fd = int(sys.argv[1])
caller, operation = sys.argv[2:4]
sys.argv = [caller, *sys.argv[4:]]
sys.path.insert(0, os.path.dirname(caller))
state = {"observed": False, "attempted": False, "unresolved": False,
         "rejected": False, "phase": "preflight", "http_status": None,
         "rpc_code": None, "request_id": None}
code = 1


def observe_post(original):
    def post(client, payload):
        business = (payload.get("method") == "tools/call"
                    and payload.get("params", {}).get("name") == operation)
        previous = state["unresolved"]
        if business:
            state.update(attempted=True, unresolved=True, rejected=False,
                         phase="request", http_status=None, rpc_code=None, request_id=None)
        try:
            response = original(client, payload)
            if business:
                status, parsed, _body = response
                if isinstance(status, int) and 100 <= status <= 599:
                    state["http_status"] = status
                messages = [item for value in parsed
                            for item in (value if isinstance(value, list) else [value])
                            if isinstance(item, dict) and item.get("id") in (payload.get("id"), None)]
                errors = [item["error"] for item in messages if isinstance(item.get("error"), dict)]
                matched_errors = [item["error"] for item in messages
                                  if item.get("id") == payload.get("id")
                                  and isinstance(item.get("error"), dict)]
                has_result = any("result" in item for item in messages)
                protocol_reject = any(error.get("code") in (-32700, -32600, -32601, -32602)
                                      for error in matched_errors)
                stale = status == 404 and any(error.get("message") == "MCP session not found or expired"
                                              for error in errors)
                draining = status == 503 and any(isinstance(error.get("data"), dict)
                    and error["data"].get("code") == "SERVICE_RESTARTING"
                    and error["data"].get("requestStarted") is False for error in errors)
                if not has_result and (status in (401, 403) or protocol_reject or stale or draining):
                    state.update(unresolved=previous, rejected=True)
                if errors and isinstance(errors[0].get("code"), int):
                    state["rpc_code"] = errors[0]["code"]
            return response
        finally:
            if business:
                diagnostic = getattr(client, "transport_diagnostic", {})
                phase = diagnostic.get("phase")
                if phase in ("request", "response", "body", "unmatched_response"):
                    state["phase"] = phase
                status = diagnostic.get("httpStatus")
                if isinstance(status, int) and 100 <= status <= 599:
                    state["http_status"] = status
                try:
                    state["request_id"] = str(uuid.UUID(diagnostic.get("requestId", "")))
                except (ValueError, TypeError, AttributeError):
                    pass
    return post


try:
    if not os.path.isfile(caller):
        state["observed"] = True
    else:
        module = runpy.run_path(caller, run_name="video_cloud_caller")
        client = module.get("Client")
        main = module.get("main")
        if client is not None and callable(getattr(client, "_post", None)) and callable(main):
            client._post = observe_post(client._post)
            state["observed"] = True
        if callable(main):
            code = main(sys.argv[1:])
        else:
            code = 0
except SystemExit as error:
    code = error.code if isinstance(error.code, int) else 1
except Exception:
    code = 1
finally:
    outcome = "unknown"
    if state["observed"] and not state["attempted"]:
        outcome = "not_sent"
    elif state["observed"] and state["rejected"] and not state["unresolved"]:
        outcome = "rejected"
    diagnostic = {"outcome": outcome, "phase": state["phase"], "exit_code": code,
                  "http_status": state["http_status"], "rpc_code": state["rpc_code"],
                  "request_id": state["request_id"]}
    os.write(fd, json.dumps(diagnostic).encode())
    os.close(fd)
sys.exit(code)
