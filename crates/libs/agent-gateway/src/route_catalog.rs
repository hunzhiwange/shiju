//! Documented gateway routes and startup summaries.
#[derive(Debug, Clone, Copy)]
pub struct GatewayRouteSummaryOptions {
    pub whatsapp_enabled: bool,
    pub linq_enabled: bool,
    pub wati_enabled: bool,
    pub nextcloud_talk_enabled: bool,
    pub qq_webhook_enabled: bool,
    pub node_control_enabled: bool,
}

pub fn gateway_route_summary(options: GatewayRouteSummaryOptions) -> Vec<String> {
    let routes = gateway_route_entries(options);
    let mut lines = vec![format!("  Gateway routes: {} total", routes.len())];
    lines.push("  Public probes and third-party webhooks (no tenant headers):".to_string());
    lines.extend(
        routes.iter().filter(|route| !route.requires_tenant).map(GatewayRouteEntry::summary_line),
    );
    lines.push("  Client APIs (require X-Platform-ID and X-Company-ID):".to_string());
    lines.extend(
        routes.iter().filter(|route| route.requires_tenant).map(GatewayRouteEntry::summary_line),
    );
    lines
}

#[derive(Debug, Clone, Copy)]
pub struct GatewayRouteEntry {
    pub method: &'static str,
    pub path: &'static str,
    pub description: &'static str,
    pub requires_tenant: bool,
}

impl GatewayRouteEntry {
    fn summary_line(&self) -> String {
        format!("    {:<6} {:<58} — {}", self.method, self.path, self.description)
    }
}

pub fn gateway_route_entries(options: GatewayRouteSummaryOptions) -> Vec<GatewayRouteEntry> {
    let mut routes = Vec::new();
    routes.extend(public_route_entries());
    if options.whatsapp_enabled {
        routes.extend([
            public_route("GET", "/v1/whatsapp", "Meta webhook verification"),
            public_route("POST", "/v1/whatsapp", "WhatsApp message webhook"),
        ]);
    }
    if options.linq_enabled {
        routes.push(public_route("POST", "/v1/linq", "Linq message webhook"));
    }
    if options.wati_enabled {
        routes.extend([
            public_route("GET", "/v1/wati", "WATI webhook verification"),
            public_route("POST", "/v1/wati", "WATI message webhook"),
        ]);
    }
    if options.nextcloud_talk_enabled {
        routes.push(public_route("POST", "/v1/nextcloud-talk", "Nextcloud Talk bot webhook"));
    }
    if options.qq_webhook_enabled {
        routes.push(public_route("POST", "/v1/qq", "QQ Bot webhook"));
    }
    routes.extend(client_route_entries());
    if options.node_control_enabled {
        routes.push(client_route("POST", "/v1/node-control", "node-control RPC"));
    }
    routes
}

/// 返回启动摘要使用的完整文档化路由清单，供跨 crate 覆盖审计复用。
pub fn documented_gateway_routes() -> Vec<GatewayRouteEntry> {
    gateway_route_entries(GatewayRouteSummaryOptions {
        whatsapp_enabled: false,
        linq_enabled: false,
        wati_enabled: false,
        nextcloud_talk_enabled: false,
        qq_webhook_enabled: false,
        node_control_enabled: false,
    })
}

const fn public_route(
    method: &'static str,
    path: &'static str,
    description: &'static str,
) -> GatewayRouteEntry {
    GatewayRouteEntry { method, path, description, requires_tenant: false }
}

const fn client_route(
    method: &'static str,
    path: &'static str,
    description: &'static str,
) -> GatewayRouteEntry {
    GatewayRouteEntry { method, path, description, requires_tenant: true }
}

fn public_route_entries() -> Vec<GatewayRouteEntry> {
    vec![
        public_route("GET", "/v1/health", "health probe"),
        public_route("POST", "/v1/webhook", "generic webhook prompt"),
    ]
}

