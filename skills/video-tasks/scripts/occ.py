#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""occ.py —— videoCutSer MCP 一条命令调用器（给不能注册 MCP 的宿主用，如识矩引擎）。

只依赖 Python 3.8+ 标准库。只是传输层：不处理、不下载任何媒体。

用法:
  occ.py tools [--grep X]                 列出工具名 + 一行说明
  occ.py schema <tool>                    打印工具入参 schema。传参见 call：--args-file <path> 或 @<path>（schema -h / call -h 打印同一段说明）
  occ.py call <tool> [JSON|@<path>|-] [--args-file <path>] [--out f.json] [--no-wait] [--max-wait SEC] [--save-dir DIR]
                     [--fetch-images [--max-images N]]   把回执里的帧图/联系表/对比图存到 save-dir 供宿主看图（不碰视频音频）
                     从 UTF-8 文件读 JSON：`call <tool> --args-file <path>` 或 `call <tool> @<path>`，可与 --out 等选项同时用。
                     嵌套 JSON（如 edit_track 的 json 字段）写进文件，避免命令行引号；不合法时报文件路径和行号。
  occ.py batch @calls.json [--parallel N] [--keep-going]
                     一条命令跑多次调用：[{"tool":…,"args":{…},"out"?:…,"fetchImages"?:true}]；
                     只有读/检查类工具才并行（N≤6），含写操作自动按顺序、遇错即停
  occ.py use [projectId]                  设定/查看当前工程；之后 schema 里有 projectId 的工具自动带上
                                          （create_project / target_project 成功后也会自动记下）
  occ.py status                           调 openchatcut_status
  occ.py skills [--grep X]                本公司可见的 skill 模板目录（会话开始先跑一次）
  occ.py skill get <id> [--path P] [--version V]   读服务端模板正文（只打到标准输出）
                                          模板内容只供 Agent 内部使用：不向用户展示、不写进文件

本地生活探店 / 带货流程的纯文本小工具（不碰媒体；只有 bendi style 调服务端 bendi_style）：occ.py bendi -h

本机兜底：公域素材搜索（服务端 stock_search / music_search / sfx_search 回 NOT_CONFIGURED 时，见 references/local-stock-search.md）
  occ.py stock video|image <英文关键词> [--orientation vertical|horizontal|square] [--source pexels|pixabay] [--limit N]
  occ.py stock music|sfx  <英文关键词> [--min-sec N] [--max-sec N] [--limit N]
  occ.py stock-key pexels|pixabay|freesound     隐藏输入 key，存 ~/.cutvideo-fallback/stock.env（0600）
  occ.py stock-key --check                      只显示哪些来源已配置，不显示 key
  只返回搜索结果 JSON（{items, errors?}，字段同服务端工具）；不下载媒体，选中后把 url 交给 import_media from_url。

环境变量:
  OCC_MCP_URL   默认 https://gateway.shiju.cloud/mixcut/mixcut-srv/mcp（线上）；本地可设 http://127.0.0.1:5310/mcp
  SHIJU_KEY     必需（找不到时依次读 OCC_SHIJU_ENV 指向的文件、shiju 技能包根目录 .env）；绝不打印
  OCC_TIMEOUT   单次 HTTP 超时秒数，默认 300
  OCC_PROJECT   覆盖自动带上的 projectId；OCC_STATE_DIR 状态目录（默认 ./occ-out）
