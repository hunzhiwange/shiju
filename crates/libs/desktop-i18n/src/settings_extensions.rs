use config_ui::DesktopLocale;

pub fn text(locale: DesktopLocale, source: &'static str) -> &'static str {
    super::I18n::new(locale).__(source)
}

pub fn operation_error(locale: DesktopLocale, raw: &str) -> String {
    crate::__!(super::I18n::new(locale), "操作失败：%s", raw)
        .expect("valid operation error template")
}

pub fn domain_label(locale: DesktopLocale, index: usize) -> String {
    crate::__!(super::I18n::new(locale), "域名 %u", index).expect("valid domain label template")
}

pub fn archived_chat_count(locale: DesktopLocale, count: usize) -> String {
    crate::__!(super::I18n::new(locale), "%u 个聊天", count).expect("valid archive count template")
}

pub fn archived_chat_updated(locale: DesktopLocale, updated: u64) -> String {
    let formatted = i64::try_from(updated)
        .ok()
        .and_then(chrono::DateTime::from_timestamp_millis)
        .map(|time| time.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| super::I18n::new(locale).__("未知时间").to_owned());
    crate::__!(super::I18n::new(locale), "更新于 %s", formatted)
        .expect("valid archive time template")
}

/// Render locally generated extension feedback at display time. Raw arguments are
/// never interpreted as templates, so paths, names and backend diagnostics survive.
pub fn feedback(i18n: super::I18n, raw: &str) -> String {
    feedback_message(raw).render(i18n).expect("valid extension feedback template")
}

/// Keep the template and its data until the current locale is known.
pub fn feedback_message(raw: &str) -> crate::format::Message {
    for source in FEEDBACK_TEMPLATES {
        let mut parts = source.split("%s");
        let prefix = parts.next().expect("template prefix");
        let Some(mut remaining) = raw.strip_prefix(prefix) else { continue };
        let mut arguments = Vec::new();
        let mut matched = true;
        for delimiter in parts {
            if delimiter.is_empty() {
                arguments.push(crate::format::Argument::from(remaining));
                remaining = "";
            } else if let Some((value, rest)) = remaining.split_once(delimiter) {
                arguments.push(crate::format::Argument::from(value));
                remaining = rest;
            } else {
                matched = false;
                break;
            }
        }
        if matched && remaining.is_empty() {
            if matches!(*source, "%s读取失败 · %s" | "提示词已保存。%s" | "目录删除已保存；%s")
            {
                return crate::format::Message::nested(
                    (*source).to_owned(),
                    arguments
                        .into_iter()
                        .map(|value| match value {
                            crate::format::Argument::Text(value) => feedback_message(&value),
                            _ => unreachable!("text feedback"),
                        })
                        .collect(),
                );
            }
            return crate::format::Message::new(*source, arguments);
        }
    }
    for suffix in [
        "已新增（保留来源启用或禁用状态）",
        "同定义，已新增绑定（保留来源启用状态）",
        "已有同定义绑定，无需变更",
        "冲突，未覆盖已有定义",
        "成功：已保存定义，未启用或分发",
        "失败：定义冲突，未覆盖",
        "失败：未确认保存成功，请刷新目录核实",
    ] {
        if let Some(id) = raw.strip_suffix(suffix).and_then(|value| value.strip_suffix(" · ")) {
            return crate::format::Message::nested(
                "%s · %s".into(),
                vec![
                    crate::format::Message::literal(id.to_owned()),
                    crate::format::Message::new(suffix, vec![]),
                ],
            );
        }
    }
    if !raw.contains('%') && super::I18n::new(DesktopLocale::English).__(raw) != raw {
        crate::format::Message::new(raw, vec![])
    } else {
        crate::format::Message::literal(raw.to_owned())
    }
}

const FEEDBACK_TEMPLATES: &[&str] = &[
    "%s %s 未知、已禁用或不在当前作用域，请移除或选择继承。",
    "%s · 已导入定义，尚未分发或启用",
    "%s · 目录已有项 · 原生 %s · %s · %s",
    "%s 个仓库读取失败",
    "%s 已保存",
    "%s读取失败 · %s",
    "%s：失败 %s（%s）",
    "主副本已安装；应用分发部分失败：%s，请在应用动作中核对",
    "主副本已安装；应用分发需核对：%s",
    "仓库 %s/%s（%s）发现失败：%s",
    "保存失败: %s",
    "保存智能体配置失败：%s",
    "卸载失败后的回滚也失败: %s",
    "原生状态：%s（脱敏预览）",
    "可复制 %s 项，跳过 %s 项",
    "备份失败后的回滚也失败: %s",
    "失败仓库：%s/%s (%s)：%s",
    "失败，可重试：%s",
    "安装失败后的回滚也失败: %s",
    "导入失败：%s",
    "已卸载；清理未完成：%s",
    "已导入 %s 项技能",
    "已导入 %s 项技能，%s 项失败：%s",
    "已跳过：%s",
    "已载入 %s 个服务器，跳过 %s 个同名项。检查 JSON 后点击保存。",
    "已迁移：%s",
    "库存刷新失败：%s",
    "当前运行时配置中找不到 MCP 服务器 %s",
    "扫码授权未返回所需字段：%s",
    "技能导入失败：%s",
    "技能检查失败：%s：%s",
    "提示词已保存。%s",
    "操作请求失败：%s；正在重新查询实际状态",
    "新增 %s · 绑定变更 %s · 跳过 %s · 失败 %s",
    "无新增服务器，跳过 %s 个同名项",
    "智能体 %s 已存在。",
    "智能体目录响应无效：%s",
    "更新任务：%s；需核对：%s；内容哈希：%s",
    "更新检查失败：%s",
    "未导入：%s（无配置、未读到或不支持）",
    "未找到 %s 的默认模板。",
    "来源导入失败：%s",
    "环境同步失败后的回滚也失败: %s",
    "环境备份失败后的回滚也失败: %s",
    "目录删除已保存；%s",
    "第 %s 项：%s",
    "缺少有效 %s",
    "网关返回 HTTP %s",
    "读取失败: %s",
    "读取配置失败: %s",
];
