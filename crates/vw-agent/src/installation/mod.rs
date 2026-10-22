//! 安装信息、版本探测与自升级入口。
//!
//! 本模块负责识别当前二进制的安装渠道，查询对应渠道的最新版本，并按安装方式执行升级命令。
//! 所有外部命令与网络请求都收敛在本文件内，便于审计升级路径和错误处理边界。

use crate::app::agent::bus;
use crate::app::agent::flag;
use crate::app::agent::util::log;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::cmp::Ordering;
use std::env;
use std::fmt;
use std::path::Path;
use std::process::{Command, Output};
use std::sync::LazyLock;

static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {
    let mut tags = Map::new();
    tags.insert("service".to_string(), Value::String("installation".to_string()));
    log::create(Some(tags))
});

pub mod event {
    //! 安装状态相关的事件定义。
    //!
    //! 这些事件用于通知运行时或 UI 当前安装已更新，或已有可用更新。

    use crate::app::agent::bus;

    /// 安装已完成更新时发布的事件。
    pub const UPDATED: bus::Definition = bus::Definition { r#type: "installation.updated" };
    /// 发现可用更新时发布的事件。
    pub const UPDATE_AVAILABLE: bus::Definition =
        bus::Definition { r#type: "installation.update-available" };
}

/// 当前安装版本与远端最新版本的聚合信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
    /// 当前正在运行的 VibeWindow 版本。
    pub version: String,
    /// 当前安装渠道可获取的最新版本。
    pub latest: String,
}

/// VibeWindow 的安装来源。
///
/// 升级命令依赖安装来源选择不同包管理器或安装脚本；无法可靠识别时使用 `Unknown`，
/// 调用方应把它视为不支持自动升级的状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// 通过官方 shell 安装脚本安装。
    Curl,
    /// 通过 npm 全局包安装。
    Npm,
    /// 通过 yarn 全局包安装。
    Yarn,
    /// 通过 pnpm 全局包安装。
    Pnpm,
    /// 通过 bun 全局包安装。
    Bun,
    /// 通过 Homebrew formula 安装。
    Brew,
    /// 通过 Scoop bucket 安装。
    Scoop,
    /// 通过 Chocolatey 包安装。
    Choco,
    /// 无法识别或当前不支持的安装来源。
    Unknown,
}

impl Method {
    /// 返回用于日志、错误消息和事件负载的稳定安装来源名称。
    pub fn as_str(self) -> &'static str {
        match self {
            Method::Curl => "curl",
            Method::Npm => "npm",
            Method::Yarn => "yarn",
            Method::Pnpm => "pnpm",
            Method::Bun => "bun",
            Method::Brew => "brew",
            Method::Scoop => "scoop",
            Method::Choco => "choco",
            Method::Unknown => "unknown",
        }
    }
}

/// 升级命令执行失败时保留的标准错误信息。
#[derive(Debug)]
pub struct UpgradeFailedError {
    /// 升级工具输出的错误文本；调用方可直接展示给用户。
    pub stderr: String,
}

impl fmt::Display for UpgradeFailedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.stderr)
    }
}

impl std::error::Error for UpgradeFailedError {}