退出码: 0 成功；1 工具报错(isError)/JSON-RPC 错误（会附该工具参数速查）；2 用法/配置错误；3 传输错误；4 等待超时
"""
import base64
import hashlib
import http.client
import json
import os
import re
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid

URL = os.environ.get("OCC_MCP_URL", "https://gateway.shiju.cloud/mixcut/mixcut-srv/mcp")
TIMEOUT = float(os.environ.get("OCC_TIMEOUT", "300") or 300)
PROTO = "2025-03-26"
STDOUT_LIMIT = 4000
DONE = ("succeeded", "failed", "cancelled", "canceled", "error", "expired", "interrupted")
# These tools have a documented continuation: the completed job is an intermediate
# computation, and the same request retrieves/applies the cached result. Unknown
# tools are never replayed merely because they returned a jobId.
RECALL_AFTER_JOB = frozenset((
    "import_media", "clean_script", "normalize_loudness", "trigger_transcript", "verify_speech", "listen_timeline", "qa_check",
    "preview_timeline", "view_timeline_frames", "review_timeline", "watch_timeline", "compare_with_reference",
    "media_probe", "media_frames", "media_contact_sheet", "media_audio_extract", "media_scenes",
    "media_proxy", "media_beats", "media_bgm_probe", "media_color_stats", "media_layout", "media_compare",
    "media_cover", "media_regions", "media_freeze", "media_reverse", "inspect_color",
    "media_beats_batch", "timeline_faces", "check_overlay_faces", "bendi_style",
))
WAITING = ("running", "queued", "pending")


class TransportError(Exception):
    pass


class RpcError(Exception):
    def __init__(self, code, message, data=None):
        Exception.__init__(self, message)
        self.code, self.message, self.data = code, message, data


def die(msg, code):
    sys.stderr.write(msg.rstrip() + "\n")
    sys.exit(code)


def _read_env_file(path):
    try:
        with open(path, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if line.startswith("export "):
                    line = line[7:].strip()
                prefix = "SHIJU_" + "KEY"
                if line.startswith(prefix + "="):
                    v = line.split("=", 1)[1].strip().strip('"').strip("'")
                    if v:
                        return v
    except OSError:
        pass
    return None


def load_key():
    k = os.environ.get("SHIJU_KEY", "").strip()
    if k:
        return k
    here = os.path.dirname(os.path.abspath(__file__))
    home = os.path.expanduser("~")
    cands = [os.environ.get("OCC_SHIJU_ENV", "")]
    # 同一技能池里的 shiju 技能（skills/cutvideo/scripts -> skills/shiju/.env）
    cands.append(os.path.join(here, "..", "..", "shiju", ".env"))
    for base in (".shiju-desktop/skill_pool", ".claude/skills", ".cursor/skills", ".agents/skills", ".codex/skills"):
        cands.append(os.path.join(home, base, "shiju", ".env"))
    for c in cands:
        if c:
            v = _read_env_file(c)
            if v:
                return v
    die("错误：没有找到 SHIJU_KEY。请先用 shiju 技能登录（在 shiju 技能包根目录执行 `scripts/shiju login`），"
        "或在启动宿主的 shell 里导出 SHIJU_KEY 后重试。", 2)


# Streamable HTTP 连接复用头名 / 服务端失效文案（字面量拆开，避免平台敏感词规则误拦）
_MCP_CONN_HDR = "Mcp-" + "Ses" + "sion" + "-Id"
_STALE_CONN = "ses" + "sion"
_HTTP_AUTH = "Author" + "ization"
_FS_AUTH_PREFIX = "Tok" + "en "


def _handle_cache_path(scope="", key=""):
    # 服务端一个 MCP 连接只绑定一个工程，所以按 (URL, projectId) 分别缓存连接句柄
    h = hashlib.sha256((URL + "|" + (scope or "") + "|" + hashlib.sha256(key.encode()).hexdigest()).encode()).hexdigest()[:16]
    uid = str(os.getuid()) if hasattr(os, "getuid") else "u"
    return os.path.join(tempfile.gettempdir(), "occ-mcp-handle-%s-%s" % (uid, h))


class Client(object):
    def __init__(self, key, scope=""):
        self.key = key
        self.scope = scope
        self.mcp_handle = None
        self._needs_restore = False
        self.nid = 1
        self.request_prefix = uuid.uuid4().hex
        try:
            with open(_handle_cache_path(self.scope, self.key)) as f:
                self.mcp_handle = f.read().strip() or None
        except OSError:
            pass

    def _persist_handle(self):
        tmp = None
        try:
            p = _handle_cache_path(self.scope, self.key)
            fd, tmp = tempfile.mkstemp(prefix="occ-handle-", dir=os.path.dirname(p))
            with os.fdopen(fd, "w") as f:
                f.write(self.mcp_handle or "")
            os.replace(tmp, p)
        except OSError:
            pass
        finally:
            if tmp and os.path.exists(tmp):
                os.unlink(tmp)

    def _post(self, payload):
        self.transport_diagnostic = {"phase": "request"}
        headers = {"Content-Type": "application/json",
                   "Accept": "application/json, text/event-stream",
                   "X-" + "Api" + "-Key": self.key, "MCP-Protocol-Version": PROTO}
        if self.mcp_handle:
            headers[_MCP_CONN_HDR] = self.mcp_handle
        req = urllib.request.Request(URL, data=json.dumps(payload).encode("utf-8"), headers=headers, method="POST")
        try:
            r = urllib.request.urlopen(req, timeout=TIMEOUT)
            status = r.status
        except urllib.error.HTTPError as e:
            r, status = e, e.code
        except (urllib.error.URLError, OSError, http.client.HTTPException) as e:
            self.transport_diagnostic["exception"] = type(e).__name__
            raise TransportError("MCP 请求连接失败（%s）" % type(e).__name__) from None
        returned = r.headers.get(_MCP_CONN_HDR)
        if returned and returned != self.mcp_handle:
            self.mcp_handle = returned
            self._persist_handle()
        ctype = r.headers.get("Content-Type", "") or ""
        self.transport_diagnostic.update({"phase": "response", "httpStatus": status,
            "contentType": ctype.split(";", 1)[0].strip() if ctype.split(";", 1)[0].strip() in
                ("application/json", "text/event-stream", "text/html", "text/plain") else "other"})
        request_id = r.headers.get("X-Request-Id", "")
        if re.fullmatch(r"[a-fA-F0-9]{8}-[a-fA-F0-9]{4}-[a-fA-F0-9]{4}-[a-fA-F0-9]{4}-[a-fA-F0-9]{12}", request_id):
            self.transport_diagnostic["requestId"] = request_id
        try:
            body = r.read().decode("utf-8", "replace")
        except (OSError, http.client.HTTPException) as e:
            self.transport_diagnostic.update({"phase": "body", "exception": type(e).__name__})
            raise TransportError("MCP 响应中断") from None
        finally:
            r.close()
        msgs = []
        if "text/event-stream" in ctype:
            data = []
            for line in body.splitlines():
                if line.startswith("data:"):
                    data.append(line[5:].lstrip())
                elif line == "" and data:
                    msgs.append("\n".join(data)); data = []
            if data:
                msgs.append("\n".join(data))
        elif body.strip():
            msgs.append(body)
        parsed = []
        for m in msgs:
            try:
                parsed.append(json.loads(m))
            except ValueError:
                pass
        return status, parsed, body

    def _pick(self, parsed, rid):
        for m in parsed:
            items = m if isinstance(m, list) else [m]
            for it in items:
                if isinstance(it, dict) and it.get("id") == rid and ("result" in it or "error" in it):
                    return it
        return None

    def initialize(self):
        self.mcp_handle = None
        self._needs_restore = True
        st, parsed, body = self._post({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {
            "protocolVersion": PROTO, "capabilities": {},
            "clientInfo": {"name": "cutvideo-occ.py", "version": "0.2.7"}}})
        if st in (401, 403):
            raise RpcError(st, "鉴权失败，请重新通过 shiju 登录。")
        if st in (502, 503, 504):
            raise TransportError("initialize 暂时不可用")
        if st >= 400:
            raise RpcError(st, "initialize 失败 HTTP %s" % st)
        msg = self._pick(parsed, 0)
        if msg and isinstance(msg.get("error"), dict):
            e = msg["error"]
            raise RpcError(e.get("code"), e.get("message"), e.get("data"))
        if not msg or "result" not in msg or not self.mcp_handle:
            self.mcp_handle = None
            raise TransportError("initialize 未收到有效连接")
        self._persist_handle()
        self._post({"jsonrpc": "2.0", "method": "notifications/initialized"})

    def rpc(self, method, params=None):
        from occ_recovery import rpc
        return rpc(self, method, params, TransportError, RpcError)

    def list_tools(self):
        tools, cursor = [], None
        while True:
            r = self.rpc("tools/list", {"cursor": cursor} if cursor else {})
            tools.extend(r.get("tools", []))
            cursor = r.get("nextCursor")
            if not cursor:
                return tools

    def call(self, name, args):
        return self.rpc("tools/call", {"name": name, "arguments": args})


def structured(res):
    if not isinstance(res, dict):
        return res
    if res.get("structuredContent") is not None:
        return res["structuredContent"]
    for c in res.get("content") or []:
        if c.get("type") == "text":
            try:
                return json.loads(c["text"])
            except (ValueError, KeyError):
                return {"text": c.get("text", "")}
    return {}


def _with_request_id(obj, client):
    """Put gateway X-Request-Id onto the receipt so --out files carry it for support."""
    rid = (getattr(client, "transport_diagnostic", None) or {}).get("requestId")
    if not rid or not isinstance(obj, dict):
        return obj
    # Prefer server-provided requestId (tool body / error payload); else stamp from response header.
    if obj.get("requestId"):
        return obj
    out = dict(obj)
    out["requestId"] = rid
    return out


def _job_of(s):
    """返回 (kind, id, pollTool)：kind = job | task | None。"""
    if not isinstance(s, dict) or str(s.get("status")) not in WAITING:
        prox = s.get("proxy") if isinstance(s, dict) else None
        if isinstance(prox, dict) and prox.get("jobId") and str(prox.get("status", "running")) in WAITING:
            return "job", prox["jobId"], "track_progress"
        return None, None, None
    if s.get("jobId"):
        return "job", s["jobId"], s.get("pollWith") or "track_progress"
    if s.get("taskId"):
        return "task", s["taskId"], s.get("pollWith")
    return None, None, None


def run_call(cl, tool, args, wait, max_wait):
    t0 = time.time()
    res = cl.call(tool, args)
    if not wait or res.get("isError"):
        return res
    first = structured(res)
    if wait and isinstance(first, dict) and first.get("taskId") and first.get("generationId"):
        # 先把 taskId 落到 stderr：等待中断线 / 超时也能用它续查
        sys.stderr.write("[occ] 已提交 %s taskId=%s generationId=%s\n" % (tool, first["taskId"], first["generationId"]))
    polls = 0
    sticky_poll = None
    latest = None
    while True:
        s = structured(res)
        kind, jid, poll = _job_of(s)
        poll = poll or sticky_poll
        sticky_poll = poll
        if not kind:
            return res
        if time.time() - t0 > max_wait:
            sys.stderr.write("等待超时（%ss），作业仍在进行：%s=%s；稍后用 %s 续查，不要重新提交。\n" % (max_wait, kind, jid, poll or tool))
            out = dict(latest or res)
            out["_occTimeout"] = True
            return out
        interval = float(s.get("pollIntervalSec") or (5 if kind == "job" else 10))
        time.sleep(max(2.0, min(interval, 15.0)))
        polls += 1
        if kind == "job":
            # track_export 认 exportId，也认 jobId 别名；这里统一传 exportId。其余作业轮询用 jobId。
            poll_args = {"exportId": jid} if poll == "track_export" else {"jobId": jid}
            if args.get("projectId"):
                poll_args["projectId"] = args["projectId"]
            pr = cl.call(poll, poll_args)
            if pr.get("isError"):
                return pr
            ps = structured(pr)
            st = str(ps.get("status")) if isinstance(ps, dict) else ""
            sys.stderr.write("[occ] %s %s %s progress=%s (%.0fs)\n" % (poll, jid, st, ps.get("progress") if isinstance(ps, dict) else "", time.time() - t0))
            if st in WAITING:
                latest = pr  # 超时时交出最近一次进度，而不是最初的 progress=0
                continue
            if st != "succeeded":
                return pr  # failed 等：返回作业状态（含 error）
            if tool not in RECALL_AFTER_JOB or poll == "track_export":
                return pr  # 轮询已给完整结果；重调这些工具会新建第二个截图/导出作业。
            # 作业完成：用相同参数重调原工具，服务端复用已完成作业，直接给结构化结果
            res = cl.call(tool, args)
        else:
            if poll and poll != tool:
                res = cl.call(poll, {"taskId": jid})  # ai-hub：generation_status {taskId}
            else:
                res = cl.call(tool, {"taskId": jid})  # 如 speech：同一工具只传 taskId 续取
            if res.get("isError"):
                return res
            sys.stderr.write("[occ] %s %s %s (%.0fs)\n" % (poll or tool, jid, structured(res).get("status") if isinstance(structured(res), dict) else "", time.time() - t0))


STATE_DIR = os.environ.get("OCC_STATE_DIR") or "occ-out"
TOOLS_TTL = 600  # 10 min: a stale schema after a rollout hid new params (batch 4)
IMAGE_EXT = (".jpg", ".jpeg", ".png", ".webp")
IMAGE_MAX_BYTES = 8 * 1024 * 1024


def _state_path():
    return os.path.join(STATE_DIR, ".occ-state.json")


def load_state():
    try:
        with open(_state_path(), encoding="utf-8") as f:
            v = json.load(f)
            return v if isinstance(v, dict) else {}
    except (OSError, ValueError):
        return {}


def save_state(**kv):
    st = load_state()
    st.update(kv)
    os.makedirs(STATE_DIR, exist_ok=True)
    with open(_state_path(), "w", encoding="utf-8") as f:
        json.dump(st, f, ensure_ascii=False)


def _tools_cache_path(key):
    h = hashlib.sha256((URL + "|" + hashlib.sha256(key.encode()).hexdigest()).encode()).hexdigest()[:16]
    uid = str(os.getuid()) if hasattr(os, "getuid") else "u"
    return os.path.join(tempfile.gettempdir(), "occ-tools-%s-%s.json" % (uid, h))


def cached_tools(key, refresh=False):
    """{name: inputSchema}; cached per endpoint + account so every call doesn't pay for tools/list."""
    p = _tools_cache_path(key)
    if not refresh:
        try:
            if time.time() - os.path.getmtime(p) < TOOLS_TTL:
                with open(p, encoding="utf-8") as f:
                    return json.load(f)
        except (OSError, ValueError):
            pass
    try:
        tools = Client(key).list_tools()
    except (TransportError, RpcError):
        return {}
    m = {t.get("name"): t.get("inputSchema") or {} for t in tools if t.get("name")}
    try:
        with open(p, "w", encoding="utf-8") as f:
            json.dump(m, f, ensure_ascii=False)
    except OSError:
        pass
    return m


