"""Bounded MCP reconnect. Never replay an uncertain mutation or generation request."""
import json
import re
import time

READS = frozenset(("openchatcut_status", "list_projects", "read_project", "read_timeline",
                  "read_script", "read_transcript", "read_captions", "inspect_item",
                  "get_edit_request", "get_my_permissions", "track_progress", "track_export",
                  "generation_status", "generation_list", "target_project"))
# Async generation submits that carry a client idempotencyKey: the server returns the original task for a resent key
# (never a second submission / charge), so a lost response may be resent.
KEYED_GENERATION = frozenset(("video_generate", "video_character_replace", "submit_avatar_video"))
# No project mutation: a lost/HTML response may be resent; the server reuses cached jobs by input hash.
CHECKS = frozenset(("listen_timeline", "watch_timeline", "preview_timeline", "qa_check", "verify_speech",
                   "review_timeline", "compare_with_reference", "inspect_color", "inspect_asset",
                   "browse_assets", "find_transcript", "read_agent_artifact", "browse_library",
                   "search_fonts", "list_templates", "search_templates", "media_probe", "media_frames",
                   "media_contact_sheet", "media_scenes", "media_layout", "media_color_stats",
                   "media_bgm_probe", "media_beats", "media_regions", "media_compare",
                   "media_audio_extract", "media_proxy", "image_understand", "video_understand",
                   "audio_understand", "asset_search", "asset_list", "asset_get", "web_search",
                   "stock_search", "music_search", "sfx_search",
                   "media_beats_batch", "timeline_faces", "check_overlay_faces"))
DELAYS = (1, 2, 4, 8, 10)


# Support both the published caller and the renamed transport handle in newer bundles.
def get_handle(client):
    return getattr(client, "mcp_handle", getattr(client, "sid", None))


def set_handle(client, value):
    setattr(client, "mcp_handle" if hasattr(client, "mcp_handle") else "sid", value)


def persist_handle(client):
    (getattr(client, "_persist_handle", None) or client._save_sid)()


def read_only(method, params):
    if method in ("tools/list", "prompts/list", "prompts/get", "resources/list", "resources/read"):
        return True
    if method != "tools/call":
        return False
    name, args = params.get("name"), params.get("arguments") or {}
    if name == "social_video_fetch":
        return bool(args.get("taskId")) and set(args).issubset({"taskId"})
    # finalize returns alreadyFinalized and from_url dedupes by content, so a 504 that actually finished is safe to resend.
    return name in READS or name in CHECKS or (name == "import_media" and args.get("action") in ("finalize", "from_url")) \
        or (name in KEYED_GENERATION and bool(args.get("idempotencyKey")))


def content(result):
    if not isinstance(result, dict):
        return {}
    if isinstance(result.get("structuredContent"), dict):
        return result["structuredContent"]
    for item in result.get("content") or []:
        if item.get("type") == "text":
            try:
                value = json.loads(item.get("text", ""))
                if isinstance(value, dict):
                    return value
            except ValueError:
                pass
    return {}


def uncertain_result(client, TransportError):
    # Only locally constructed metadata; never include an HTTP body, URL or arbitrary upstream message.
    raw = getattr(client, "transport_diagnostic", {})
    fields = []
    status = raw.get("httpStatus")
    if isinstance(status, int) and 100 <= status <= 599:
        fields.append("HTTP=%s" % status)
    for key, choices in (("phase", ("request", "response", "body", "unmatched_response")),
                         ("contentType", ("application/json", "text/event-stream", "text/html", "text/plain", "other"))):
        if raw.get(key) in choices:
            fields.append("%s=%s" % (key, raw[key]))
    if raw.get("exception") in ("URLError", "TimeoutError", "ConnectionResetError", "RemoteDisconnected", "IncompleteRead", "OSError"):
        fields.append("exception=%s" % raw["exception"])
    rid = raw.get("requestId", "")
    if isinstance(rid, str) and re.fullmatch(r"[a-fA-F0-9]{8}-[a-fA-F0-9]{4}-[a-fA-F0-9]{4}-[a-fA-F0-9]{4}-[a-fA-F0-9]{12}", rid):
        fields.append("requestId=%s" % rid)
    detail = " [%s]" % ", ".join(fields) if fields else ""
    return TransportError("调用结果未知；请先读取原工程/作业状态核对，勿重复执行写入或生成。" + detail)


