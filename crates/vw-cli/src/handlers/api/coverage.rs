//! Gateway 静态路由到根级 `shiju` 命令族的覆盖分类。

use vw_agent::gateway::GatewayRouteEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RouteCoverage {
    Command(&'static str),
    Alias(&'static str),
    Excluded(&'static str),
}

pub(super) fn classify_route(route: &GatewayRouteEntry) -> Option<RouteCoverage> {
    let path = route.path;
    if path == "/v1/health" {
        return Some(RouteCoverage::Command("system"));
    }
    if !route.requires_tenant {
        return Some(RouteCoverage::Excluded("public probe or third-party inbound webhook"));
    }
    if matches!(path, "/v1/doc" | "/v1/find/symbol") {
        return Some(RouteCoverage::Excluded("documented placeholder returns 501 Not Implemented"));
    }
    if path.ends_with("/worker-events")
        || (path == "/v1/task-pool/matrix/events" && route.method == "POST")
    {
        return Some(RouteCoverage::Excluded("authenticated internal producer callback"));
    }
    if path.starts_with("/v1/apiQL/") && path.ends_with('/') {
        return Some(RouteCoverage::Alias("apiql canonical route without trailing slash"));
    }
    if path == "/v1/cron/runs/{id}" {
        return Some(RouteCoverage::Alias("api operations cron runs"));
    }
    if path == "/v1/cron/{id}" && matches!(route.method, "POST" | "PUT") {
        return Some(RouteCoverage::Alias("api operations cron update uses PATCH"));
    }

    command_family(route).map(RouteCoverage::Command)
}

fn command_family(route: &GatewayRouteEntry) -> Option<&'static str> {
    let path = route.path;
    let method = route.method;
    if matches!(path, "/v1/agent" | "/v1/chat/completions") && method == "POST"
        || matches!(path, "/v1/events" | "/v1/global/event" | "/v1/event" | "/v1/ws/chat")
        || path.starts_with("/v1/chat/stream")
        || path.starts_with("/v1/task-plan-run/stream")
        || path.ends_with("/events/stream")
    {
        return Some("realtime");
    }
    if path.starts_with("/v1/cron")
        || path.starts_with("/v1/integrations")
        || path.starts_with("/v1/memory")
        || path == "/v1/doctor"
    {
        return Some("operations");
    }
    if path.starts_with("/v1/auth/")
        || path.starts_with("/v1/hooks")
        || matches!(path, "/v1/global/dispose" | "/v1/instance/dispose")
    {
        return Some("control");
    }
    if path.starts_with("/v1/dashboard/config")
        || path == "/v1/config"
        || path == "/v1/config/providers"
        || path.starts_with("/v1/global/config")
    {
        return Some("config");
    }
    if path.starts_with("/v1/provider") {
        return Some("provider");
    }
    if matches!(path, "/v1/permission" | "/v1/question")
        || path.starts_with("/v1/permission/")
        || path.starts_with("/v1/question/")
        || path.starts_with("/v1/pty")
    {
        return Some("interactive");
    }
    if path.starts_with("/v1/projects") {
        return Some("project");
    }
    if path.starts_with("/v1/knowledge") {
        return Some("knowledge");
    }
    if path.starts_with("/v1/task-pool") {
        return Some("task-pool");
    }
    if path.starts_with("/v1/agent-scheduler") {
        return Some("scheduler");
    }
    if path.starts_with("/v1/apiQL/") {
        return Some("apiql");
    }
    if path.starts_with("/v1/workflow") {
        return Some("workflow");
    }
    if path.starts_with("/v1/session") {
        if path.contains("/message")
            || path.contains("/ui")
            || path.ends_with("/preview")
            || path.ends_with("/path")
            || path.ends_with("/any")
            || matches!(path, "/v1/session/archived" | "/v1/session/scope")
        {
            return Some("session-content");
        }
        return Some("session");
    }
    if path.starts_with("/v1/task-lifecycle") {
        return Some("task-lifecycle");
    }
    if path.starts_with("/v1/shiju/account") || path.starts_with("/v1/shiju/apps") {
        return Some("shiju-apps");
    }
    if path.starts_with("/v1/shiju/skills") || path.starts_with("/v1/skill-market") {
        return Some("skill-market");
    }
    if path.starts_with("/v1/skills") || path.starts_with("/v1/plugins") {
        return Some("extensions");
    }
    if path.starts_with("/v1/desktop") {
        return Some("desktop");
    }
    if path.starts_with("/v1/find")
        || path.starts_with("/v1/file")
        || path.starts_with("/v1/files")
        || path.starts_with("/v1/git")
    {
        return Some("workspace");
    }
    if matches!(
        path,
        "/v1/health"
            | "/v1/metrics"
            | "/v1/models"
            | "/v1/status"
            | "/v1/tools"
            | "/v1/mcp/inspect"
            | "/v1/cli-tools"
            | "/v1/api-health"
            | "/v1/global/health"
            | "/v1/path"
            | "/v1/vcs"
            | "/v1/command"
            | "/v1/skill"
    ) || (path == "/v1/agent" && method == "GET")
    {
        return Some("system");
    }
    None
}