/// 安装与升级流程的统一错误类型。
#[derive(Debug)]
pub enum Error {
    /// 文件系统、进程启动等 I/O 错误。
    Io(std::io::Error),
    /// 外部命令输出无法按 UTF-8 解码。
    Utf8(std::string::FromUtf8Error),
    /// 远端版本响应无法反序列化。
    Json(serde_json::Error),
    /// HTTP 请求或状态码错误。
    Http(reqwest::Error),
    /// 升级命令已执行但退出状态表示失败。
    UpgradeFailed(UpgradeFailedError),
    /// 当前平台、安装方式或输入不支持。
    Unsupported(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{}", e),
            Error::Utf8(e) => write!(f, "{}", e),
            Error::Json(e) => write!(f, "{}", e),
            Error::Http(e) => write!(f, "{}", e),
            Error::UpgradeFailed(e) => write!(f, "{}", e),
            Error::Unsupported(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Io(value)
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(value: std::string::FromUtf8Error) -> Self {
        Error::Utf8(value)
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Error::Json(value)
    }
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        Error::Http(value)
    }
}

/// 返回当前构建注入的版本号。
///
/// 当环境变量缺失时返回 `local`，表示这是本地开发或未打包运行环境。
pub fn version() -> String {
    env::var("VIBEWINDOW_VERSION").unwrap_or_else(|_| "local".to_string())
}

/// 返回当前发布渠道。
///
/// 当环境变量缺失时返回 `local`，避免把本地构建误判为稳定发布渠道。
pub fn channel() -> String {
    env::var("VIBEWINDOW_CHANNEL").unwrap_or_else(|_| "local".to_string())
}

/// 生成安装模块发起 HTTP 请求时使用的 User-Agent。
///
/// 返回值包含渠道、版本和客户端标识，方便服务端区分不同分发来源。
pub fn user_agent() -> String {
    format!("vibewindow/{}/{}/{}", channel(), version(), flag::vibewindow_client())
}

/// 查询当前安装版本和对应渠道的最新版本。
///
/// # 错误
///
/// 当安装方式探测、远端版本查询或响应解析失败时返回 [`Error`]。
pub fn info() -> Result<Info, Error> {
    Ok(Info { version: version(), latest: latest(None)? })
}

/// 判断当前运行渠道是否不是稳定 `latest`。
pub fn is_preview() -> bool {
    channel() != "latest"
}

/// 判断当前运行环境是否为本地开发构建。
pub fn is_local() -> bool {
    channel() == "local"
}

/// 探测当前二进制的安装来源。
///
/// 探测先查看可执行文件路径，再查询各包管理器的全局安装清单。路径命中优先处理，
/// 是为了让脚本安装的独立二进制无需依赖包管理器存在。
pub fn method() -> Method {
    let exe = env::current_exe().ok();
    if let Some(p) = exe.as_ref().and_then(|p| p.to_str()) {
        if p.contains(
            Path::new(vw_config_types::paths::PROD_HOME_CONFIG_DIR_NAME)
                .join("bin")
                .to_string_lossy()
                .as_ref(),
        ) {
            return Method::Curl;
        }
        if p.contains(Path::new(".local").join("bin").to_string_lossy().as_ref()) {
            return Method::Curl;
        }
    }
    let exec = exe
        .as_ref()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let mut checks: Vec<(Method, &str, Vec<&str>)> = vec![
        (Method::Npm, "npm", vec!["npm", "list", "-g", "--depth=0"]),
        (Method::Yarn, "yarn", vec!["yarn", "global", "list"]),
        (Method::Pnpm, "pnpm", vec!["pnpm", "list", "-g", "--depth=0"]),
        (Method::Bun, "bun", vec!["bun", "pm", "ls", "-g"]),
        (Method::Brew, "brew", vec!["brew", "list", "--formula", "vibewindow"]),
        (Method::Scoop, "scoop", vec!["scoop", "list", "vibewindow"]),
        (Method::Choco, "choco", vec!["choco", "list", "--limit-output", "vibewindow"]),
    ];

    checks.sort_by(|a, b| {
        let a_matches = exec.contains(a.1);
        let b_matches = exec.contains(b.1);
        // 如果可执行文件名暗示某个包管理器，优先验证该来源，减少多包管理器共存时的误判。
        match (a_matches, b_matches) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => Ordering::Equal,
        }
    });

    for (m, _name, cmd) in checks {
        let output = run_quiet(cmd);
        let installed_name = matches!(m, Method::Brew | Method::Choco | Method::Scoop);
        let needle = if installed_name { "vibewindow" } else { "vibewindow-ai" };
        if output.contains(needle) {
            return m;
        }
    }

    Method::Unknown
}