def rpc(client, method, params, TransportError, RpcError):
    params = params or {}
    safe = read_only(method, params)
    for attempt in range(len(DELAYS) + 1):
        # Initialization/rebinding itself is safe to repeat, even for a following write.
        sent = False
        retry = False
        try:
            if not get_handle(client):
                client.initialize()
            if client._needs_restore and client.scope and method == "tools/call" and params.get("name") not in ("create_project", "target_project"):
                restored = rpc(client, "tools/call", {"name": "target_project", "arguments": {"projectId": client.scope}}, TransportError, RpcError)
                if restored.get("isError"):
                    return restored  # never run against a different/unauthorized project
                client._needs_restore = False
            client.nid += 1
            rid = "%s:%s" % (client.request_prefix, client.nid)
            sent = True
            status, parsed, body = client._post({"jsonrpc": "2.0", "id": rid, "method": method, "params": params})
            diagnostic = getattr(client, "transport_diagnostic", {})
            diagnostic["httpStatus"] = status
            client.transport_diagnostic = diagnostic
            msg = client._pick(parsed, rid)
            if msg is None and status < 400:
                diagnostic["phase"] = "unmatched_response"
            errors = [m.get("error") for m in parsed if isinstance(m, dict) and m.get("id") in (rid, None)]
            errors = [e for e in errors if isinstance(e, dict)]
            # These exact responses are emitted before dispatch, so a write can be retried.
            stale = status == 404 and any(e.get("message") == "MCP session not found or expired" for e in errors)
            draining = status == 503 and any(isinstance(e.get("data"), dict) and
                e["data"].get("code") == "SERVICE_RESTARTING" and e["data"].get("requestStarted") is False for e in errors)
            if status in (401, 403):
                raise RpcError(status, "鉴权失败，请重新通过 shiju 登录。")
            if stale or draining:
                set_handle(client, None)
                persist_handle(client)
                retry = True
            elif status in (502, 503, 504) or msg is None:
                if not safe:
                    raise uncertain_result(client, TransportError)
                retry = status in (502, 503, 504) or status < 400
                if not retry:
                    raise RpcError(status, "MCP 返回无匹配结果的 HTTP %s" % status)
            elif "error" in msg:
                error = msg["error"]
                if not isinstance(error, dict):
                    raise RpcError(status, "MCP 返回错误")
                raise RpcError(error.get("code"), error.get("message"), error.get("data"))
            else:
                result = msg["result"]
                if method == "tools/call" and params.get("name") in ("create_project", "target_project") and not result.get("isError"):
                    data = content(result)
                    project = data.get("projectId") or (data.get("binding") or {}).get("projectId") or params.get("arguments", {}).get("projectId")
                    if project:
                        if client.scope != str(project):
                            handle = get_handle(client)
                            set_handle(client, None)
                            persist_handle(client)  # unbound cache must not point at a now-bound connection
                            set_handle(client, handle)
                        client.scope = str(project)
                        persist_handle(client)
                        client._needs_restore = False
                return result
        except TransportError:
            if sent and not safe:
                raise uncertain_result(client, TransportError) from None
            if not sent:
                set_handle(client, None)
                persist_handle(client)
            retry = True
        if retry and attempt < len(DELAYS):
            time.sleep(DELAYS[attempt])
            continue
        raise TransportError("服务暂不可用，自动重连次数已用完；保留原 projectId/jobId，稍后继续，勿重新建工程。")
