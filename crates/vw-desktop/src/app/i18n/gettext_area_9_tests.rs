use super::I18n;
use vw_config_types::ui::DesktopLocale;

#[test]
fn desktop_gettext_area_9_uses_chinese_keys() {
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "没有匹配的文件"),
        (DesktopLocale::TraditionalChinese, "沒有符合的檔案"),
        (DesktopLocale::English, "No matching files"),
    ] {
        assert_eq!(I18n::new(locale).__("没有匹配的文件"), expected);
    }
    assert!(
        include_str!("../components/input_panel/file_search.rs")
            .contains(".__(\"没有匹配的文件\")")
    );
}

#[test]
fn desktop_gettext_area_9_display_keys_are_covered() {
    let en = super::catalog::parse_catalog(include_str!("locales/en.json")).unwrap();
    let tw = super::catalog::parse_catalog(include_str!("locales/zh-TW.json")).unwrap();
    for key in [
        "%s\n标识：%s\n版本：%s\n\n%s",
        "%s（上次检测）",
        "%u 个工作中",
        "ACP 智能体",
        "AI 未生成有效的提交标题和摘要，未执行提交",
        "Git 工作树已切换，未执行提交",
        "Git 提交正在进行中",
        "MCP 应用加载失败: %s",
        "auto / provider/model / 自定义模型",
        "worktree 名称仅支持小写英文、数字、连字符(-)",
        "上下文上限: %d",
        "上移",
        "下移",
        "主工作区",
        "主工作区禁止删除",
        "主工作区禁止重置",
        "任务拆分",
        "任务模式",
        "任务池",
        "优先级",
        "会话保存失败：%s",
        "会话压缩失败：%s",
        "会话压缩完成",
        "会话压缩正在进行中",
        "会话开始后不能切换 ACP 后端",
        "会话消息保存失败，已停止自动重试，待存消息仍保留在内存中：%s",
        "使用率: %.1f%%",
        "例如: feature-login",
        "保存项目扩展配置失败: %s",
        "停止",
        "停止 plan-run 失败：%s",
        "停止 plan-run 失败：响应任务不匹配（请求 %s，响应 %s）",
        "停止任务",
        "停止后台请求失败：%s",
        "全选",
        "关闭",
        "兼容特殊调度器或未出现在列表中的模型 ID",
        "再显示 %u 项",
        "再次编辑文字",
        "分叉会话失败: %s",
        "切换 ACP 后端：%s",
        "创建会话",
        "创建并使用该工作区",
        "创建新会话失败：%s",
        "删除",
        "删除失败: %s",
        "删除失败，是否强制删除工作区 %s ?",
        "刷新会话",
        "前往识矩技能升级",
        "剪切",
        "加载中...",
        "加载中…",
        "加载会话",
        "加载失败：%s",
        "加载工作区列表失败: %s",
        "加载更多 (剩余 %s)",
        "压缩会话",
        "压缩当前会话",
        "取消",
        "取消专家",
        "另一个快捷动作正在进行中",
        "在访达中显示",
        "复制",
        "复制对话",
        "大模型",
        "子任务",
        "子智能体",
        "将执行 git reset --hard <base-ref> 和 git clean -fd",
        "展开发送方式",
        "工作中的子代理会话",
        "工作区已重置",
        "工作流执行",
        "工具 %u：%s",
        "已完成 %u / %u 项待办",
        "已请求停止，正在等待运行状态收敛",
        "已选工具与技能",
        "已选择",
        "强制删除",
        "归档",
        "当前会话没有可用的项目目录",
        "当前会话没有需要压缩的较早历史",
        "当前工具范围无法直接反选",
        "当前没有可压缩的会话",
        "当前没有可反选的 Git 变更",
        "当前没有可选择的 Git 变更",
        "当前项目目录为空，无法创建新 worktree",
        "快捷消息内容不能为空",
        "恢复消息 ID 失败: %s",
        "手工填写模型",
        "打开 Git 面板",
        "打开 Git 面板 · %u",
        "技能 %u：%s",
        "排队 %s",
        "推理强度",
        "搜索模型…",
        "收起发送方式",
        "数字越小越优先",
        "新增子任务",
        "新增项目",
        "无执行中任务",
        "无法停止：plan-run task ID 为空",
        "无法恢复消息 ID: 本地 %s 条，网关 %s 条，改用本地历史分叉",
        "暂无可选模型（请先在系统设置里启用模型）",
        "暂无空间",
        "暂无配置",
        "有更新",
        "服务端版本：",
        "未安装",
        "未找到 MCP 服务器：%s",
        "未知",
        "未知（刷新失败）",
        "未选择项目，无法创建会话",
        "本地分叉会话失败: %s",
        "本地文件或截图粘贴，已选 %u 个，图片 %u/%u，单张最多 %u MB",
        "本地版本：",
        "检测中",
        "正在加载 ACP 列表，稍后会自动显示。",
        "没有会话",
        "没有匹配的文件",
        "没有匹配的项目",
        "确认删除",
        "确认删除工作区 %s ?",
        "确认重置",
        "确认重置工作区 %s ?",
        "移除项目",
        "立即引导",
        "第 %u 行",
        "第 %u 行, 第 %u 列",
        "第 %u 行, 第 %u 列 到 第 %u 行, 第 %u 列",
        "第 %u-%u 行",
        "粘贴",
        "累计 Token: %d",
        "编辑项目",
        "网关历史不可用，已仅重置本地会话历史: %s",
        "网关没有可回滚历史，已仅重置本地会话历史",
        "自动模型",
        "至少保留一个会话工具",
        "至少保留一个工具分桶",
        "规划 · %s · 计划任务",
        "解析项目失败，无法创建会话: %s",
        "识矩 %u：%s",
        "识矩技能版本",
        "请先切换到聊天模式，再生成 Git 提交信息",
        "请先打开项目，再创建新会话",
        "请先打开项目，再刷新 Git 变更",
        "请先打开项目，再提交已选 Git 变更",
        "请先打开项目，再暂存并提交 Git 变更",
        "请先选择要提交的 Git 变更",
        "请检查 MCP 表单的必填字段、类型与取值范围",
        "请等待当前回复完成后再压缩会话",
        "请等待当前回复完成后再生成 Git 提交信息",
        "请输入 worktree 英文名称",
        "调整",
        "输入子任务内容...",
        "输入窗口: %d / %d",
        "运行中",
        "进度",
        "选择本会话的推理强度",
        "选择空间",
        "重命名",
        "重置",
        "重置会话失败: %s",
        "重置失败: %s",
        "附件",
        "限制：使用保守默认值",
        "随便问点什么...",
        "预估成本: $%.4f",
    ] {
        assert!(en.contains_key(key), "missing English: {key}");
        assert!(tw.contains_key(key), "missing Traditional Chinese: {key}");
    }
}

