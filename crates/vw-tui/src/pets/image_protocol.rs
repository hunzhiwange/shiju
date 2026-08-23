//! 宠物图片协议的能力探测、Kitty 控制串生成与 Sixel 帧缓存。
//!
//! 自动探测以安全为先：tmux 与 Zellij 中图片无法可靠限制在 pane 内，因此默认拒绝；
//! 支持的终端才选择 Kitty、iTerm2 本地文件扩展或 Sixel。显式协议选择仍由调用方负责。

use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use base64::Engine as _;
use base64::engine::general_purpose;
use codex_terminal_detection::Multiplexer;
use codex_terminal_detection::TerminalInfo;
use codex_terminal_detection::TerminalName;
use codex_terminal_detection::terminal_info;
use image::imageops::FilterType;

use super::sixel;

const ESC: &str = "\x1b";
const ST: &str = "\x1b\\";
const KITTY_CHUNK_SIZE: usize = 4096;
const SIXEL_CACHE_VERSION: &str = "v2";
const ITERM2_KITTY_MIN_VERSION: (u64, u64, u64) = (3, 6, 0);

/// 实际用于终端输出的图片协议变体。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageProtocol {
    Kitty,
    KittyLocalFile,
    Sixel,
}

/// 自动探测结果，失败时保留可展示的具体原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PetImageSupport {
    Supported(ImageProtocol),
    Unsupported(PetImageUnsupportedReason),
}

impl PetImageSupport {
    pub(crate) fn protocol(self) -> Option<ImageProtocol> {
        match self {
            Self::Supported(protocol) => Some(protocol),
            Self::Unsupported(_) => None,
        }
    }

    pub(crate) fn unsupported_message(self) -> Option<&'static str> {
        match self {
            Self::Supported(_) => None,
            Self::Unsupported(reason) => Some(reason.message()),
        }
    }
}

/// 自动协议不可用的安全或兼容原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PetImageUnsupportedReason {
    Tmux,
    Zellij,
    Iterm2TooOld,
    Terminal,
}

impl PetImageUnsupportedReason {
    fn message(self) -> &'static str {
        match self {
            Self::Tmux => {
                "Pets are disabled in tmux. Terminal images don’t stay pane-local in tmux and can corrupt scrollback or move between panes. Run Codex outside tmux to use pets."
            }
            Self::Zellij => {
                "Pets are disabled in Zellij. Terminal images don’t stay reliably pane-local in Zellij. Run Codex outside Zellij to use pets."
            }
            Self::Iterm2TooOld => {
                "Pets require iTerm2 3.6 or newer. Upgrade iTerm2 to use terminal pets."
            }
            Self::Terminal => {
                "Pets aren’t available in this terminal. Terminal pets need image support, and this terminal environment doesn’t expose a supported image protocol. Try a terminal with Kitty graphics or Sixel support, or run Codex outside tmux."
            }
        }
    }
}

/// 用户可配置的协议选择；`Auto` 才执行终端能力探测。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolSelection {
    Auto,
    Kitty,
    Sixel,
}

impl ProtocolSelection {
    /// 将选择解析为具体支持状态；显式选择不会被自动探测覆盖。
    pub(crate) fn resolve(self) -> PetImageSupport {
        match self {
            Self::Kitty => PetImageSupport::Supported(ImageProtocol::Kitty),
            Self::Sixel => PetImageSupport::Supported(ImageProtocol::Sixel),
            Self::Auto => detect_pet_image_support(),
        }
    }
}

impl FromStr for ProtocolSelection {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "auto" => Ok(Self::Auto),
            "kitty" => Ok(Self::Kitty),
            "sixel" => Ok(Self::Sixel),
            other => bail!("unknown protocol {other}; expected auto, kitty, or sixel"),
        }
    }
}

/// 按环境变量与终端识别结果探测宠物图片支持，复用器风险优先于协议能力。
pub(crate) fn detect_pet_image_support() -> PetImageSupport {
    if env::var_os("TMUX").is_some() || env::var_os("TMUX_PANE").is_some() {
        return PetImageSupport::Unsupported(PetImageUnsupportedReason::Tmux);
    }

    if env::var_os("ZELLIJ").is_some()
        || env::var_os("ZELLIJ_SESSION_NAME").is_some()
        || env::var_os("ZELLIJ_VERSION").is_some()
    {
        return PetImageSupport::Unsupported(PetImageUnsupportedReason::Zellij);
    }

    if env::var_os("KITTY_WINDOW_ID").is_some() {
        return PetImageSupport::Supported(ImageProtocol::Kitty);
    }

    if env::var_os("WEZTERM_EXECUTABLE").is_some() || env::var_os("WEZTERM_VERSION").is_some() {
        return PetImageSupport::Supported(ImageProtocol::Kitty);
    }

    pet_image_support_for_terminal(&terminal_info())
}