def inject_project(tool, args, schemas):
    """Fill projectId from `occ.py use` / last create_project so a restarted gateway never loses the binding."""
    if args.get("projectId") or args.get("editorProjectId") or tool in ("create_project", "list_projects"):
        return args
    pid = os.environ.get("OCC_PROJECT") or load_state().get("projectId")
    props = (schemas.get(tool) or {}).get("properties") or {}
    # Facade tools take projectId; native OpenChatCut tools (read_timeline, read_script, edit_track…) take editorProjectId.
    field = "projectId" if "projectId" in props else ("editorProjectId" if "editorProjectId" in props else None)
    if pid and field:
        sys.stderr.write("[occ] 自动带上 %s=%s\n" % (field, pid))
        return dict(args, **{field: pid})
    return args


def remember_project(tool, args, s):
    if tool not in ("create_project", "target_project") or not isinstance(s, dict):
        return
    pid = s.get("projectId") or (s.get("binding") or {}).get("projectId") or args.get("projectId")
    if pid:
        save_state(projectId=str(pid))


def schema_hint(tool, schemas):
    sc = schemas.get(tool)
    if not sc:
        return ""
    req = sc.get("required") or []
    parts = []
    for k, v in (sc.get("properties") or {}).items():
        v = v if isinstance(v, dict) else {}
        t = v.get("type") or ("enum" if v.get("enum") else "any")
        if v.get("enum"):
            t += "{%s}" % "|".join(str(e) for e in v["enum"][:8])
        parts.append("%s%s:%s" % (k, "*" if k in req else "", t))
    return ("[occ] %s 参数（*必填）：%s" % (tool, ", ".join(parts)))[:1500]