fn client_route_entries() -> Vec<GatewayRouteEntry> {
    vec![
        client_route("GET", "/v1/metrics", "Prometheus metrics"),
        client_route("POST", "/v1/agent", "tool-enabled agent chat"),
        client_route("POST", "/v1/chat/completions", "OpenAI-compatible chat"),
        client_route("GET", "/v1/models", "list available models"),
        client_route("POST", "/v1/channels/{channel}/qrcode", "start channel QR authorization"),
        client_route(
            "GET",
            "/v1/channels/{channel}/qrcode/status",
            "poll channel QR authorization",
        ),
        client_route("GET", "/v1/ws/chat", "WebSocket agent chat"),
        client_route("GET", "/v1/events", "gateway event stream"),
        client_route("GET", "/v1/status", "runtime status"),
        client_route("GET", "/v1/tools", "tool catalog"),
        client_route("GET", "/v1/mcp/inspect", "MCP inspection"),
        #[cfg(target_os = "macos")]
        client_route(
            "POST",
            "/v1/agent-apps/diagnostics/jobs",
            "start authorized application diagnostics",
        ),
        #[cfg(target_os = "macos")]
        client_route("POST", "/v1/agent-apps/mcp/checks", "start authorized MCP connection checks"),
        #[cfg(target_os = "macos")]
        client_route(
            "POST",
            "/v1/agent-apps/diagnostics/endpoint-candidates/query",
            "query authorized endpoint candidates without probing",
        ),
        #[cfg(target_os = "macos")]
        client_route(
            "GET",
            "/v1/agent-apps/diagnostics/jobs/{id}",
            "read scoped diagnostic progress and results",
        ),
        #[cfg(target_os = "macos")]
        client_route(
            "POST",
            "/v1/agent-apps/diagnostics/jobs/{id}/cancel",
            "cancel an owned diagnostic job",
        ),
        client_route("GET", "/v1/agent-apps/mcp", "managed MCP catalog and capabilities"),
        client_route("POST", "/v1/agent-apps/mcp", "create managed MCP catalog record"),
        client_route("GET", "/v1/agent-apps/mcp/presets", "managed MCP presets"),
        client_route("GET", "/v1/agent-apps/mcp/native", "managed MCP native preview"),
        client_route("POST", "/v1/mcp/apps/call", "session-bound MCP App tool call"),
        client_route("GET", "/v1/cron/history", "cron history"),
        client_route("GET", "/v1/cron", "cron jobs"),
        client_route("POST", "/v1/cron", "create cron job"),
        client_route("POST", "/v1/cron/preview", "preview cron schedule"),
        client_route("POST", "/api/cron/preview", "preview cron schedule"),
        client_route("GET", "/v1/cron/{id}/runs", "cron job runs"),
        client_route("GET", "/v1/cron/runs/{id}", "cron run detail"),
        client_route("PATCH", "/v1/cron/{id}", "update cron job"),
        client_route("POST", "/v1/cron/{id}", "update cron job"),
        client_route("PUT", "/v1/cron/{id}", "update cron job"),
        client_route("DELETE", "/v1/cron/{id}", "delete cron job"),
        client_route("GET", "/v1/integrations", "integrations"),
        client_route("GET", "/v1/integrations/settings", "integration settings"),
        client_route("PUT", "/v1/integrations/{id}/credentials", "update credentials"),
        client_route("GET", "/v1/doctor", "diagnostics"),
        client_route("POST", "/v1/doctor", "diagnostics"),
        client_route("GET", "/v1/memory", "memory query"),
        client_route("POST", "/v1/memory", "store memory"),
        client_route("DELETE", "/v1/memory/{key}", "delete memory"),
        client_route("GET", "/v1/cli-tools", "CLI tool discovery"),
        client_route("GET", "/v1/api-health", "API health summary"),
        client_route("GET", "/v1/dashboard/config", "dashboard config"),
        client_route("PUT", "/v1/dashboard/config", "update dashboard config"),
        client_route("PUT", "/v1/auth/{provider_id}", "set provider auth"),
        client_route("DELETE", "/v1/auth/{provider_id}", "remove provider auth"),
        client_route("GET", "/v1/config", "runtime config"),
        client_route("PATCH", "/v1/config", "update runtime config"),
        client_route("GET", "/v1/config/providers", "provider config catalog"),
        client_route("GET", "/v1/hooks", "plugin hook catalog"),
        client_route("POST", "/v1/hooks/trust", "trust current plugin hook"),
        client_route("POST", "/v1/hooks/revoke", "revoke current plugin hook"),
        client_route("GET", "/v1/hooks/audit", "bounded command audit"),
        client_route("GET", "/v1/global/health", "global health"),
        client_route("GET", "/v1/global/event", "global event stream"),
        client_route("GET", "/v1/global/config", "global config"),
        client_route("PATCH", "/v1/global/config", "update global config"),
        client_route("GET", "/v1/global/config/acp", "ACP config"),
        client_route("POST", "/v1/global/dispose", "dispose global instance"),
        client_route("GET", "/v1/path", "workspace path"),
        client_route("GET", "/v1/vcs", "VCS status"),
        client_route("GET", "/v1/event", "instance event stream"),
        client_route("POST", "/v1/instance/dispose", "dispose instance"),
        client_route("GET", "/v1/provider", "provider list"),
        client_route("GET", "/v1/provider/auth", "provider auth methods"),
        client_route("POST", "/v1/provider/{provider_id}/oauth/authorize", "start provider OAuth"),
        client_route("POST", "/v1/provider/{provider_id}/oauth/callback", "finish provider OAuth"),
        client_route("GET", "/v1/doc", "documentation placeholder"),
        client_route("GET", "/v1/command", "command catalog"),
        client_route("GET", "/v1/agent", "agent catalog"),
        client_route("POST", "/v1/agent/{key}/copy", "copy agent profile"),
        client_route("GET", "/v1/skill", "skill catalog"),
        client_route("GET", "/v1/find", "find text"),
        client_route("GET", "/v1/find/file", "find file"),
        client_route("GET", "/v1/find/symbol", "find symbol placeholder"),
        client_route("GET", "/v1/file", "file list"),
        client_route("POST", "/v1/file/read", "read file"),
        client_route("GET", "/v1/file/content", "read file content"),
        client_route("POST", "/v1/file/write", "write file"),
        client_route("GET", "/v1/file/status", "file status"),
        client_route("POST", "/v1/files/move", "move file"),
        client_route("POST", "/v1/files/import-local", "import local paths"),
        client_route("POST", "/v1/files/copy", "copy file"),
        client_route("POST", "/v1/files/duplicate", "duplicate file"),
        client_route("POST", "/v1/files/delete", "delete files"),
        client_route("POST", "/v1/git/commit", "git commit"),
        client_route("POST", "/v1/git/command", "git command"),
        client_route("POST", "/v1/git/merge", "git merge"),
        client_route("GET", "/v1/permission", "permission requests"),
        client_route("POST", "/v1/permission/{request_id}/reply", "approve permission request"),
        client_route("GET", "/v1/question", "questions"),
        client_route("POST", "/v1/question/{request_id}/reply", "answer question"),
        client_route("POST", "/v1/question/{request_id}/reject", "reject question"),
        client_route("GET", "/v1/pty", "PTY list"),
        client_route("POST", "/v1/pty", "create PTY"),
        client_route("GET", "/v1/pty/{pty_id}", "PTY detail"),
        client_route("PUT", "/v1/pty/{pty_id}", "update PTY"),
        client_route("DELETE", "/v1/pty/{pty_id}", "remove PTY"),
        client_route("GET", "/v1/pty/{pty_id}/connect", "PTY WebSocket"),
        client_route("GET", "/v1/projects", "project list"),
        client_route("POST", "/v1/projects/resolve", "resolve project"),
        client_route("GET", "/v1/projects/{project_id}", "project detail"),
        client_route("PATCH", "/v1/projects/{project_id}", "update project"),
        client_route("GET", "/v1/projects/{project_id}/change-records", "project change records"),
        client_route("GET", "/v1/projects/{project_id}/worktrees", "project worktrees"),
        client_route("POST", "/v1/projects/{project_id}/worktrees", "create worktree"),
        client_route("GET", "/v1/projects/{project_id}/worktrees/{worktree_id}", "worktree detail"),
        client_route(
            "DELETE",
            "/v1/projects/{project_id}/worktrees/{worktree_id}",
            "delete worktree",
        ),
        client_route(
            "POST",
            "/v1/projects/{project_id}/worktrees/{worktree_id}/reset",
            "reset worktree",
        ),
        client_route("GET", "/v1/knowledge/status", "knowledge status"),
        client_route("GET", "/v1/knowledge/embedding/status", "embedding status"),
        client_route("GET", "/v1/knowledge/embedding/presets", "embedding presets"),
        client_route("POST", "/v1/knowledge/embedding/test", "test embedding"),
        client_route("GET", "/v1/knowledge/datasets", "knowledge datasets"),
        client_route("POST", "/v1/knowledge/datasets", "create knowledge dataset"),
        client_route("GET", "/v1/knowledge/datasets/{dataset_id}", "knowledge dataset"),
        client_route("DELETE", "/v1/knowledge/datasets/{dataset_id}", "delete knowledge dataset"),
        client_route("GET", "/v1/knowledge/datasets/{dataset_id}/documents", "knowledge documents"),
        client_route(
            "POST",
            "/v1/knowledge/datasets/{dataset_id}/documents",
            "create knowledge document",
        ),
        client_route(
            "POST",
            "/v1/knowledge/datasets/{dataset_id}/documents/import",
            "import document",
        ),
        client_route(
            "POST",
            "/v1/knowledge/datasets/{dataset_id}/documents/import-async",
            "import document async",
        ),
        client_route("GET", "/v1/knowledge/index-jobs/{job_id}", "knowledge index job"),
        client_route("POST", "/v1/knowledge/documents/batch", "batch update documents"),
        client_route("PATCH", "/v1/knowledge/documents/{document_id}", "update document"),
        client_route("DELETE", "/v1/knowledge/documents/{document_id}", "delete document"),
        client_route("GET", "/v1/knowledge/documents/{document_id}/chunks", "document chunks"),
        client_route("POST", "/v1/knowledge/documents/{document_id}/reindex", "reindex document"),
        client_route("POST", "/v1/knowledge/retrieve", "knowledge retrieve"),
        client_route("POST", "/v1/knowledge/retrieval-test", "knowledge retrieval test"),
        client_route("POST", "/v1/task-pool/worktrees/snapshot", "worktree pool snapshot"),
        client_route("POST", "/v1/task-pool/worktrees/maintain", "maintain worktree pool"),
        client_route("POST", "/v1/task-pool/worktrees/acquire", "acquire worktree"),
        client_route("POST", "/v1/task-pool/worktrees/release", "release worktree"),
        client_route("POST", "/v1/task-pool/worktrees/recycle", "recycle worktree"),
        client_route("POST", "/v1/task-pool/worktrees/slots/recycle", "recycle worktree slots"),
        client_route("POST", "/v1/task-pool/worktrees/merge-lock", "lock worktree merge"),
        client_route("POST", "/v1/task-pool/worktrees/merge-unlock", "unlock worktree merge"),
        client_route(
            "GET",
            "/v1/agent-scheduler/projects/{project_id}/workflow",
            "scheduler workflow",
        ),
        client_route("GET", "/v1/agent-scheduler/projects/{project_id}/tasks", "scheduler tasks"),
        client_route(
            "POST",
            "/v1/agent-scheduler/projects/{project_id}/tasks",
            "create scheduler task",
        ),
        client_route(
            "POST",
            "/v1/agent-scheduler/projects/{project_id}/commands",
            "scheduler command",
        ),
        client_route(
            "POST",
            "/v1/agent-scheduler/projects/{project_id}/commands/bulk",
            "bulk scheduler commands",
        ),
        client_route(
            "PATCH",
            "/v1/agent-scheduler/projects/{project_id}/tasks/{task_id}/metadata",
            "update task metadata",
        ),
        client_route(
            "POST",
            "/v1/agent-scheduler/projects/{project_id}/legacy-import",
            "legacy task import",
        ),
        client_route(
            "GET",
            "/v1/agent-scheduler/projects/{project_id}/tasks/{task_id}/logs",
            "task logs",
        ),
        client_route(
            "GET",
            "/v1/agent-scheduler/projects/{project_id}/tasks/{task_id}/artifacts",
            "task artifacts",
        ),
        client_route("GET", "/v1/agent-scheduler/projects/{project_id}/events", "scheduler events"),
        client_route(
            "GET",
            "/v1/agent-scheduler/projects/{project_id}/events/stream",
            "scheduler event stream",
        ),
        client_route(
            "POST",
            "/v1/agent-scheduler/projects/{project_id}/wake",
            "wake scheduler project",
        ),
        client_route(
            "POST",
            "/v1/agent-scheduler/projects/{project_id}/outbox/drain",
            "drain scheduler outbox",
        ),
        client_route("POST", "/v1/workflow/resume", "resume workflow"),
        client_route("POST", "/v1/workflow/applications/chat-messages", "workflow chat"),
        client_route(
            "POST",
            "/v1/workflow/applications/{uuid}/chat-messages",
            "workflow application chat",
        ),
        client_route("GET", "/v1/workflow/applications", "workflow applications"),
        client_route("POST", "/v1/workflow/applications", "create workflow application"),
        client_route("GET", "/v1/workflow/applications/{uuid}", "workflow application"),
        client_route("PUT", "/v1/workflow/applications/{uuid}", "update workflow application"),
        client_route("DELETE", "/v1/workflow/applications/{uuid}", "delete workflow application"),
        client_route("POST", "/v1/chat/stream", "session chat stream"),
        client_route("POST", "/v1/chat/stream/{request_id}/stop", "stop session chat stream"),
        client_route("POST", "/v1/task-plan-run/stream", "task plan-run stream"),
        client_route(
            "POST",
            "/v1/task-plan-run/stream/{request_id}/stop",
            "stop task plan-run stream",
        ),
        client_route("GET", "/v1/session", "sessions"),
        client_route("POST", "/v1/session", "create session"),
        client_route("GET", "/v1/session/status", "session status"),
        client_route(
            "POST",
            "/v1/session/{session_id}/dynamic-tools/hosts",
            "bind dynamic tool host",
        ),
        client_route(
            "PUT",
            "/v1/session/{session_id}/dynamic-tools",
            "replace session dynamic tools",
        ),
        client_route(
            "GET",
            "/v1/session/{session_id}/dynamic-tools/calls",
            "host dynamic tool call stream",
        ),
        client_route(
            "POST",
            "/v1/session/{session_id}/dynamic-tools/calls/{call_id}/response",
            "respond to host tool call",
        ),
        client_route("GET", "/v1/session/{session_id}", "session"),
        client_route("PATCH", "/v1/session/{session_id}", "update session"),
        client_route("DELETE", "/v1/session/{session_id}", "delete session"),
        client_route("GET", "/v1/session/{session_id}/children", "session children"),
        client_route("GET", "/v1/session/{session_id}/todo", "session todo"),
        client_route("PUT", "/v1/session/{session_id}/todo", "update session todo"),
        client_route("POST", "/v1/session/{session_id}/fork", "fork session"),
        client_route("POST", "/v1/session/{session_id}/reset", "reset session"),
        client_route("GET", "/v1/session/{session_id}/diff", "session diff"),
        client_route("POST", "/v1/session/{session_id}/summarize", "summarize session"),
        client_route("GET", "/v1/session/{session_id}/message", "session messages"),
        client_route("GET", "/v1/session/{session_id}/message/{message_id}", "session message"),
        client_route(
            "PATCH",
            "/v1/session/{session_id}/message/{message_id}/part/{part_id}",
            "update message part",
        ),
        client_route(
            "DELETE",
            "/v1/session/{session_id}/message/{message_id}/part/{part_id}",
            "delete message part",
        ),
        client_route("GET", "/v1/session/ui-previews", "session UI previews"),
        client_route("GET", "/v1/session/archived", "archived sessions"),
        client_route("PUT", "/v1/session/archived", "update archived sessions"),
        client_route("GET", "/v1/session/scope", "session scope"),
        client_route("PUT", "/v1/session/scope", "update session scope"),
        client_route("GET", "/v1/session/{session_id}/ui", "session UI"),
        client_route("PUT", "/v1/session/{session_id}/ui", "save session UI"),
        client_route("GET", "/v1/session/{session_id}/preview", "session preview"),
        client_route("GET", "/v1/session/{session_id}/path", "session path"),
        client_route("GET", "/v1/session/{session_id}/any", "session by any ID"),
        client_route("POST", "/v1/session/{session_id}/title", "generate session title"),
        client_route("GET", "/v1/task-lifecycle/workflows", "task lifecycle workflows"),
        client_route("POST", "/v1/task-lifecycle/workflows", "create task lifecycle workflow"),
        client_route("GET", "/v1/task-lifecycle/workflows/{id}", "task lifecycle workflow"),
        client_route("PUT", "/v1/task-lifecycle/workflows/{id}", "update task lifecycle workflow"),
        client_route(
            "DELETE",
            "/v1/task-lifecycle/workflows/{id}",
            "delete task lifecycle workflow",
        ),
        client_route(
            "POST",
            "/v1/task-lifecycle/workflows/{id}/reset",
            "reset task lifecycle workflow",
        ),
        client_route("POST", "/v1/shiju/account/login", "start SHIJU account login"),
        client_route("POST", "/v1/shiju/account/login/poll", "poll SHIJU account login"),
        client_route("GET", "/v1/shiju/account/profile", "SHIJU account profile"),
        client_route("GET", "/v1/shiju/account/monthly-quota", "SHIJU account monthly quota"),
        client_route("GET", "/v1/shiju/apps/catalog", "SHIJU application catalog"),
        client_route("GET", "/v1/shiju/apps", "installed SHIJU applications"),
        client_route("GET", "/v1/shiju/apps/api-keys", "SHIJU IAM API keys"),
        client_route(
            "GET",
            "/v1/shiju/apps/{installation_id}/api-key-binding",
            "SHIJU application API key binding",
        ),
        client_route(
            "PUT",
            "/v1/shiju/apps/{installation_id}/api-key-binding",
            "update SHIJU application API key binding",
        ),
        client_route(
            "POST",
            "/v1/shiju/apps/{installation_id}/launch",
            "prepare SHIJU application launch",
        ),
        client_route("GET", "/v1/shiju/skills/search", "search SHIJU skills"),
        client_route("GET", "/v1/shiju/skills/installed", "installed SHIJU skills"),
        client_route("GET", "/v1/shiju/skills/system", "SHIJU system skills"),
        client_route("GET", "/v1/shiju/skills/maintained", "maintained SHIJU skills"),
        client_route("GET", "/v1/shiju/skills/{skill_id}", "SHIJU skill detail"),
        client_route("GET", "/v1/shiju/skills/{skill_id}/versions", "SHIJU skill versions"),
        client_route("POST", "/v1/shiju/skills/{skill_id}/install", "install SHIJU skill"),
        client_route("DELETE", "/v1/shiju/skills/{skill_id}/install", "uninstall SHIJU skill"),
        client_route("GET", "/v1/skill-market/sources", "skill market sources"),
        client_route("GET", "/v1/skill-market/showcase/recommended", "recommended market skills"),
        client_route("GET", "/v1/skill-market/skills", "market skill catalog"),
        client_route("GET", "/v1/skill-market/categories", "market skill categories"),
        client_route("GET", "/v1/skill-market/skills/{slug}", "market skill detail"),
        client_route("GET", "/v1/skill-market/skills/{slug}/file", "market skill file"),
        client_route("GET", "/v1/skill-market/skillsets", "market skill-set catalog"),
        client_route("GET", "/v1/skill-market/skillsets/{slug}", "market skill-set detail"),
        client_route("GET", "/v1/skills", "skills catalog"),
        client_route("GET", "/v1/skills/detail", "skill detail"),
        client_route("POST", "/v1/skills/create", "create skill"),
        client_route("POST", "/v1/skills/install-built-in", "install built-in skill"),
        client_route("POST", "/v1/skills/set-enabled", "set skill enabled"),
        client_route("POST", "/v1/skills/delete", "delete skill"),
        client_route("GET", "/v1/plugins", "plugins catalog"),
        client_route("GET", "/v1/plugins/detail", "plugin detail"),
        client_route("POST", "/v1/plugins/set-enabled", "set plugin enabled"),
        client_route("POST", "/v1/plugins/delete", "delete plugin"),
        client_route("GET", "/v1/desktop/preferences", "desktop preferences"),
        client_route("PATCH", "/v1/desktop/preferences", "update desktop preferences"),
        client_route("GET", "/v1/desktop/tool-content/{tool_type}", "desktop tool content"),
        client_route("PUT", "/v1/desktop/tool-content/{tool_type}", "update desktop tool content"),
        client_route("GET", "/v1/desktop/skills", "desktop skills catalog"),
        client_route("GET", "/v1/desktop/skills/detail", "desktop skill detail"),
        client_route("POST", "/v1/desktop/skills/create", "create desktop skill"),
        client_route(
            "POST",
            "/v1/desktop/skills/install-built-in",
            "install desktop built-in skill",
        ),
        client_route("POST", "/v1/desktop/skills/set-enabled", "set desktop skill enabled"),
        client_route("POST", "/v1/desktop/skills/delete", "delete desktop skill"),
        client_route("GET", "/v1/desktop/external-apps", "external apps"),
        client_route("POST", "/v1/desktop/external-apps/open", "open external app"),
        client_route("POST", "/v1/desktop/external-path/kind", "classify external path"),
        client_route("POST", "/v1/desktop/external-path/reveal", "reveal external path"),
        client_route("POST", "/v1/desktop/service/{command}", "desktop service command"),
        client_route("GET", "/v1/desktop/project-preferences", "desktop project preferences"),
        client_route(
            "PUT",
            "/v1/desktop/project-preferences",
            "update desktop project preferences",
        ),
    ]
}
