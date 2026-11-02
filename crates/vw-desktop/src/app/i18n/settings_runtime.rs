use vw_config_types::ui::DesktopLocale;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Text {
    RuntimeKind,
    EnvironmentHelp,
    Auto,
    Enabled,
    Disabled,
    EnableReasoning,
    ReasoningHelp,
    Default,
    Minimal,
    Low,
    Medium,
    High,
    ExtraHigh,
    ReasoningLevel,
    LevelHelp,
    DockerConfig,
    DockerHelp,
    Image,
    ImageHelp,
    Network,
    NetworkHelp,
    MemoryLimit,
    UnlimitedHelp,
    Unlimited,
    CpuLimit,
    CpuExampleHelp,
    CpuExample,
    ReadOnly,
    ReadOnlyHelp,
    Mount,
    MountHelp,
    AllowedRoots,
    RootsHelp,
    PathsHint,
    RootsHint,
    RuntimeConfig,
    RuntimeIntro,
    Basic,
    KindHelp,
    Overrides,
    OverridesHelp,
}
pub fn text(locale: DesktopLocale, key: Text) -> &'static str {
    let values = match key {
        Text::RuntimeKind => ("运行时类型", "執行環境類型", "Runtime type"),
        Text::EnvironmentHelp => (
            "选择 native 或 docker 执行环境。",
            "選擇 native 或 docker 執行環境。",
            "Choose the native or docker execution environment.",
        ),
        Text::Auto => ("自动", "自動", "Automatic"),
        Text::Enabled => ("启用", "啟用", "Enabled"),
        Text::Disabled => ("禁用", "停用", "Disabled"),
        Text::EnableReasoning => ("启用推理", "啟用推理", "Enable reasoning"),
        Text::ReasoningHelp => (
            "覆盖 runtime.reasoning_enabled 的兼容配置。",
            "覆寫 runtime.reasoning_enabled 的相容設定。",
            "Override the runtime.reasoning_enabled compatibility setting.",
        ),
        Text::Default => ("默认", "預設", "Default"),
        Text::Minimal => ("最小", "最小", "Minimal"),
        Text::Low => ("低", "低", "Low"),
        Text::Medium => ("中", "中", "Medium"),
        Text::High => ("高", "高", "High"),
        Text::ExtraHigh => ("超高", "超高", "Extra high"),
        Text::ReasoningLevel => ("推理级别", "推理等級", "Reasoning level"),
        Text::LevelHelp => (
            "兼容别名 runtime.reasoning_level。",
            "相容別名 runtime.reasoning_level。",
            "Compatibility alias runtime.reasoning_level.",
        ),
        Text::DockerConfig => ("Docker 配置", "Docker 設定", "Docker configuration"),
        Text::DockerHelp => (
            "当 kind=docker 时生效，用于控制镜像、资源限制和工作区挂载。",
            "於 kind=docker 時生效，控制映像、資源限制與工作區掛載。",
            "Applies when kind=docker; controls the image, resource limits and workspace mounts.",
        ),
        Text::Image => ("镜像 (image)", "映像 (image)", "Image (image)"),
        Text::ImageHelp => ("默认执行镜像。", "預設執行映像。", "Default execution image."),
        Text::Network => ("网络模式 (network)", "網路模式 (network)", "Network mode (network)"),
        Text::NetworkHelp => {
            ("控制容器的网络模式。", "控制容器的網路模式。", "Control the container network mode.")
        }
        Text::MemoryLimit => (
            "内存限制 MB (memory_limit_mb)",
            "記憶體限制 MB (memory_limit_mb)",
            "Memory limit MB (memory_limit_mb)",
        ),
        Text::UnlimitedHelp => {
            ("留空表示不限制。", "留空表示不限制。", "Leave blank for no limit.")
        }
        Text::Unlimited => ("留空表示不限制", "留空表示不限制", "Leave blank for no limit"),
        Text::CpuLimit => ("CPU 限制 (cpu_limit)", "CPU 限制 (cpu_limit)", "CPU limit (cpu_limit)"),
        Text::CpuExampleHelp => {
            ("例如 1 / 1.5 / 2。", "例如 1 / 1.5 / 2。", "For example 1 / 1.5 / 2.")
        }
        Text::CpuExample => ("例如 1 / 1.5 / 2", "例如 1 / 1.5 / 2", "For example 1 / 1.5 / 2"),
        Text::ReadOnly => (
            "只读根文件系统 (read_only_rootfs)",
            "唯讀根檔案系統 (read_only_rootfs)",
            "Read-only root filesystem (read_only_rootfs)",
        ),
        Text::ReadOnlyHelp => (
            "限制容器根文件系统为只读。",
            "將容器根檔案系統限制為唯讀。",
            "Make the container root filesystem read-only.",
        ),
        Text::Mount => (
            "挂载工作区 (mount_workspace)",
            "掛載工作區 (mount_workspace)",
            "Mount workspace (mount_workspace)",
        ),
        Text::MountHelp => (
            "决定是否把工作区挂载进容器。",
            "決定是否將工作區掛載至容器。",
            "Choose whether to mount the workspace in the container.",
        ),
        Text::AllowedRoots => (
            "允许的工作区根目录 (allowed_workspace_roots)",
            "允許的工作區根目錄 (allowed_workspace_roots)",
            "Allowed workspace roots (allowed_workspace_roots)",
        ),
        Text::RootsHelp => (
            "仅放行明确需要挂载的根目录。",
            "僅允許明確需要掛載的根目錄。",
            "Allow only the root directories that need mounting.",
        ),
        Text::PathsHint => {
            ("逗号或换行分隔路径", "以逗號或換行分隔路徑", "Paths separated by commas or newlines")
        }
        Text::RootsHint => (
            "建议仅放行明确需要挂载的根目录。",
            "建議僅允許明確需要掛載的根目錄。",
            "Allow only the root directories that need mounting.",
        ),
        Text::RuntimeConfig => ("运行时配置", "執行環境設定", "Runtime configuration"),
        Text::RuntimeIntro => (
            "配置 native、docker 执行环境及推理覆盖项。",
            "設定 native、docker 執行環境及推理覆寫項目。",
            "Configure native and docker execution environments and reasoning overrides.",
        ),
        Text::Basic => ("基础行为", "基本行為", "Basic behavior"),
        Text::KindHelp => ("运行时类型。", "執行環境類型。", "Runtime type."),
        Text::Overrides => ("推理覆盖", "推理覆寫", "Reasoning overrides"),
        Text::OverridesHelp => (
            "该区域用于配置 runtime.reasoning_enabled 与兼容别名 runtime.reasoning_level。",
            "設定 runtime.reasoning_enabled 與相容別名 runtime.reasoning_level。",
            "Configure runtime.reasoning_enabled and the compatibility alias runtime.reasoning_level.",
        ),
    };
    match locale {
        DesktopLocale::SimplifiedChinese => values.0,
        DesktopLocale::TraditionalChinese => values.1,
        DesktopLocale::English => values.2,
    }
}