/// 使用指定安装来源升级到目标版本。
///
/// # 参数
///
/// - `method`: 已探测或由调用方指定的安装来源。
/// - `target`: 目标版本号，传递给对应包管理器或安装脚本。
///
/// # 错误
///
/// 当安装来源不支持、命令无法启动、命令退出失败或输出无法处理时返回 [`Error`]。
pub fn upgrade(method: Method, target: &str) -> Result<(), Error> {
    let result = match method {
        Method::Curl => run_shell(
            "curl -fsSL https://vibewindow.huododo.com/install | bash",
            Some(&[("VERSION", target)]),
        )?,
        Method::Npm => {
            run_cmd(&["npm", "install", "-g", &format!("vibewindow-ai@{}", target)], None)?
        }
        Method::Pnpm => {
            run_cmd(&["pnpm", "install", "-g", &format!("vibewindow-ai@{}", target)], None)?
        }
        Method::Bun => {
            run_cmd(&["bun", "install", "-g", &format!("vibewindow-ai@{}", target)], None)?
        }
        Method::Brew => {
            let formula = get_brew_formula()?;
            if formula.contains('/') {
                // tap formula 需要先更新 tap 仓库；显式关闭 Homebrew 自动更新以避免升级流程不可控地变慢。
                let cmd = format!(
                    "brew tap anomalyco/tap && cd \"$(brew --repo anomalyco/tap)\" && git pull --ff-only && brew upgrade {}",
                    formula
                );
                run_shell(&cmd, Some(&[("HOMEBREW_NO_AUTO_UPDATE", "1")]))?
            } else {
                run_cmd(&["brew", "upgrade", &formula], Some(&[("HOMEBREW_NO_AUTO_UPDATE", "1")]))?
            }
        }
        Method::Choco => {
            run_shell(&format!("echo Y | choco upgrade vibewindow --version={}", target), None)?
        }
        Method::Scoop => run_cmd(&["scoop", "install", &format!("vibewindow@{}", target)], None)?,
        Method::Yarn | Method::Unknown => {
            return Err(Error::Unsupported(format!("Unknown method: {}", method.as_str())));
        }
    };

    if !result.status.success() {
        let stderr = if method == Method::Choco {
            // Chocolatey 未提权时的错误输出不稳定，使用固定文本让 UI 能给出可理解的提示。
            "not running from an elevated command shell".to_string()
        } else {
            String::from_utf8_lossy(&result.stderr).to_string()
        };
        return Err(Error::UpgradeFailed(UpgradeFailedError { stderr }));
    }

    LOGGER.info(
        "upgraded",
        Some(extra_from_output(
            &result,
            [
                ("method", Value::String(method.as_str().to_string())),
                ("target", Value::String(target.to_string())),
            ],
        )),
    );

    let _ = env::current_exe()
        .ok()
        .and_then(|p| p.to_str().map(|s| s.to_string()))
        .map(|exe| run_cmd(&[&exe, "--version"], None));

    // 升级成功后发布事件即可；通知失败不应反向标记升级失败。
    let _ = bus::publish(event::UPDATED, serde_json::json!({ "version": target }), None);
    Ok(())
}

/// 查询指定或自动探测安装方式对应的最新可用版本。
///
/// # 参数
///
/// - `install_method`: 可选安装来源；为空时会调用 [`method`] 自动探测。
///
/// # 错误
///
/// 当包管理器查询、HTTP 请求或远端响应解析失败时返回 [`Error`]。
pub fn latest(install_method: Option<Method>) -> Result<String, Error> {
    let detected = install_method.unwrap_or_else(method);
    let ch = channel();

    if detected == Method::Brew {
        let formula = get_brew_formula()?;
        if formula.contains('/') {
            // tap formula 不一定会出现在官方 formula API 中，需要通过本地 brew info 读取版本。
            let out = run_cmd(&["brew", "info", "--json=v2", &formula], None)?;
            let txt = String::from_utf8(out.stdout)?;
            let v: serde_json::Value = serde_json::from_str(&txt)?;
            let stable = v
                .get("formulae")
                .and_then(|x| x.get(0))
                .and_then(|x| x.get("versions"))
                .and_then(|x| x.get("stable"))
                .and_then(|x| x.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| {
                    Error::Unsupported(format!(
                        "Could not detect version for tap formula: {}",
                        formula
                    ))
                })?;
            return Ok(stable);
        }

        #[derive(Deserialize)]
        struct BrewApi {
            versions: BrewVersions,
        }
        #[derive(Deserialize)]
        struct BrewVersions {
            stable: String,
        }
        let api: BrewApi =
            http_get_json("https://formulae.brew.sh/api/formula/vibewindow.json", None)?;
        return Ok(api.versions.stable);
    }

    if matches!(detected, Method::Npm | Method::Bun | Method::Pnpm) {
        let registry = npm_registry()?;
        #[derive(Deserialize)]
        struct NpmDistTag {
            version: String,
        }
        let url = format!("{}/vibewindow-ai/{}", registry, ch);
        let v: NpmDistTag = http_get_json(&url, None)?;
        return Ok(v.version);
    }

    if detected == Method::Choco {
        #[derive(Deserialize)]
        struct ChocoRoot {
            d: ChocoD,
        }
        #[derive(Deserialize)]
        struct ChocoD {
            results: Vec<ChocoResult>,
        }
        #[derive(Deserialize)]
        struct ChocoResult {
            #[serde(rename = "Version")]
            version: String,
        }

        let url = "https://community.chocolatey.org/api/v2/Packages?$filter=Id%20eq%20%27vibewindow%27%20and%20IsLatestVersion&$select=Version";
        let v: ChocoRoot =
            http_get_json(url, Some(&[("Accept", "application/json;odata=verbose")]))?;
        return Ok(v.d.results.get(0).map(|r| r.version.clone()).unwrap_or_default());
    }

    if detected == Method::Scoop {
        #[derive(Deserialize)]
        struct Scoop {
            version: String,
        }
        let url =
            "https://raw.githubusercontent.com/ScoopInstaller/Main/master/bucket/vibewindow.json";
        let v: Scoop = http_get_json(url, Some(&[("Accept", "application/json")]))?;
        return Ok(v.version);
    }

    #[derive(Deserialize)]
    struct GithubRelease {
        tag_name: String,
    }
    let gh: GithubRelease =
        http_get_json("https://api.github.com/repos/anomalyco/vibewindow/releases/latest", None)?;
    Ok(gh.tag_name.trim_start_matches('v').to_string())
}

