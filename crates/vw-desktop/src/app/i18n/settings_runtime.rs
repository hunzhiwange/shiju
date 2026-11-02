use vw_config_types::ui::DesktopLocale;
/// Translate a Chinese source key in the caller's locale.
pub fn text(locale: DesktopLocale, source: &'static str) -> &'static str {
    super::I18n::new(locale).__(source)
}

/// Help content is selected at render time; configuration keys retain their spelling.
pub fn observability_help(locale: DesktopLocale) -> String {
    let content = super::I18n::new(locale).__("可观测性配置说明\n\n一、作用\n- observability 用于控制运行时观测后端与追踪存储行为。\n\n二、字段含义\n1) backend\n- 类型: \"none\" | \"log\" | \"prometheus\" | \"otel\"\n\n2) otelendpoint\n- OTLP 采集端点，仅 backend = \"otel\" 时生效。\n\n3) otel_service_name\n- 上报到 OTel 的服务名，仅 backend = \"otel\" 时生效。\n\n4) runtime_trace_mode\n- 类型: \"none\" | \"rolling\" | \"full\"\n\n5) runtime_trace_path\n- 追踪文件路径；相对路径相对 workspace_dir 解析。\n\n6) runtime_trace_max_entries\n- rolling 模式下保留的最大条目数。\n\n三、示例\n{\n  \"observability\": {\n    \"backend\": \"none\",\n    \"otel_endpoint\": null,\n    \"otel_service_name\": null,\n    \"runtime_trace_mode\": \"none\",\n    \"runtime_trace_path\": \"state/runtime-trace.jsonl\",\n    \"runtime_trace_max_entries\": 200\n  }\n}\n");
    if locale == DesktopLocale::SimplifiedChinese {
        content.to_owned()
    } else {
        format!(
            "{content}\n\n{}",
            "{\n  \"observability\": {\n    \"backend\": \"none\",\n    \"otel_endpoint\": null,\n    \"otel_service_name\": null,\n    \"runtime_trace_mode\": \"none\",\n    \"runtime_trace_path\": \"state/runtime-trace.jsonl\",\n    \"runtime_trace_max_entries\": 200\n  }\n}"
        )
    }
}

/// Translate only locally produced validation messages; external errors remain verbatim.
pub fn validation_error(locale: DesktopLocale, raw: &str) -> String {
    let i18n = super::I18n::new(locale);
    if let Some(value) = raw.strip_prefix("列表项已存在：") {
        return crate::__!(i18n, "列表项已存在：%s", value).expect("valid duplicate message");
    }
    match raw {
        "列表项不能为空" => i18n.__(raw).to_owned(),
        "要删除的列表项不存在" => i18n.__(raw).to_owned(),
        "connect_timeout_secs 必须是整数秒或留空" => i18n.__(raw).to_owned(),
        "reasoning_level 仅支持 minimal / low / medium / high / xhigh" => {
            i18n.__(raw).to_owned()
        }
        "Docker 内存限制 必须是非负整数" => i18n.__(raw).to_owned(),
        "Docker 内存限制 必须是数字" => i18n.__(raw).to_owned(),
        "Docker 内存限制 必须大于 0" => i18n.__(raw).to_owned(),
        "Docker CPU 限制 必须是非负整数" => i18n.__(raw).to_owned(),
        "Docker CPU 限制 必须是数字" => i18n.__(raw).to_owned(),
        "Docker CPU 限制 必须大于 0" => i18n.__(raw).to_owned(),
        _ => raw.to_owned(),
    }
}
