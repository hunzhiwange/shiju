#[test]
fn global_command_shortcuts_are_case_insensitive_and_require_command_modifier() {
    use super::GlobalCommand;
    use iced::keyboard::{Key, Modifiers};

    assert!(matches!(
        super::global_command(&Key::Character("P".into()), Modifiers::COMMAND),
        Some(GlobalCommand::Search)
    ));
    assert!(matches!(
        super::global_command(&Key::Character("c".into()), Modifiers::COMMAND),
        Some(GlobalCommand::Copy)
    ));
    assert!(super::global_command(&Key::Character("p".into()), Modifiers::NONE).is_none());
    assert!(super::global_command(&Key::Character("x".into()), Modifiers::COMMAND).is_none());
}

#[test]
#[cfg(target_os = "macos")]
fn desktop_gettext_navigation_visible_copy() {
    use super::link_preview::{preview_actions, provider_content};
    use config_ui::DesktopLocale::*;
    use desktop_i18n::I18n;
    let url =
        "shiju://v1/import?resource=provider&app=claude&name=SecretName&apiKey=SecretCredential";
    for (locale, title, app, actions) in [
        (SimplifiedChinese, "供应商链接预览", "目标应用：claude", ["继续预览", "拒绝链接"]),
        (
            English,
            "Provider link preview",
            "Target application: claude",
            ["Continue preview", "Reject link"],
        ),
        (TraditionalChinese, "供應商連結預覽", "目標應用：claude", ["繼續預覽", "拒絕連結"]),
    ] {
        let i18n = I18n::new(locale);
        let (actual_title, lines) = provider_content(url, i18n);
        assert_eq!(actual_title, title);
        assert_eq!(lines[0], app);
        assert_eq!(preview_actions(i18n), actions);
        assert!(!lines.join("\n").contains("Secret"));
        assert_eq!(desktop_platform::provider_link_preview::parse(url).unwrap().app, "claude");
        let (_, invalid) = provider_content("not-a-link", i18n);
        assert_eq!(invalid, [i18n.__("供应商链接格式无效")]);
    }
}