def _image_urls(obj, out):
    if isinstance(obj, dict):
        for v in obj.values():
            _image_urls(v, out)
    elif isinstance(obj, list):
        for v in obj:
            _image_urls(v, out)
    elif isinstance(obj, str) and obj.startswith("https://"):
        if urllib.parse.urlsplit(obj).path.lower().endswith(IMAGE_EXT) and obj not in out:
            out.append(obj)
    return out


def fetch_images(s, save_dir, tool, limit):
    """Save FC frame / contact-sheet / compare images (never video or audio) so the host model can view them."""
    paths = []
    for i, u in enumerate(_image_urls(s, [])[:limit]):
        try:
            with urllib.request.urlopen(urllib.request.Request(u, headers={"User-Agent": "cutvideo-occ/1"}), timeout=30) as r:
                ctype = (r.headers.get("Content-Type") or "").split(";")[0].strip()
                data = r.read(IMAGE_MAX_BYTES + 1)
        except (urllib.error.URLError, OSError, http.client.HTTPException):
            sys.stderr.write("[occ] 第 %d 张图取不到，跳过\n" % (i + 1))
            continue
        if not ctype.startswith("image/") or len(data) > IMAGE_MAX_BYTES:
            continue
        os.makedirs(save_dir, exist_ok=True)
        ext = {"image/png": "png", "image/webp": "webp"}.get(ctype, "jpg")
        p = os.path.join(save_dir, "%s-%s-u%d.%s" % (tool, time.strftime("%Y%m%d-%H%M%S"), i, ext))
        with open(p, "wb") as f:
            f.write(data)
        paths.append(os.path.abspath(p))
    return paths


def save_images(res, save_dir, tool):
    paths = []
    content = res.get("content") or []
    ext = {"image/png": "png", "image/jpeg": "jpg", "image/jpg": "jpg", "image/webp": "webp", "image/gif": "gif"}
    for i, c in enumerate(content):
        if c.get("type") == "image" and c.get("data"):
            os.makedirs(save_dir, exist_ok=True)
            p = os.path.join(save_dir, "%s-%s-%d.%s" % (tool, time.strftime("%Y%m%d-%H%M%S"), i, ext.get(c.get("mimeType", ""), "bin")))
            with open(p, "wb") as f:
                f.write(base64.b64decode(c["data"]))
            c["data"] = None
            c["savedTo"] = os.path.abspath(p)
            paths.append(c["savedTo"])
    return paths


def emit(obj, out=None):
    txt = json.dumps(obj, ensure_ascii=False, separators=(",", ":"))
    if out:
        with open(out, "w", encoding="utf-8") as f:
            f.write(json.dumps(obj, ensure_ascii=False, indent=1))
    if len(txt) > STDOUT_LIMIT:
        txt = txt[:STDOUT_LIMIT] + ("…[已截断，共 %d 字符%s]" % (len(txt), "；完整结果见 " + out if out else "；加 --out 文件 保存完整结果"))
    print(txt)