/// 根据结构化终端信息选择最安全且功能完整的协议。
fn pet_image_support_for_terminal(info: &TerminalInfo) -> PetImageSupport {
    match info.multiplexer {
        Some(Multiplexer::Tmux { .. }) => {
            return PetImageSupport::Unsupported(PetImageUnsupportedReason::Tmux);
        }
        Some(Multiplexer::Zellij { .. }) => {
            return PetImageSupport::Unsupported(PetImageUnsupportedReason::Zellij);
        }
        None => {}
    }

    if supports_iterm2_kitty_graphics(info) {
        return PetImageSupport::Supported(ImageProtocol::KittyLocalFile);
    }

    if is_iterm2_terminal(info) {
        return PetImageSupport::Unsupported(PetImageUnsupportedReason::Iterm2TooOld);
    }

    if supports_kitty_graphics(info) {
        return PetImageSupport::Supported(ImageProtocol::Kitty);
    }

    if supports_sixel(info) {
        return PetImageSupport::Supported(ImageProtocol::Sixel);
    }

    PetImageSupport::Unsupported(PetImageUnsupportedReason::Terminal)
}

/// iTerm2 仅从 3.6 起支持这里使用的 Kitty 本地文件图形能力。
fn supports_iterm2_kitty_graphics(info: &TerminalInfo) -> bool {
    is_iterm2_terminal(info)
        && version_is_at_least(
            info.version.as_deref(),
            /*minimum*/ ITERM2_KITTY_MIN_VERSION,
        )
}

/// 识别原生名称或 TERM_PROGRAM 中报告的 iTerm2。
fn is_iterm2_terminal(info: &TerminalInfo) -> bool {
    matches!(info.name, TerminalName::Iterm2)
        || terminal_field_contains(info.term_program.as_deref(), "iterm")
}

/// 识别已知 Kitty 图形终端及其常见环境字段。
fn supports_kitty_graphics(info: &TerminalInfo) -> bool {
    matches!(
        info.name,
        TerminalName::Ghostty | TerminalName::Kitty | TerminalName::WezTerm
    ) || terminal_field_contains(info.term.as_deref(), "kitty")
        || terminal_field_contains(info.term.as_deref(), "ghostty")
        || terminal_field_contains(info.term.as_deref(), "wezterm")
        || terminal_field_contains(info.term_program.as_deref(), "kitty")
        || terminal_field_contains(info.term_program.as_deref(), "ghostty")
        || terminal_field_contains(info.term_program.as_deref(), "wezterm")
}

/// 识别 Windows Terminal 及声明 Sixel、mlterm 或 foot 的终端。
fn supports_sixel(info: &TerminalInfo) -> bool {
    matches!(info.name, TerminalName::WindowsTerminal)
        || terminal_field_contains(info.term.as_deref(), "sixel")
        || terminal_field_contains(info.term.as_deref(), "mlterm")
        || terminal_field_contains(info.term.as_deref(), "foot")
}

fn terminal_field_contains(value: Option<&str>, needle: &str) -> bool {
    value.is_some_and(|value| value.to_ascii_lowercase().contains(needle))
}

fn version_is_at_least(version: Option<&str>, minimum: (u64, u64, u64)) -> bool {
    parse_dotted_version(version).is_some_and(|version| version >= minimum)
}

/// 解析最多三个纯数字点分版本分量，缺失的次版本或补丁按零处理。
fn parse_dotted_version(version: Option<&str>) -> Option<(u64, u64, u64)> {
    let version = version?;
    let mut parts = version.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;

    if parts.next().is_some() {
        return None;
    }

    Some((major, minor, patch))
}

/// 生成按图片 ID 删除 Kitty 图片的控制序列。
pub fn kitty_delete_image(image_id: u32) -> String {
    wrap_for_tmux_if_needed(&format!("{ESC}_Ga=d,d=I,i={image_id},q=2;{ST}"))
}

