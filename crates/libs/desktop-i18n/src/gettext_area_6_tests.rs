use super::settings_extensions::{domain_label, operation_error, text};
use config_ui::DesktopLocale;

#[test]
fn desktop_gettext_area_6_uses_chinese_keys() {
    let raw = "用户/%n/原始错误";
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "钩子配置"),
        (DesktopLocale::TraditionalChinese, "鉤子設定"),
        (DesktopLocale::English, "Hook configuration"),
    ] {
        assert_eq!(text(locale, "钩子配置"), expected);
        assert!(operation_error(locale, raw).ends_with(raw));
        assert!(domain_label(locale, 42).ends_with("42"));
    }
}

#[test]
fn desktop_gettext_area_6_catalog_covers_hooks_and_http() {
    let en = super::catalog::parse_catalog(include_str!("locales/en.json")).unwrap();
    let tw = super::catalog::parse_catalog(include_str!("locales/zh-TW.json")).unwrap();
    for source in [
        "钩子配置",
        "网络请求配置",
        "操作失败：%s",
        "域名 %u",
        "例如：example.com 或 *.example.org 或 *",
    ] {
        assert!(en.contains_key(source), "{source}");
        assert!(tw.contains_key(source), "{source}");
    }
    let raw = "原始 %s %n";
    let chinese = operation_error(DesktopLocale::SimplifiedChinese, raw);
    let english = operation_error(DesktopLocale::English, raw);
    assert_eq!(chinese, format!("操作失败：{raw}"));
    assert_eq!(english, format!("Operation failed: {raw}"));
}

#[test]
fn desktop_gettext_area_6_network_and_agent_basic_cover_visible_sources() {
    for source in ["隧道配置", "网页搜索配置", "子智能体委托", "max_results 必须是整数"]
    {
        let en = super::I18n::new(DesktopLocale::English).__(source);
        let tw = super::I18n::new(DesktopLocale::TraditionalChinese).__(source);
        assert_ne!(en, source);
        assert_ne!(tw, source);
        assert_eq!(super::I18n::new(DesktopLocale::SimplifiedChinese).__(source), source);
    }
}

#[test]
fn desktop_gettext_area_6_remaining_sections_have_catalog_entries() {
    let en = super::catalog::parse_catalog(include_str!("locales/en.json")).unwrap();
    let tw = super::catalog::parse_catalog(include_str!("locales/zh-TW.json")).unwrap();
    for source in [
        "智能体配置",
        "技能访问策略",
        "只保留只读分析能力。",
        "群聊回复模式",
        "全部消息",
        "飞书实例 %s",
        "工作项目",
        "MCP 配置",
        "%u 个服务器",
        "另有 %u 个工具未展开",
        "技能配置",
        "发现顺序",
        "安装插件",
        "原声明：技能目录 %u · MCP 声明组 %u · 钩子声明组 %u · 界面 %s",
        "读取技能目录失败: %s",
        "已删除插件: %s",
        "隧道配置",
        "网页搜索配置",
    ] {
        assert!(en.contains_key(source), "missing English: {source}");
        assert!(tw.contains_key(source), "missing Traditional Chinese: {source}");
    }
}