#[test]
fn desktop_gettext_area_9_dynamic_values_switch_without_reinterpreting_arguments() {
    let raw = "中文/%n/%s";
    let message = super::format::Message::new("接口地址: %s", vec![raw.into()]);
    for (locale, prefix) in [
        (DesktopLocale::SimplifiedChinese, "接口地址: "),
        (DesktopLocale::TraditionalChinese, "介面位址: "),
        (DesktopLocale::English, "API URL: "),
    ] {
        assert_eq!(message.render(I18n::new(locale)).unwrap(), format!("{prefix}{raw}"));
    }
    assert_eq!(
        I18n::new(DesktopLocale::English)
            .format("输入窗口: %d / %d", &[12i64.into(), 100i64.into()])
            .unwrap(),
        "Input window: 12 / 100"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_9_existing_toast_renders_current_locale() {
    let mut app = crate::app::App::new().0;
    let raw = "用户/%n/%s";
    let _task = app.show_localized_toast(
        crate::app::state::ToastKind::Error,
        "加载失败：%s",
        vec![raw.into()],
    );
    let toast = app.active_toast.as_ref().unwrap();
    let message = toast.localized_message.as_ref().unwrap();
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "加载失败：用户/%n/%s"),
        (DesktopLocale::TraditionalChinese, "載入失敗：用户/%n/%s"),
        (DesktopLocale::English, "Loading failed: 用户/%n/%s"),
    ] {
        assert_eq!(message.render(I18n::new(locale)).unwrap(), expected);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_9_notifications_and_errors_retain_semantic_source() {
    let mut app = crate::app::App::new().0;
    let raw = "service 中文 %s %n";
    app.push_localized_notification("未找到 MCP 服务器：%s", vec![raw.into()]);
    assert_eq!(
        app.notifications.last().unwrap().render(I18n::new(DesktopLocale::English)),
        format!("MCP server not found: {raw}")
    );
    app.set_localized_input_error(
        super::input::ErrorField::General,
        "加载工作区列表失败: %s",
        vec![raw.into()],
    );
    app.desktop_locale = DesktopLocale::English;
    assert_eq!(
        app.render_input_error(
            super::input::ErrorField::General,
            app.error_message.as_deref().unwrap()
        ),
        format!("Failed to load workspaces: {raw}")
    );
    assert_eq!(
        app.render_input_error(super::input::ErrorField::General, "raw external error"),
        "raw external error"
    );
}