/// 读取 PNG、Base64 编码并按 Kitty 上限分块生成内联传输控制串。
pub fn kitty_transmit_png_with_id(
    path: &Path,
    columns: u16,
    rows: u16,
    image_id: Option<u32>,
) -> Result<String> {
    let png = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let payload = general_purpose::STANDARD.encode(png);
    let chunks = payload
        .as_bytes()
        .chunks(KITTY_CHUNK_SIZE)
        .collect::<Vec<_>>();

    // 首块携带格式、尺寸与可选图片 ID；后续块只携带继续标记和数据。
    let mut command = String::new();
    for (index, chunk) in chunks.iter().enumerate() {
        let chunk = std::str::from_utf8(chunk).context("base64 payload is not valid UTF-8")?;
        let has_more = index + 1 < chunks.len();
        let more_flag = u8::from(has_more);
        if index == 0 {
            let image_id = kitty_image_id_arg(image_id);
            command.push_str(&format!(
                "{ESC}_Ga=T,t=d,f=100,c={columns},r={rows},q=2{image_id},m={more_flag};{chunk}{ST}",
            ));
        } else {
            command.push_str(&format!("{ESC}_Gm={more_flag};{chunk}{ST}"));
        }
    }

    Ok(wrap_for_tmux_if_needed(&command))
}

/// 生成引用规范化本地 PNG 路径的 Kitty 文件传输控制串。
pub fn kitty_transmit_png_file_with_id(
    path: &Path,
    columns: u16,
    rows: u16,
    image_id: Option<u32>,
) -> Result<String> {
    let path = path
        .canonicalize()
        .with_context(|| format!("canonicalize {}", path.display()))?;
    let payload = general_purpose::STANDARD.encode(path.to_string_lossy().as_bytes());
    let image_id = kitty_image_id_arg(image_id);
    let command = format!("{ESC}_Ga=T,t=f,f=100,c={columns},r={rows},q=2{image_id};{payload}{ST}");

    Ok(wrap_for_tmux_if_needed(&command))
}

fn kitty_image_id_arg(image_id: Option<u32>) -> String {
    image_id
        .map(|image_id| format!(",i={image_id}"))
        .unwrap_or_default()
}

/// 在 tmux 内按其透传规范包装并双写 ESC；自动模式本身仍会拒绝 tmux。
fn wrap_for_tmux_if_needed(command: &str) -> String {
    if env::var_os("TMUX").is_none() {
        return command.to_string();
    }

    let escaped = command.replace(ESC, "\x1b\x1b");
    format!("{ESC}Ptmux;{escaped}{ST}")
}