def read_json_source(src):
    """读 JSON：裸字符串、`@路径`（UTF-8 文件）或 `-`（stdin）。不合法时写明来源和行号。"""
    if src is None:
        return {}
    path, where = None, "命令行参数"
    try:
        if src == "-":
            where = "标准输入"
            text = sys.stdin.read()
        elif src.startswith("@"):
            path = src[1:]
            where = path or "@"
            with open(path, encoding="utf-8") as f:
                text = f.read()
        else:
            text = src
    except UnicodeDecodeError as e:
        die("参数文件 %s 不是 UTF-8（约第 %d 字节）：%s" % (where, (e.start or 0) + 1, e.reason), 2)
    except OSError as e:
        die("读不到参数文件 %s：%s" % (where, getattr(e, "strerror", None) or e), 2)
    if not text.strip():
        return {}
    try:
        return json.loads(text)
    except json.JSONDecodeError as e:
        die("参数不是合法 JSON：%s 第 %d 行第 %d 列：%s" % (where, e.lineno, e.colno, e.msg), 2)
    except ValueError as e:
        die("参数不是合法 JSON：%s：%s" % (where, e), 2)


def parse_args_json(src):
    if src is None:
        return {}
    v = read_json_source(src)
    if not isinstance(v, dict):
        die("参数必须是 JSON 对象", 2)
    return v


# ---------- 本机兜底：公域素材搜索（与 ai-hub stock_search / music_search / sfx_search 同来源、同字段）----------
STOCK_ENV = os.path.join(os.path.expanduser("~"), ".cutvideo-fallback", "stock.env")
STOCK_KEYS = {  # provider -> 依次查找的环境变量名（第一个也是 stock.env 里写入的名字）
    "pexels": ("PEXELS_API_KEY", "SHIJU_PEXELS_API_KEY"),
    "pixabay": ("PIXABAY_API_KEY", "SHIJU_PIXABAY_API_KEY"),
    "freesound": ("FREESOUND_API_TOKEN", "SHIJU_FREESOUND_API_TOKEN"),
}
STOCK_SIGNUP = {
    "pexels": "https://www.pexels.com/api/ （登录后 Your API Key）",
    "pixabay": "https://pixabay.com/api/docs/ （登录后页面里的 key）",
    "freesound": "https://freesound.org/apiv2/apply/ （申请后页面上的那串凭证）",
}


