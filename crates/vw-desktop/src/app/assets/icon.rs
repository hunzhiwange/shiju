/// 应用程序图标枚举
///
/// 定义了应用程序中所有可用的图标类型，包括：
/// - UI 交互图标（光标、箭头、菜单等）
/// - 编辑操作图标（保存、撤销、重做、复制等）
/// - 文件类型图标（Rust、TypeScript、Python 等）
/// - 应用程序图标（VSCode、Cursor、Zed 等 IDE/编辑器）
///
/// 这些图标用于在 UI 中表示不同的功能和状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    /// 光标/指针图标
    Cursor,
    /// 四向箭头（调整大小）
    Arrows,
    /// 移动箭头
    ArrowsMove,
    /// 全屏箭头
    ArrowsFullscreen,
    /// 向上箭头
    ArrowUp,
    /// 向下箭头
    ArrowDown,
    /// 无序列表图标
    ListUl,
    /// 链接图标
    Link,
    /// 45度倾斜链接图标
    Link45Deg,
    /// 贝塞尔曲线图标
    Bezier,
    /// 向右箭头（展开/导航）
    ChevronRight,
    /// 向左箭头（折叠/返回）
    ChevronLeft,
    /// 向下箭头（展开/下拉）
    ChevronDown,
    /// 垃圾桶/删除图标
    Trash,
    /// 正方形图标
    Square,
    /// 半选正方形图标
    SquareHalf,
    /// 勾选正方形图标
    CheckSquare,
    /// 圆形图标
    Circle,
    /// 星形图标
    Star,
    /// 三角形图标
    Triangle,
    /// 菱形图标
    Diamond,
    /// 五边形图标
    Pentagon,
    /// 六边形图标
    Hexagon,
    /// 胶囊图标
    Capsule,
    /// 平行四边形图标（复用）
    Parallelogram,
    /// 梯形图标（复用）
    Trapezoid,
    /// 速度计/性能图标
    Speedometer2,
    /// 加号/新增图标
    Plus,
    /// 铅笔/编辑图标
    Pencil,
    /// 橡皮擦图标
    Eraser,
    /// 眼睛/显示图标
    Eye,
    /// 斜杠眼睛/隐藏图标
    EyeSlash,
    /// 图片图标
    Image,
    /// 云下载图标
    CloudDownload,
    /// 笔/绘图图标
    Pen,
    /// 油漆桶/填充图标
    PaintBucket,
    /// 文字/字体图标
    Type,
    /// 文本窗口布局图标
    LayoutTextWindow,
    /// 文本文件图标
    FileText,
    /// 文件标记加号/新建文件图标
    FileEarmarkPlus,
    /// 手指索引/选择图标
    HandIndex,
    /// 滑块/设置图标
    Sliders,
    /// 调色板图标
    Palette,
    /// 键盘图标
    Keyboard,
    /// 齿轮/设置图标
    Gear,
    /// 宽齿轮连接图标
    GearWideConnected,
    /// 网关/网络存储图标
    HddNetwork,
    /// 花括号/代码块图标
    Braces,
    /// 盒子/组件图标
    Box,
    /// 文件图标
    File,
    /// 打开的文件夹图标
    FolderOpen,
    /// 聊天文本填充图标
    ChatTextFill,
    /// 日志/笔记本图标
    Journals,
    /// 云上传图标
    CloudUpload,
    /// 应用程序 Logo
    Logo,
    /// 侧边栏布局图标
    LayoutSidebar,
    /// 反向侧边栏布局图标
    LayoutSidebarReverse,
    /// 代码图标
    Code,
    /// 粗体文字图标
    TypeBold,
    /// 斜体文字图标
    TypeItalic,
    /// 下划线文字图标
    TypeUnderline,
    /// 删除线文字图标
    TypeStrikethrough,
    /// 左对齐文字图标
    TextLeft,
    /// 居中对齐文字图标
    TextCenter,
    /// 右对齐文字图标
    TextRight,
    /// 顶部对齐图标
    AlignTop,
    /// 垂直居中对齐图标
    AlignMiddle,
    /// 底部对齐图标
    AlignBottom,
    /// 边框样式图标
    BorderStyle,
    /// 保存图标
    Save,
    /// 逆时针箭头/撤销图标
    ArrowCounterClockwise,
    /// 顺时针箭头/重做图标
    ArrowClockwise,
    /// 搜索图标
    Search,
    /// 机器人/小宠物图标
    Robot,
    /// 循环箭头/刷新图标
    ArrowRepeat,
    /// X/关闭图标
    X,
    /// 终端图标
    Terminal,
    /// 1x2 网格图标
    Grid1x2,
    /// 多列布局图标
    Columns,
    /// 水平对称图标
    SymmetryHorizontal,
    /// 垂直对称图标
    SymmetryVertical,
    /// 向上箭头（折叠）
    ChevronUp,
    /// 剪贴板图标
    Clipboard,
    /// 勾选/完成图标
    Check,
    /// 问号圆圈/帮助图标
    QuestionCircle,
    /// 返回图标
    Back,
    /// 主页图标
    Home,
    /// Git 分支图标
    GitBranch,
    /// 时钟/历史图标
    Clock,
    /// 盾牌锁定/安全权限图标
    ShieldLock,
    /// Rust 语言文件图标
    Rust,
    /// TypeScript 语言文件图标
    Typescript,
    /// JavaScript 语言文件图标
    Javascript,
    /// JSON 文件图标
    Json,
    /// TOML 配置文件图标
    Toml,
    /// YAML 配置文件图标
    Yaml,
    /// Markdown 文件图标
    Markdown,
    /// HTML 文件图标
    Html,
    /// CSS 样式文件图标
    Css,
    /// Python 语言文件图标
    Python,
    /// Go 语言文件图标
    Go,
    /// 控制台图标
    Console,
    /// 文档图标
    Document,
    /// 二维码图标
    QrCode,
    /// 剪刀/剪切图标
    Scissors,
    /// 复制图标
    Copy,
    /// GitHub Copilot 应用图标
    AppGitHubCopilot,
    /// VS Code 应用图标
    AppVSCode,
    /// Cursor 应用图标
    AppCursor,
    /// Auggie 应用图标
    AppAuggie,
    /// Claude Code 应用图标
    AppClaudeCode,
    /// Codex 应用图标
    AppCodex,
    /// Factory Droid 应用图标
    AppFactoryDroid,
    /// Gemini CLI 应用图标
    AppGeminiCli,
    /// KiloCode 应用图标
    AppKiloCode,
    /// Kimi Code 应用图标
    AppKimiCode,
    /// Zed 应用图标
    AppZed,
    /// Sublime Text 应用图标
    AppSublimeText,
    /// Ghostty 终端应用图标
    AppGhostty,
    /// iTerm2 终端应用图标
    AppITerm2,
    /// PowerShell 应用图标
    AppPowerShell,
    /// Android Studio 应用图标
    AppAndroidStudio,
    /// Antigravity 应用图标
    AppAntigravity,
    /// 文件资源管理器应用图标
    AppFileExplorer,
    /// Finder 应用图标（macOS）
    AppFinder,
    /// Terminal 应用图标（macOS）
    AppTerminal,
    /// TextMate 应用图标
    AppTextMate,
    /// OpenClaw 应用图标
    AppOpenClaw,
    /// OpenCode 应用图标
    AppOpenCode,
    /// pi-acp 应用图标
    AppPi,
    /// Qoder 应用图标
    AppQoder,
    /// Qwen Code 应用图标
    AppQwenCode,
    /// Trae 应用图标
    AppTrae,
    /// Xcode 应用图标
    AppXcode,
    /// Kiro 应用图标
    AppKiro,
    /// Windsurf 应用图标
    AppWindsurf,
    /// 三个垂直点/更多选项图标
    DotsThreeVertical,
    /// 全屏图标
    Fullscreen,
    /// 退出全屏图标
    FullscreenExit,
}
#[cfg(test)]
#[path = "icon_tests.rs"]
mod icon_tests;