#[test]
fn desktop_gettext_navigation_catalog_copy() {
    let sources = [
        "无法暂存启动请求",
        "已唤醒主窗口；当前尚不支持启动参数",
        "API 凭据：已提供（已隐藏）",
        "API 凭据：未提供",
        "JSON配置",
        "MCP JSON 解析失败: %s",
        "MCP 定义 %u 项，内容已隐藏。继续后须再次确认保存，逐项报告成功或失败。",
        "MCP 配置必须是 server 数组",
        "MCP 链接格式无效",
        "MCP 链接预览",
        "PATH 配置已存在，无需重复写入。",
        "VibeWindow 伙伴",
        "下载已取消，可重新下载安装包。",
        "不支持该提示词应用",
        "仓库：%s/%s\n分支：%s\n项目目录：%s",
        "供应商链接包含未知字段",
        "供应商链接应用无效",
        "供应商链接格式无效",
        "供应商链接缺少名称",
        "供应商链接缺少应用",
        "供应商链接预览",
        "保存关闭到托盘失败：%s",
        "关于 Vibe Window WebView",
        "关闭将拒绝此链接。导入确认由后续导入流程处理。",
        "关闭页面",
        "切换当前应用供应商",
        "加载更多 · 还有 %u 条",
        "升级来源未保存，本轮选择仍有效：%s",
        "发布结果与所选来源不一致",
        "可观测性",
        "右上角",
        "名称、端点、模型及其他载荷均已隐藏。链接仍在待确认队列中；未写入供应商，未读取远程配置。",
        "后台运行中",
        "复制失败：系统剪贴板不可用或访问被拒绝",
        "外观与主题",
        "外部技能链接需要确认后才保存仓库。保存不会安装技能。",
        "失败：定义格式不支持，未保存",
        "安装包已就绪：%s。%s",
        "已保存的升级来源无效，请重新选择来源。",
        "已安全替换旧 CLI；如果后台 service 正在运行，请重启 service 以加载新版本。",
        "已安装到 %s\n可执行文件: %s",
        "已更新 PATH 配置文件：",
        "归档会话",
        "当前平台不支持",
        "当前平台不支持安装 CLI 工具",
        "当前平台不支持应用更新",
        "当前平台不支持检测应用更新",
        "当前平台不支持检测更新",
        "当前平台不支持独立伙伴窗口",
        "当前平台不支持重启应用",
        "当前平台暂不支持原生桌面通知",
        "我的",
        "打开新项目",
        "技能来源链接无效",
        "技能链接预览",
        "拒绝链接",
        "授权中心",
        "提示词正在加载，请稍后重试",
        "提示词正文 Base64 无效",
        "提示词正文必须为 UTF-8",
        "提示词链接格式无效",
        "提示词链接预览",
        "搜索设置",
        "文件管理器",
        "无法关闭到托盘：%s",
        "无法取消提示词链接",
        "无法启动内置 WebView。\n\n请安装 Microsoft Edge WebView2 Runtime（Evergreen x64）。\n\n即将使用系统默认浏览器打开。",
        "无法启用托盘：%s",
        "无法打开终端：%s",
        "无法拒绝 MCP 链接",
        "无法拒绝供应商链接",
        "无法拒绝技能链接",
        "暂无项目，请先在设置中配置网关或项目。",
        "未在应用包中发现 CLI，也未设置 VIBEWINDOW_CLI_URL 可供下载。\n请下载独立 CLI 包并手动安装：\n1) 解压后将可执行文件复制到 %s\n2) 重命名为 shiju 或 shiju.exe\n3) 确保 PATH 包含 %s 后重启终端",
        "来源应用：%s",
        "桌面偏好未保存：%s",
        "正在加载会话…",
        "正在退出",
        "此工具尚未适配手机布局。",
        "没有匹配的移动设置",
        "没有可打开终端的本机仓库目录",
        "浏览器不支持开机启动",
        "浏览器不支持系统托盘",
        "浏览器使用远端网关；localhost 指当前手机。项目、模型及智能体等高级配置请切换桌面布局。",
        "点击自动更新后，应用将退出并自动替换完整应用包，无需拖拽；成功后确认启动新版。",
        "点击自动更新后，应用将退出并请求系统安装授权；Ubuntu/Debian 使用 DEB，RPM 系统使用 RPM。取消授权或安装失败时保留安装包，成功后提示启动新版。",
        "点击自动更新后，应用将退出并通过 Windows Installer 更新；请允许系统授权，成功后确认启动新版。",
        "电脑操控",
        "登录 ShiJu",
        "登录后管理你的 ShiJu 账号",
        "目标使用当前提示词页面；尚未打开时，继续将加载来源应用的提示词，再次继续即可确认导入。默认保持停用。",
        "目标应用：%s",
        "系统剪贴板不可用或访问被拒绝",
        "继续确认仓库",
        "继续确认导入",
        "继续预览",
        "网关连接与认证",
        "设置开机启动失败：%s",
        "识矩 - 定时任务",
        "识矩 - 项目管理",
        "识矩知识库",
        "请先关闭仓库管理弹窗",
        "请先关闭已有 MCP 导入弹窗",
        "请先在目标应用中打开提示词页面，等待加载完成后重试",
        "请先打开项目，再预览技能链接",
        "请先连接目标网关",
        "请先选择有效的升级来源",
        "请检查网关连接并关闭已有 MCP 导入弹窗",
        "请选择来源并检测最新发布版本。",
        "读取开机启动状态失败：%s",
        "读取文件失败，未修改剪贴板",
        "输入底部",
        "输入网址",
        "返回会话",
        "进入手机模式",
        "远端终端只能通过网关 PTY 打开",
        "退出手机模式",
        "通知线程异常退出",
        "配置字段：%u 项（内容已隐藏）",
        "链接不是供应商资源",
        "链接的 apps 与 enabled 不授予权限；不会执行命令、连接服务或分发配置。",
        "页面：%s",
        "页面：（空）",
        "项目与会话",
        "项目管理器",
    ];
    for source in [
        include_str!("../../desktop-i18n/src/locales/en.json"),
        include_str!("../../desktop-i18n/src/locales/zh-TW.json"),
    ] {
        let catalog = desktop_i18n::catalog::parse_catalog(source).expect("strict catalog");
        for key in sources {
            let translation = catalog.get(key).expect(key);
            assert!(!translation.trim().is_empty(), "{key}");
            desktop_i18n::format::validate_translation(key, translation).expect(key);
        }
    }
}