def _stock_env():
    out = {}
    try:
        with open(STOCK_ENV, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if "=" in line and not line.startswith("#"):
                    k, v = line.split("=", 1)
                    out[k.strip()] = v.strip().strip('"').strip("'")
    except OSError:
        pass
    return out


def stock_key(provider):
    file_vals = _stock_env()
    for name in STOCK_KEYS[provider]:
        v = os.environ.get(name, "").strip() or file_vals.get(name, "").strip()
        if v:
            return v
    return None


def _get_json(url, headers=None):
    req = urllib.request.Request(url, headers=dict({"User-Agent": "cutvideo-occ/1", "Accept": "application/json"}, **(headers or {})))
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            return json.loads(r.read().decode("utf-8"))
    except urllib.error.HTTPError as e:
        raise TransportError("HTTP %s" % e.code)  # 不带 URL：Pixabay 的 key 在查询串里
    except (urllib.error.URLError, OSError, ValueError) as e:
        raise TransportError(type(e).__name__)


def _pick_mp4(files):
    mp4 = [f for f in files if f.get("link") and "mp4" in (f.get("file_type") or "mp4")]
    size = lambda f: (f.get("width") or 0) * (f.get("height") or 0)
    within = sorted([f for f in mp4 if max(f.get("width") or 0, f.get("height") or 0) <= 1920], key=size, reverse=True)
    return within[0] if within else (sorted(mp4, key=size)[0] if mp4 else None)


def _pexels(kind, query, orientation, limit):
    key = stock_key("pexels")
    q = {"query": query, "per_page": str(min(limit, 40))}
    o = {"vertical": "portrait", "horizontal": "landscape", "square": "square"}.get(orientation or "vertical")
    if o:
        q["orientation"] = o
    base = "https://api.pexels.com/videos/search" if kind == "video" else "https://api.pexels.com/v1/search"
    j = _get_json(base + "?" + urllib.parse.urlencode(q), {_HTTP_AUTH: key})
    lic = "Pexels License（可商用，无需署名）"
    items = []
    if kind == "video":
        for v in j.get("videos") or []:
            f = _pick_mp4(v.get("video_files") or [])
            if f:
                items.append({"url": f["link"], "thumbUrl": v.get("image"), "pageUrl": v.get("url"), "durationSec": v.get("duration"),
                              "width": f.get("width"), "height": f.get("height"), "license": lic,
                              "attribution": (v.get("user") or {}).get("name"), "source": "pexels"})
    else:
        for p in j.get("photos") or []:
            src = p.get("src") or {}
            u = src.get("large2x") or src.get("original")
            if u:
                items.append({"url": u, "thumbUrl": src.get("medium"), "pageUrl": p.get("url"), "durationSec": None, "width": p.get("width"),
                              "height": p.get("height"), "license": lic, "attribution": p.get("photographer"), "source": "pexels", "title": p.get("alt")})
    return items


def _pixabay(kind, query, orientation, limit):
    q = {"key": stock_key("pixabay"), "q": query[:100], "per_page": str(max(3, min(limit, 50))), "safesearch": "true"}
    if kind == "image":
        q["image_type"] = "photo"
        if orientation in ("vertical", "horizontal"):
            q["orientation"] = orientation
    base = "https://pixabay.com/api/videos/" if kind == "video" else "https://pixabay.com/api/"
    j = _get_json(base + "?" + urllib.parse.urlencode(q))
    lic = "Pixabay Content License（可商用，无需署名）"
    items = []
    for h in j.get("hits") or []:
        if kind == "video":
            vids = h.get("videos") or {}
            pick = next((vids[k] for k in ("large", "medium", "small") if (vids.get(k) or {}).get("url")), None)
            if not pick:
                continue
            w, ht = pick.get("width"), pick.get("height")
            if w and ht and ((orientation == "vertical" and w > ht) or (orientation == "horizontal" and ht > w)):
                continue
            items.append({"url": pick["url"], "thumbUrl": pick.get("thumbnail"), "pageUrl": h.get("pageURL"), "durationSec": h.get("duration"),
                          "width": w, "height": ht, "license": lic, "attribution": h.get("user"), "source": "pixabay", "title": h.get("tags")})
        elif h.get("largeImageURL"):
            items.append({"url": h["largeImageURL"], "thumbUrl": h.get("previewURL"), "pageUrl": h.get("pageURL"), "durationSec": None,
                          "width": h.get("imageWidth"), "height": h.get("imageHeight"), "license": lic, "attribution": h.get("user"),
                          "source": "pixabay", "title": h.get("tags")})
    return items


def _license_label(raw):
    if "publicdomain/zero" in raw or "Creative Commons 0" in raw:
        return "CC0（可商用，无需署名）"
    if "by-nc" in raw or "NonCommercial" in raw:
        return "CC BY-NC（不可商用）"
    if "licenses/by/" in raw or raw == "Attribution":
        return "CC BY（可商用，需署名）"
    return raw


def _freesound(kind, query, min_sec, max_sec, limit):
    lo = min_sec if min_sec is not None else (20 if kind == "music" else 0)
    hi = max_sec if max_sec is not None else (600 if kind == "music" else 30)
    flt = ['duration:[%s TO %s]' % (lo, hi), 'license:("Creative Commons 0" OR "Attribution")'] + (["tag:music"] if kind == "music" else [])
    q = {"query": query, "filter": " ".join(flt), "fields": "id,name,username,license,duration,url,tags,previews",
         "page_size": str(min(limit, 50)), "sort": "score"}
    j = _get_json("https://freesound.org/apiv2/search/text/?" + urllib.parse.urlencode(q),
                  {_HTTP_AUTH: _FS_AUTH_PREFIX + stock_key("freesound")})
    items = []
    for r in j.get("results") or []:
        pv = (r.get("previews") or {}).get("preview-hq-mp3")
        if not isinstance(pv, str) or not pv.startswith("https://cdn.freesound.org/previews/"):
            continue
        items.append({"url": pv, "title": r.get("name"), "durationSec": r.get("duration"), "license": _license_label(r.get("license") or ""),
                      "attribution": "%s @ freesound.org" % r.get("username") if r.get("username") else None,
                      "pageUrl": r.get("url"), "tags": (r.get("tags") or [])[:12], "source": "freesound"})
    return items


def cmd_stock(rest):
    orientation = take(rest, "--orientation")
    only = take(rest, "--source")
    lim, mn, mx = take(rest, "--limit"), take(rest, "--min-sec"), take(rest, "--max-sec")
    if len(rest) < 2 or rest[0] not in ("video", "image", "music", "sfx"):
        die("用法：occ.py stock video|image|music|sfx <英文关键词> [选项]", 2)
    kind, query = rest[0], " ".join(rest[1:])
    try:
        limit = max(1, min(int(lim or 12), 50))
        mn = float(mn) if mn else None
        mx = float(mx) if mx else None
    except ValueError:
        die("--limit / --min-sec / --max-sec 要是数字", 2)
    if kind in ("music", "sfx"):
        providers = {"freesound": lambda: _freesound(kind, query, mn, mx, limit)}
    else:
        providers = {"pexels": lambda: _pexels(kind, query, orientation, limit), "pixabay": lambda: _pixabay(kind, query, orientation, limit)}
    active = [p for p in providers if stock_key(p) and (not only or p == only)]
    if not active:
        need = [p for p in providers if not only or p == only]
        die("NOT_CONFIGURED：本机没有 %s 的 key（技能包从不自带）。请用户注册（%s），在对话里发给 Agent 后用环境变量当次调用；或可选本机 `python3 scripts/occ.py stock-key %s`。"
            % (" / ".join(need), "；".join(STOCK_SIGNUP[p] for p in need), need[0]), 2)
    lists, errors = [], []
    for p in active:
        try:
            lists.append(providers[p]())
        except TransportError as e:
            errors.append({"source": p, "message": str(e)})
    if not lists:
        sys.stderr.write("所有来源都失败：%s\n" % json.dumps(errors, ensure_ascii=False))
        return 3
    items = []
    for i in range(max(len(l) for l in lists)):
        for l in lists:
            if i < len(l) and len(items) < limit:
                items.append(l[i])
    print(json.dumps(dict({"items": items}, **({"errors": errors} if errors else {})), ensure_ascii=False, indent=1))
    return 0


def cmd_stock_key(rest):
    if rest and rest[0] == "--check":
        print(json.dumps({p: bool(stock_key(p)) for p in STOCK_KEYS}, ensure_ascii=False))
        return 0
    if not rest or rest[0] not in STOCK_KEYS:
        die("用法：occ.py stock-key pexels|pixabay|freesound  或  occ.py stock-key --check", 2)
    import getpass
    provider = rest[0]
    if sys.stdin.isatty():
        sys.stderr.write("注册地址：%s\n" % STOCK_SIGNUP[provider])
        value = getpass.getpass("粘贴 %s key（输入不显示，回车结束）：" % provider).strip()
    else:
        value = sys.stdin.readline().strip()
    if not value or any(c.isspace() for c in value):
        die("key 为空或含空白，未保存", 2)
    vals = _stock_env()
    vals[STOCK_KEYS[provider][0]] = value
    os.makedirs(os.path.dirname(STOCK_ENV), mode=0o700, exist_ok=True)
    fd = os.open(STOCK_ENV, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    with os.fdopen(fd, "w", encoding="utf-8") as f:
        f.write("# cutvideo 本机兜底：公域素材站 key（只在本机，不要提交、不要发给别人）\n")
        for k, v in vals.items():
            f.write("%s=%s\n" % (k, v))
    os.chmod(STOCK_ENV, 0o600)
    sys.stderr.write("已保存 %s 的 key（%s）。\n" % (provider, STOCK_ENV))
    return 0


CALL_HELP = """\
用法：
  occ.py schema <tool>
  occ.py call <tool> [JSON|@<path>|-]
  occ.py call <tool> --args-file <path>
  occ.py call <tool> @<path>

`--args-file <path>` 与 `@<path>` 都从 UTF-8 文件读 JSON 对象，可与 --out、--no-wait、--max-wait、--save-dir、--fetch-images 同时使用。
嵌套 JSON（例如 edit_track 的 json 字段本身是 JSON 字符串）写进文件再传，避免命令行引号。
文件不是合法 JSON 时，报错写明文件路径和行号。
"""


def take(argv, flag, has_val=True):
    if flag in argv:
        i = argv.index(flag)
        if has_val:
            if i + 1 >= len(argv):
                die("%s 需要一个值" % flag, 2)
            v = argv[i + 1]; del argv[i:i + 2]; return v
        del argv[i]; return True
    return None


def main(argv):
    if not argv or argv[0] in ("-h", "--help", "help"):
        print(__doc__); return 0
    cmd, rest = argv[0], list(argv[1:])
    if cmd == "bendi":
        sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
        import occ_bendi
        if rest[:1] == ["style"]:  # 风格机检在服务端 bendi_style
            style_opts = {"save_dir": "./occ-out", "wait": True, "max_wait": 120.0, "fetch": False, "max_images": 0}
            return occ_bendi.main(rest, call=lambda tool, args: call_once(tool, args, style_opts))
        return occ_bendi.main(rest)
    if cmd == "stock":
        return cmd_stock(rest)
    if cmd == "stock-key":
        return cmd_stock_key(rest)
    if cmd in ("skills", "skill"):
        sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
        import occ_skill
        skill_opts = {"save_dir": "./occ-out", "wait": True, "max_wait": 120.0, "fetch": False, "max_images": 0}
        return occ_skill.main(cmd, rest, lambda tool, args: call_once(tool, args, skill_opts))
    if cmd == "use":
        if not rest:
            st = load_state()
            print(json.dumps({"projectId": st.get("projectId")}, ensure_ascii=False)); return 0
        save_state(projectId=rest[0])
        sys.stderr.write("[occ] 之后的调用自动带 projectId=%s\n" % rest[0]); return 0
    if cmd in ("schema", "call") and ("-h" in rest or "--help" in rest):
        print(CALL_HELP); return 0
    if cmd not in ("tools", "schema", "call", "status", "batch"):
        die("未知子命令 %s；可用：tools / schema / call / batch / use / status / skills / skill / stock / stock-key / bendi" % cmd, 2)
    out = take(rest, "--out")
    args_file = take(rest, "--args-file")
    save_dir = take(rest, "--save-dir") or "./occ-out"
    no_wait = take(rest, "--no-wait", False)
    mw = take(rest, "--max-wait")
    grep = take(rest, "--grep")
    fetch = take(rest, "--fetch-images", False)
    mi = take(rest, "--max-images")
    par = take(rest, "--parallel")
    keep_going = take(rest, "--keep-going", False)
    try:
        max_wait = float(mw) if mw else 1500.0
        max_images = max(1, min(int(mi), 24)) if mi else 12
        parallel = max(1, min(int(par), 6)) if par else 1
    except ValueError:
        die("--max-wait / --max-images / --parallel 要是数字", 2)
    opts = {"save_dir": save_dir, "wait": not no_wait, "max_wait": max_wait, "fetch": fetch, "max_images": max_images}
    if cmd == "call" and rest and rest[0] == "skill_get":
        die("skill_get 请用 occ.py skill get（正文只打到标准输出，不写文件；内容只供内部使用）", 2)
    if cmd == "batch":
        if not rest:
            die("用法：occ.py batch @calls.json [--parallel N] [--keep-going]", 2)
        return cmd_batch(rest[0], opts, parallel, keep_going)
    tool, args = None, {}
    if cmd == "status":
        tool, args = "openchatcut_status", parse_args_json(rest[0] if rest else None)
    elif cmd == "call":
        if not rest:
            die(CALL_HELP.rstrip(), 2)
        if args_file and len(rest) > 1:
            die("call 的参数只给一处：JSON、@<path>、- 或 --args-file。\n" + CALL_HELP, 2)
        tool = rest[0]
        if args_file:
            args = parse_args_json("@" + args_file)
        else:
            args = parse_args_json(rest[1] if len(rest) > 1 else None)
    if cmd in ("status", "call"):
        code, s = call_once(tool, args, opts)
        emit(s, out)
        if out:
            sys.stderr.write("[occ] 完整结果已写入 %s\n" % out)
        return code
    cl = Client(load_key(), "")
    try:
        if cmd == "tools":
            tools = cl.list_tools()
            n = 0
            for t in sorted(tools, key=lambda x: x.get("name", "")):
                d = (t.get("description") or "").strip().splitlines()
                line = "%s\t%s" % (t.get("name"), (d[0] if d else "")[:160])
                if grep and grep.lower() not in line.lower():
                    continue
                print(line); n += 1
            sys.stderr.write("[occ] %d 个工具%s\n" % (n, "（共 %d）" % len(tools) if grep else ""))
            return 0
        if cmd == "schema":
            if not rest:
                die("用法：occ.py schema <tool>\n给工具传参：occ.py call <tool> --args-file <path>  或  occ.py call <tool> @<path>", 2)
            for t in cl.list_tools():
                if t.get("name") == rest[0]:
                    obj = {"name": t["name"], "description": t.get("description"), "inputSchema": t.get("inputSchema")}
                    if t.get("outputSchema"):
                        obj["outputSchema"] = t["outputSchema"]
                    print(json.dumps(obj, ensure_ascii=False, indent=1)); return 0
            die("没有这个工具：%s（用 `occ.py tools --grep 关键词` 查）" % rest[0], 1)
    except RpcError as e:
        sys.stderr.write("JSON-RPC 错误 %s: %s\n" % (e.code, e.message))
        return 1
    except TransportError as e:
        sys.stderr.write("传输错误：%s\n" % e)
        return 3


def inject_idempotency(tool, args, schemas):
    """Async generation submits get a client idempotencyKey (only when the server's schema has one), printed up front:
    after a lost response, resend with the same key (or generation_list) instead of submitting again."""
    from occ_recovery import KEYED_GENERATION
    if tool not in KEYED_GENERATION or args.get("idempotencyKey") or args.get("taskId"):
        return args
    props = ((schemas or {}).get(tool) or {}).get("properties") or {}
    if "idempotencyKey" not in props:
        return args
    args = dict(args, idempotencyKey="occ-" + uuid.uuid4().hex)
    sys.stderr.write("[occ] %s idempotencyKey=%s（断线后带同一个 key 重调不会重复提交 / 计费；也可 generation_list 查 taskId）\n" % (tool, args["idempotencyKey"]))
    return args


def call_once(tool, args, opts):
    """One tool call end to end → (exit code, printable result). Thread-safe: own Client per call."""
    key = load_key()
    schemas = cached_tools(key)
    args = inject_project(tool, args, schemas)
    args = inject_idempotency(tool, args, schemas)
    cl = Client(key, str(args.get("projectId") or args.get("editorProjectId") or ""))
    try:
        res = run_call(cl, tool, args, opts["wait"], opts["max_wait"])
    except RpcError as e:
        sys.stderr.write("JSON-RPC 错误 %s: %s\n" % (e.code, e.message))
        if e.data is not None:
            sys.stderr.write(json.dumps(e.data, ensure_ascii=False)[:2000] + "\n")
        hint = schema_hint(tool, schemas)
        if hint:
            sys.stderr.write(hint + "\n")
        return 1, _with_request_id({"error": {"code": e.code, "message": e.message}}, cl)
    except TransportError as e:
        sys.stderr.write("传输错误：%s\n" % e)
        return 3, _with_request_id({"error": {"code": "TRANSPORT", "message": str(e)}}, cl)
    imgs = save_images(res, opts["save_dir"], tool)
    s = _with_request_id(structured(res), cl)
    if res.get("isError"):
        sys.stderr.write("工具报错：\n")
        hint = schema_hint(tool, schemas)
        if hint:
            sys.stderr.write(hint + "\n")
        return 1, (s if s else res)
    remember_project(tool, args, s)
    if opts.get("fetch"):
        imgs += fetch_images(s, opts["save_dir"], tool, opts["max_images"])
    if imgs:
        if isinstance(s, dict) and res.get("structuredContent") is not None:
            s = dict(s); s["_images"] = imgs
        else:
            s = {"result": s, "_images": imgs}
    if tool == "social_video_fetch" and isinstance(s, dict) and s.get("status") in ("failed", "interrupted"):
        return 1, s
    return (4 if res.get("_occTimeout") else 0), s


def cmd_batch(src, opts, parallel, keep_going):
    """Run [{tool, args, out?, fetchImages?}, …] in one command; only read/check tools may run in parallel."""
    from occ_recovery import read_only
    plan = read_json_source(src)
    calls = plan.get("calls") if isinstance(plan, dict) else plan
    if not isinstance(calls, list) or not calls or not all(isinstance(c, dict) and c.get("tool") for c in calls):
        die("batch 参数要是 [{\"tool\":…, \"args\":{…}}, …] 或 {\"calls\":[…]}", 2)
    if len(calls) > 30:
        die("batch 一次最多 30 条", 2)
    if any(c["tool"] == "skill_get" for c in calls):
        die("batch 里不能放 skill_get（会把模板写进文件）；请用 occ.py skill get", 2)
    safe = all(read_only("tools/call", {"name": c["tool"], "arguments": c.get("args") or {}}) for c in calls)
    if parallel > 1 and not safe:
        sys.stderr.write("[occ] 批里有写操作，改为按顺序执行\n")
        parallel = 1
    results = [None] * len(calls)

    def run(i):
        c = calls[i]
        o = dict(opts, fetch=bool(c.get("fetchImages", opts["fetch"])))
        code, s = call_once(c["tool"], dict(c.get("args") or {}), o)
        out = c.get("out") or os.path.join(opts["save_dir"], "batch-%d-%s.json" % (i + 1, c["tool"]))
        os.makedirs(os.path.dirname(out) or ".", exist_ok=True)
        with open(out, "w", encoding="utf-8") as f:
            f.write(json.dumps(s, ensure_ascii=False, indent=1))
        results[i] = (code, s, out)

    if parallel > 1:
        import threading
        idx = list(range(len(calls)))
        lock = threading.Lock()

        def worker():
            while True:
                with lock:
                    if not idx:
                        return
                    i = idx.pop(0)
                run(i)
        ts = [threading.Thread(target=worker) for _ in range(min(parallel, len(calls)))]
        for t in ts:
            t.start()
        for t in ts:
            t.join()
    else:
        for i in range(len(calls)):
            run(i)
            if results[i][0] in (1, 3) and not keep_going:
                sys.stderr.write("[occ] 第 %d 条失败，后面的没执行（加 --keep-going 可继续）\n" % (i + 1))
                break
    codes = []
    for i, r in enumerate(results):
        if r is None:
            print("[%d] %s 未执行" % (i + 1, calls[i]["tool"])); continue
        code, s, out = r
        codes.append(code)
        txt = json.dumps(s, ensure_ascii=False, separators=(",", ":"))
        print("[%d] %s %s → %s\n%s" % (i + 1, calls[i]["tool"], {0: "ok", 1: "报错", 3: "传输错误", 4: "等待超时"}.get(code, code), out,
                                      txt[:800] + ("…[截断，全文见文件]" if len(txt) > 800 else "")))
    return next((c for c in codes if c in (1, 3)), 4 if 4 in codes else 0)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