/// 按目标像素高度缩放 PNG 并缓存确定性的 Sixel 编码文件。
pub fn sixel_frame(frame_path: &Path, cache_dir: &Path, height_px: u16) -> Result<PathBuf> {
    fs::create_dir_all(cache_dir).with_context(|| format!("create {}", cache_dir.display()))?;

    let stem = frame_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .context("frame path has no valid file stem")?;
    // 缓存名同时包含高度和编码版本，算法或尺寸变化不会复用旧载荷。
    let path = cache_dir.join(format!("{stem}_h{height_px}_{SIXEL_CACHE_VERSION}.six"));
    if path.exists() {
        return Ok(path);
    }

    let frame =
        image::open(frame_path).with_context(|| format!("read {}", frame_path.display()))?;
    let height = u32::from(height_px).max(1);
    let width = ((u64::from(frame.width()) * u64::from(height)) / u64::from(frame.height()))
        .try_into()
        .unwrap_or(u32::MAX)
        .max(1);
    let rgba = frame.resize(width, height, FilterType::Lanczos3).to_rgba8();
    let (width, height) = rgba.dimensions();
    let sixel = sixel::encode_rgba(&rgba.into_raw(), width, height)?;

    fs::write(&path, sixel).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;

    /// 串行测试使用的环境变量守卫，离开作用域时精确恢复原值。
    struct EnvVarGuard {
        name: &'static str,
        previous: Option<std::ffi::OsString>,
    }

    impl EnvVarGuard {
        fn new(name: &'static str, value: Option<&str>) -> Self {
            let previous = env::var_os(name);
            match value {
                Some(value) => unsafe { env::set_var(name, value) },
                None => unsafe { env::remove_var(name) },
            }
            Self { name, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match self.previous.take() {
                Some(value) => unsafe { env::set_var(self.name, value) },
                None => unsafe { env::remove_var(self.name) },
            }
        }
    }

    #[test]
    #[serial]
    // 验证 Kitty 内联传输会编码 PNG 数据并携带尺寸参数。
    fn kitty_png_transmission_encodes_inline_data() {
        let _guard = EnvVarGuard::new("TMUX", /*value*/ None);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("frame.png");
        fs::write(&path, b"png").unwrap();

        let command = kitty_transmit_png_with_id(
            &path, /*columns*/ 4, /*rows*/ 3, /*image_id*/ None,
        )
        .unwrap();

        assert!(command.starts_with("\x1b_Ga=T,t=d,f=100,c=4,r=3,q=2,m=0;"));
        assert!(command.contains("cG5n"));
        assert!(command.ends_with("\x1b\\"));
    }

    #[test]
    #[serial]
    // 验证 tmux 透传包装及 ESC 转义格式。
    fn tmux_passthrough_wraps_and_escapes_control_sequence() {
        let _guard = EnvVarGuard::new("TMUX", Some("session"));
        assert_eq!(
            wrap_for_tmux_if_needed("\x1b_Gx;\x1b\\"),
            "\x1bPtmux;\x1b\x1b_Gx;\x1b\x1b\\\x1b\\"
        );
    }

    #[test]
    // 验证配置只接受 auto、kitty 与 sixel 三个稳定值。
    fn parses_protocol_selection() {
        assert_eq!(
            "auto".parse::<ProtocolSelection>().unwrap(),
            ProtocolSelection::Auto
        );
        assert_eq!(
            "kitty".parse::<ProtocolSelection>().unwrap(),
            ProtocolSelection::Kitty
        );
        assert_eq!(
            "sixel".parse::<ProtocolSelection>().unwrap(),
            ProtocolSelection::Sixel
        );
    }

    #[test]
    #[serial]
    // 验证自动模式在 tmux 中安全禁用图片协议。
    fn auto_protocol_is_disabled_inside_tmux() {
        let _guard = EnvVarGuard::new("TMUX", Some("session"));

        assert_eq!(
            ProtocolSelection::Auto.resolve(),
            PetImageSupport::Unsupported(PetImageUnsupportedReason::Tmux)
        );
    }

    #[test]
    #[serial]
    // 验证用户显式选择协议时仍可在 tmux 中解析，责任由调用方承担。
    fn explicit_protocol_still_resolves_inside_tmux() {
        let _guard = EnvVarGuard::new("TMUX", Some("session"));

        assert_eq!(
            ProtocolSelection::Kitty.resolve(),
            PetImageSupport::Supported(ImageProtocol::Kitty)
        );
        assert_eq!(
            ProtocolSelection::Sixel.resolve(),
            PetImageSupport::Supported(ImageProtocol::Sixel)
        );
    }

    #[test]
    // 验证结构化复用器信息优先触发安全拒绝。
    fn pet_image_support_prefers_multiplexer_safety() {
        assert_eq!(
            pet_image_support_for_terminal(&terminal_info_for_test(
                TerminalName::Ghostty,
                Some(Multiplexer::Tmux { version: None }),
                Some("Ghostty"),
                /*term*/ None,
            )),
            PetImageSupport::Unsupported(PetImageUnsupportedReason::Tmux)
        );
        assert_eq!(
            pet_image_support_for_terminal(&terminal_info_for_test(
                TerminalName::Kitty,
                Some(Multiplexer::Zellij { version: None }),
                Some("kitty"),
                /*term*/ None,
            )),
            PetImageSupport::Unsupported(PetImageUnsupportedReason::Zellij)
        );
    }

    #[test]
    // 验证 iTerm2 3.6 及以上选择本地文件 Kitty 图形协议。
    fn pet_image_support_detects_iterm2_kitty_file_graphics() {
        for info in [
            terminal_info_with_version_for_test(
                TerminalName::Iterm2,
                /*multiplexer*/ None,
                Some("iTerm.app"),
                Some("3.6.10"),
                /*term*/ None,
            ),
            terminal_info_with_version_for_test(
                TerminalName::Unknown,
                /*multiplexer*/ None,
                Some("iTerm.app"),
                Some("3.6.10"),
                Some("xterm-256color"),
            ),
        ] {
            assert_eq!(
                pet_image_support_for_terminal(&info),
                PetImageSupport::Supported(ImageProtocol::KittyLocalFile)
            );
        }
    }

    #[test]
    // 验证过旧或版本不完整的 iTerm2 不启用图片协议。
    fn pet_image_support_rejects_old_iterm2_versions() {
        for info in [
            terminal_info_with_version_for_test(
                TerminalName::Iterm2,
                /*multiplexer*/ None,
                Some("iTerm.app"),
                Some("3.5.14"),
                /*term*/ None,
            ),
            terminal_info_with_version_for_test(
                TerminalName::Unknown,
                /*multiplexer*/ None,
                Some("iTerm.app"),
                /*version*/ None,
                Some("xterm-256color"),
            ),
            terminal_info_with_version_for_test(
                TerminalName::Iterm2,
                /*multiplexer*/ None,
                Some("iTerm.app"),
                Some("3.5"),
                /*term*/ None,
            ),
        ] {
            assert_eq!(
                pet_image_support_for_terminal(&info),
                PetImageSupport::Unsupported(PetImageUnsupportedReason::Iterm2TooOld)
            );
        }
    }

    #[test]
    // 验证旧 iTerm2 的用户提示包含明确升级要求。
    fn pet_image_support_old_iterm2_message_mentions_upgrade() {
        let message = PetImageSupport::Unsupported(PetImageUnsupportedReason::Iterm2TooOld)
            .unsupported_message();

        assert_eq!(
            message,
            Some("Pets require iTerm2 3.6 or newer. Upgrade iTerm2 to use terminal pets.")
        );
    }

    #[test]
    // 验证已知终端名称及环境字段均可识别 Kitty 图形支持。
    fn pet_image_support_detects_kitty_graphics_terminals() {
        for info in [
            terminal_info_for_test(
                TerminalName::Ghostty,
                /*multiplexer*/ None,
                Some("Ghostty"),
                /*term*/ None,
            ),
            terminal_info_for_test(
                TerminalName::Kitty,
                /*multiplexer*/ None,
                Some("kitty"),
                /*term*/ None,
            ),
            terminal_info_for_test(
                TerminalName::WezTerm,
                /*multiplexer*/ None,
                Some("WezTerm"),
                /*term*/ None,
            ),
            terminal_info_for_test(
                TerminalName::Unknown,
                /*multiplexer*/ None,
                /*term_program*/ None,
                Some("xterm-kitty"),
            ),
            terminal_info_for_test(
                TerminalName::Unknown,
                /*multiplexer*/ None,
                /*term_program*/ None,
                Some("wezterm"),
            ),
            terminal_info_for_test(
                TerminalName::Unknown,
                /*multiplexer*/ None,
                Some("WezTerm"),
                Some("xterm-256color"),
            ),
        ] {
            assert_eq!(
                pet_image_support_for_terminal(&info),
                PetImageSupport::Supported(ImageProtocol::Kitty)
            );
        }
    }

    #[test]
    // 验证 Windows Terminal 与常见标识可识别 Sixel 支持。
    fn pet_image_support_detects_sixel_terminals() {
        for info in [
            terminal_info_for_test(
                TerminalName::Unknown,
                /*multiplexer*/ None,
                /*term_program*/ None,
                Some("xterm-sixel"),
            ),
            terminal_info_for_test(
                TerminalName::Unknown,
                /*multiplexer*/ None,
                /*term_program*/ None,
                Some("foot"),
            ),
            terminal_info_for_test(
                TerminalName::Unknown,
                /*multiplexer*/ None,
                /*term_program*/ None,
                Some("mlterm"),
            ),
            terminal_info_for_test(
                TerminalName::WindowsTerminal,
                /*multiplexer*/ None,
                Some("WindowsTerminal"),
                Some("xterm-256color"),
            ),
        ] {
            assert_eq!(
                pet_image_support_for_terminal(&info),
                PetImageSupport::Supported(ImageProtocol::Sixel)
            );
        }
    }

    #[test]
    #[serial]
    // 验证 WezTerm 环境变量足以让环境宠物选择 Kitty 图形协议。
    fn wezterm_env_uses_kitty_graphics_for_ambient_pets() {
        let _tmux = EnvVarGuard::new("TMUX", /*value*/ None);
        let _tmux_pane = EnvVarGuard::new("TMUX_PANE", /*value*/ None);
        let _zellij = EnvVarGuard::new("ZELLIJ", /*value*/ None);
        let _zellij_session = EnvVarGuard::new("ZELLIJ_SESSION_NAME", /*value*/ None);
        let _zellij_version = EnvVarGuard::new("ZELLIJ_VERSION", /*value*/ None);
        let _kitty = EnvVarGuard::new("KITTY_WINDOW_ID", /*value*/ None);
        let _wezterm = EnvVarGuard::new("WEZTERM_VERSION", Some("20240203"));
        let _wezterm_executable = EnvVarGuard::new("WEZTERM_EXECUTABLE", /*value*/ None);

        assert_eq!(
            detect_pet_image_support(),
            PetImageSupport::Supported(ImageProtocol::Kitty)
        );
    }

    #[test]
    // 验证未知且无能力标识的终端被明确拒绝。
    fn pet_image_support_rejects_unknown_terminals() {
        assert_eq!(
            pet_image_support_for_terminal(&terminal_info_for_test(
                TerminalName::Unknown,
                /*multiplexer*/ None,
                /*term_program*/ None,
                Some("xterm-256color"),
            )),
            PetImageSupport::Unsupported(PetImageUnsupportedReason::Terminal)
        );
    }

    fn terminal_info_for_test(
        name: TerminalName,
        multiplexer: Option<Multiplexer>,
        term_program: Option<&str>,
        term: Option<&str>,
    ) -> TerminalInfo {
        terminal_info_with_version_for_test(
            name,
            multiplexer,
            term_program,
            /*version*/ None,
            term,
        )
    }

    fn terminal_info_with_version_for_test(
        name: TerminalName,
        multiplexer: Option<Multiplexer>,
        term_program: Option<&str>,
        version: Option<&str>,
        term: Option<&str>,
    ) -> TerminalInfo {
        TerminalInfo {
            name,
            term_program: term_program.map(str::to_string),
            version: version.map(str::to_string),
            term: term.map(str::to_string),
            multiplexer,
        }
    }

    #[test]
    // 验证点分版本拒绝额外分量和非数字内容。
    fn parse_dotted_version_requires_simple_numeric_components() {
        assert_eq!(parse_dotted_version(Some("3.6.10")), Some((3, 6, 10)));
        assert_eq!(parse_dotted_version(Some("3.6")), Some((3, 6, 0)));
        assert_eq!(parse_dotted_version(Some("3")), Some((3, 0, 0)));
        assert_eq!(parse_dotted_version(Some("3.6.10.1")), None);
        assert_eq!(parse_dotted_version(Some("3.6beta")), None);
        assert_eq!(parse_dotted_version(/*version*/ None), None);
    }

    #[test]
    // 验证 Sixel 缓存由内部编码器生成，无需外部命令。
    fn sixel_frame_encodes_without_external_crate() {
        let dir = tempfile::tempdir().unwrap();
        let frame_path = dir.path().join("frame.png");
        let rgba = image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]));
        rgba.save(&frame_path).unwrap();

        let sixel_path =
            sixel_frame(&frame_path, &dir.path().join("sixel"), /*height_px*/ 1).unwrap();
        let sixel = fs::read_to_string(sixel_path).unwrap();

        assert!(sixel.starts_with("\x1bP9;1;0q\"1;1;1;1"));
        assert!(sixel.contains("#224;2;100;0;0"));
        assert!(sixel.contains("#224@"));
        assert!(sixel.ends_with("\x1b\\"));
    }

    #[test]
    #[serial]
    // 验证 Kitty 本地文件传输编码规范化路径而非文件内容。
    fn kitty_file_png_transmission_encodes_local_file_reference() {
        let _guard = EnvVarGuard::new("TMUX", /*value*/ None);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("frame.png");
        fs::write(&path, b"png").unwrap();

        let command = kitty_transmit_png_file_with_id(
            &path,
            /*columns*/ 4,
            /*rows*/ 3,
            /*image_id*/ Some(7),
        )
        .unwrap();
        let path = path.canonicalize().unwrap();
        let payload = general_purpose::STANDARD.encode(path.to_string_lossy().as_bytes());

        assert_eq!(
            command,
            format!("\x1b_Ga=T,t=f,f=100,c=4,r=3,q=2,i=7;{payload}\x1b\\")
        );
    }
}