fn npm_registry() -> Result<String, Error> {
    let out = run_cmd(&["npm", "config", "get", "registry"], None)?;
    let raw = String::from_utf8(out.stdout)?.trim().to_string();
    let mut reg = if raw.is_empty() { "https://registry.npmjs.org".to_string() } else { raw };
    if reg.ends_with('/') {
        // 后续 URL 拼接会自行插入分隔符，先规范化 registry 末尾避免双斜杠。
        reg.pop();
    }
    Ok(reg)
}

fn get_brew_formula() -> Result<String, Error> {
    let tap = run_cmd(&["brew", "list", "--formula", "anomalyco/tap/vibewindow"], None)?;
    let tap_txt = String::from_utf8(tap.stdout)?;
    if tap_txt.contains("vibewindow") {
        return Ok("anomalyco/tap/vibewindow".to_string());
    }
    let core = run_cmd(&["brew", "list", "--formula", "vibewindow"], None)?;
    let core_txt = String::from_utf8(core.stdout)?;
    if core_txt.contains("vibewindow") {
        return Ok("vibewindow".to_string());
    }
    Ok("vibewindow".to_string())
}

fn run_quiet(cmd: Vec<&str>) -> String {
    if cmd.is_empty() {
        return String::new();
    }
    let mut c = Command::new(cmd[0]);
    if cmd.len() > 1 {
        c.args(&cmd[1..]);
    }
    match c.output() {
        Ok(out) => {
            let mut s = String::new();
            s.push_str(&String::from_utf8_lossy(&out.stdout));
            s.push_str(&String::from_utf8_lossy(&out.stderr));
            s
        }
        Err(_) => String::new(),
    }
}

fn run_cmd(args: &[&str], envs: Option<&[(&str, &str)]>) -> Result<Output, Error> {
    if args.is_empty() {
        return Err(Error::Unsupported("missing command".to_string()));
    }
    let mut cmd = Command::new(args[0]);
    if args.len() > 1 {
        cmd.args(&args[1..]);
    }
    if let Some(envs) = envs {
        for (k, v) in envs {
            cmd.env(k, v);
        }
    }
    Ok(cmd.output()?)
}

fn run_shell(script: &str, envs: Option<&[(&str, &str)]>) -> Result<Output, Error> {
    #[cfg(windows)]
    let shell = "cmd";
    #[cfg(windows)]
    let args: [&str; 2] = ["/C", script];

    #[cfg(not(windows))]
    let shell = "bash";
    #[cfg(not(windows))]
    let args: [&str; 2] = ["-lc", script];

    let mut cmd = Command::new(shell);
    cmd.args(args);
    if let Some(envs) = envs {
        for (k, v) in envs {
            cmd.env(k, v);
        }
    }
    Ok(cmd.output()?)
}

#[cfg(not(target_arch = "wasm32"))]
fn http_get_json<T: for<'de> Deserialize<'de>>(
    url: &str,
    headers: Option<&[(&str, &str)]>,
) -> Result<T, Error> {
    let client = reqwest::blocking::Client::builder().user_agent(user_agent()).build()?;
    let mut req = client.get(url);
    if let Some(headers) = headers {
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
    }
    let resp = req.send()?.error_for_status()?;
    Ok(resp.json::<T>()?)
}

#[cfg(target_arch = "wasm32")]
fn http_get_json<T: for<'de> Deserialize<'de>>(
    _url: &str,
    _headers: Option<&[(&str, &str)]>,
) -> Result<T, Error> {
    Err(Error::Unsupported("http_get_json not supported in WASM".to_string()))
}

fn extra<const N: usize>(pairs: [(&'static str, Value); N]) -> Map<String, Value> {
    let mut m = Map::new();
    for (k, v) in pairs {
        m.insert(k.to_string(), v);
    }
    m
}

fn extra_from_output<const N: usize>(
    out: &Output,
    base: [(&'static str, Value); N],
) -> Map<String, Value> {
    let mut m = extra(base);
    m.insert(
        "exit_code".to_string(),
        Value::Number(serde_json::Number::from(out.status.code().unwrap_or(-1) as i64)),
    );
    m.insert("stdout".to_string(), Value::String(String::from_utf8_lossy(&out.stdout).to_string()));
    m.insert("stderr".to_string(), Value::String(String::from_utf8_lossy(&out.stderr).to_string()));
    m
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
