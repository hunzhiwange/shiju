//! WASM 沙箱运行时 — 通过 `wasmi` 实现进程内工具隔离。
//!
//! 提供基于能力的沙箱机制，无需 Docker 或外部运行时。
//! 每个 WASM 模块运行时具有：
//! - **燃料限制**：防止无限循环（每条指令消耗 1 燃料）
//! - **内存上限**：可配置的每个模块内存上限
//! - **无文件系统访问**：默认情况下，工具仅进行纯计算
//! - **无网络访问**：除非配置了明确允许的主机白名单
//!
//! # 功能门控
//! 此模块仅在启用 `--features runtime-wasm` 时编译。
//! 默认的 VibeWindow 二进制文件排除它以保持 4.6 MB 的体积目标。

use super::traits::RuntimeAdapter;
use crate::app::agent::config::{
    WasmCapabilityEscalationMode, WasmModuleHashPolicy, WasmRuntimeConfig,
};
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

/// WASM 沙箱运行时 — 在隔离的解释器中执行工具模块。
///
/// 该结构体封装了 WASM 模块的加载、验证和执行逻辑，
/// 提供了安全的沙箱环境来运行不受信任的代码。
#[derive(Debug, Clone)]
pub struct WasmRuntime {
    /// WASM 运行时配置
    config: WasmRuntimeConfig,
    /// 可选的工作空间目录路径
    workspace_dir: Option<PathBuf>,
}

/// WASM 模块执行结果。
///
/// 包含执行后的各种输出信息和度量数据。
#[derive(Debug, Clone)]
pub struct WasmExecutionResult {
    /// 从模块捕获的标准输出（如果使用 WASI）
    pub stdout: String,
    /// 从模块捕获的标准错误
    pub stderr: String,
    /// 退出码（0 表示成功）
    pub exit_code: i32,
    /// 执行期间消耗的燃料
    pub fuel_consumed: u64,
    /// 已执行模块字节的 SHA-256 摘要（十六进制格式）
    pub module_sha256: String,
}

/// 授予 WASM 工具模块的能力。
///
/// 定义了模块在沙箱中可以执行的操作。
/// 默认情况下，所有能力都是受限的。
#[derive(Debug, Clone, Default)]
pub struct WasmCapabilities {
    /// 允许从工作空间读取文件
    pub read_workspace: bool,
    /// 允许向工作空间写入文件
    pub write_workspace: bool,
    /// 允许访问的 HTTP 主机列表（空表示无网络访问）
    pub allowed_hosts: Vec<String>,
    /// 自定义燃料覆盖值（0 表示使用配置默认值）
    pub fuel_override: u64,
    /// 自定义内存覆盖值（MB，0 表示使用配置默认值）
    pub memory_override_mb: u64,
}

impl WasmRuntime {
    /// 最大允许内存限制：4 GB（4096 MB）
    ///
    /// 这是 32 位 WASM 的安全上限
    const MAX_MEMORY_MB: u64 = 4096;

    /// 最大燃料限制：100 亿
    ///
    /// 防止配置过大的燃料值导致潜在的资源耗尽
    const MAX_FUEL_LIMIT: u64 = 10_000_000_000;

    /// 使用给定配置创建新的 WASM 运行时实例。
    ///
    /// # 参数
    /// - `config`: WASM 运行时配置
    ///
    /// # 返回
    /// 配置好的 WasmRuntime 实例，但未绑定工作空间
    ///
    /// # 示例
    /// ```ignore
    /// let config = WasmRuntimeConfig::default();
    /// let runtime = WasmRuntime::new(config);
    /// ```
    pub fn new(config: WasmRuntimeConfig) -> Self {
        Self { config, workspace_dir: None }
    }

    /// 创建绑定到特定工作空间目录的 WASM 运行时。
    ///
    /// # 参数
    /// - `config`: WASM 运行时配置
    /// - `workspace_dir`: 工作空间目录的路径
    ///
    /// # 返回
    /// 绑定了工作空间的 WasmRuntime 实例
    ///
    /// # 示例
    /// ```ignore
    /// let config = WasmRuntimeConfig::default();
    /// let workspace = PathBuf::from("/path/to/workspace");
    /// let runtime = WasmRuntime::with_workspace(config, workspace);
    /// ```
    pub fn with_workspace(config: WasmRuntimeConfig, workspace_dir: PathBuf) -> Self {
        Self { config, workspace_dir: Some(workspace_dir) }
    }

    /// 检查 WASM 运行时功能在当前构建中是否可用。
    ///
    /// # 返回
    /// 如果启用了 `runtime-wasm` 功能则返回 `true`，否则返回 `false`
    pub fn is_available() -> bool {
        cfg!(feature = "runtime-wasm")
    }

    /// 验证 WASM 配置是否存在常见错误配置。
    ///
    /// 执行一系列验证检查，包括：
    /// - 燃料限制的有效性（大于 0 且不超过最大值）
    /// - 内存限制的有效性（大于 0 且不超过 4 GB）
    /// - 模块大小限制的有效性
    /// - 工具目录配置的有效性
    /// - 允许主机列表的格式验证
    /// - 模块哈希策略的一致性检查
    ///
    /// # 返回
    /// - `Ok(())`: 配置有效
    /// - `Err(...)`: 配置无效，包含具体错误信息
    ///
    /// # 错误
    /// 当配置项不符合安全要求时返回错误
    pub fn validate_config(&self) -> Result<()> {
        // 验证燃料限制必须大于 0
        if self.config.fuel_limit == 0 {
            bail!("runtime.wasm.fuel_limit must be > 0");
        }
        // 验证燃料限制不能超过安全上限
        if self.config.fuel_limit > Self::MAX_FUEL_LIMIT {
            bail!(
                "runtime.wasm.fuel_limit of {} exceeds safety ceiling of {}",
                self.config.fuel_limit,
                Self::MAX_FUEL_LIMIT
            );
        }
        // 验证内存限制必须大于 0
        if self.config.memory_limit_mb == 0 {
            bail!("runtime.wasm.memory_limit_mb must be > 0");
        }
        // 验证内存限制不能超过 4 GB（32 位 WASM 的限制）
        if self.config.memory_limit_mb > Self::MAX_MEMORY_MB {
            bail!(
                "runtime.wasm.memory_limit_mb of {} exceeds the 4 GB safety limit for 32-bit WASM",
                self.config.memory_limit_mb
            );
        }
        // 验证模块大小限制必须大于 0
        if self.config.max_module_size_mb == 0 {
            bail!("runtime.wasm.max_module_size_mb must be > 0");
        }
        // 验证工具目录不能为空
        if self.config.tools_dir.is_empty() {
            bail!("runtime.wasm.tools_dir cannot be empty");
        }
        // 如果要求工具目录必须相对于工作空间，则进行路径遍历检查
        if self.config.security.require_workspace_relative_tools_dir {
            let tools_dir_path = Path::new(&self.config.tools_dir);
            // 工具目录不能是绝对路径
            if tools_dir_path.is_absolute() {
                bail!("runtime.wasm.tools_dir must be a workspace-relative path");
            }
            // 工具目录不能包含父目录引用（防止路径遍历攻击）
            if tools_dir_path.components().any(|c| matches!(c, Component::ParentDir)) {
                bail!("runtime.wasm.tools_dir must not contain '..' path traversal");
            }
        }
        // 验证并规范化允许的主机列表
        let _ = self.normalize_hosts_with_policy(
            self.config.allowed_hosts.iter().map(String::as_str),
            "runtime.wasm.allowed_hosts",
        )?;
        // 验证并规范化模块 SHA-256 引脚
        let normalized_pins = self.normalize_module_sha256_pins()?;
        // 如果启用了强制哈希策略，则必须至少有一个模块引脚
        if matches!(self.config.security.module_hash_policy, WasmModuleHashPolicy::Enforce)
            && normalized_pins.is_empty()
        {
            bail!(
                "runtime.wasm.security.module_hash_policy='enforce' requires at least one module pin in runtime.wasm.security.module_sha256"
            );
        }
        Ok(())
    }

    /// 解析 WASM 工具目录的绝对路径。
    ///
    /// # 参数
    /// - `workspace_dir`: 工作空间目录路径
    ///
    /// # 返回
    /// 工作空间目录与配置的工具目录组合后的绝对路径
    pub fn tools_dir(&self, workspace_dir: &Path) -> PathBuf {
        workspace_dir.join(&self.config.tools_dir)
    }

    /// 从配置默认值构建能力对象。
    ///
    /// # 返回
    /// 使用配置中的默认值初始化的 WasmCapabilities 实例
    pub fn default_capabilities(&self) -> WasmCapabilities {
        WasmCapabilities {
            read_workspace: self.config.allow_workspace_read,
            write_workspace: self.config.allow_workspace_write,
            allowed_hosts: self.config.allowed_hosts.clone(),
            fuel_override: 0,
            memory_override_mb: 0,
        }
    }

    /// 获取调用的有效燃料限制。
    ///
    /// 如果能力对象中指定了覆盖值且大于 0，则使用覆盖值与配置限制的较小值；
    /// 否则使用配置中的默认燃料限制。
    ///
    /// # 参数
    /// - `caps`: 能力对象引用
    ///
    /// # 返回
    /// 有效的燃料限制值
    pub fn effective_fuel(&self, caps: &WasmCapabilities) -> u64 {
        if caps.fuel_override > 0 {
            caps.fuel_override.min(self.config.fuel_limit)
        } else {
            self.config.fuel_limit
        }
    }

    /// 获取有效内存限制（字节）。
    ///
    /// 如果能力对象中指定了覆盖值且大于 0，则使用覆盖值与配置限制的较小值；
    /// 否则使用配置中的默认内存限制。结果转换为字节数。
    ///
    /// # 参数
    /// - `caps`: 能力对象引用
    ///
    /// # 返回
    /// 有效的内存限制字节数
    pub fn effective_memory_bytes(&self, caps: &WasmCapabilities) -> u64 {
        let mb = if caps.memory_override_mb > 0 {
            caps.memory_override_mb.min(self.config.memory_limit_mb)
        } else {
            self.config.memory_limit_mb
        };
        // 使用 saturating_mul 防止溢出
        mb.saturating_mul(1024 * 1024)
    }

    /// 验证模块名称的有效性。
    ///
    /// 模块名称必须满足以下条件：
    /// - 非空
    /// - 长度不超过 128 个字符
    /// - 仅包含字母、数字、下划线和连字符
    ///
    /// # 参数
    /// - `module_name`: 要验证的模块名称
    ///
    /// # 返回
    /// - `Ok(())`: 模块名称有效
    /// - `Err(...)`: 模块名称无效
    fn validate_module_name(module_name: &str) -> Result<()> {
        // 模块名称不能为空
        if module_name.is_empty() {
            bail!("WASM module name cannot be empty");
        }
        // 模块名称长度不能超过 128 字符
        if module_name.len() > 128 {
            bail!("WASM module name is too long (max 128 chars): {module_name}");
        }
        // 模块名称只能包含字母、数字、下划线和连字符
        if !module_name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-') {
            bail!(
                "WASM module name '{module_name}' contains invalid characters; \
                 allowed set is [A-Za-z0-9_-]"
            );
        }
        Ok(())
    }

    /// 规范化主机字符串并进行格式验证。
    ///
    /// 执行以下验证：
    /// - 非空
    /// - 不允许通配符
    /// - 不能包含 scheme、路径或查询字符串
    /// - 不能以点号或连字符开头/结尾
    /// - 字符集限制
    /// - 端口格式验证
    ///
    /// # 参数
    /// - `host`: 要规范化的主机字符串
    ///
    /// # 返回
    /// 规范化后的主机字符串（小写、去除空白）
    fn normalize_host(host: &str) -> Result<String> {
        // 去除空白并转换为小写
        let normalized = host.trim().to_ascii_lowercase();
        // 主机不能为空
        if normalized.is_empty() {
            bail!("runtime.wasm.allowed_hosts contains an empty entry");
        }
        // 不允许通配符
        if normalized == "*" || normalized.contains('*') {
            bail!(
                "runtime.wasm.allowed_hosts entry '{host}' is invalid; wildcard hosts are not allowed"
            );
        }
        // 不能包含 scheme、路径或查询字符串
        if normalized.contains("://")
            || normalized.contains('/')
            || normalized.contains('?')
            || normalized.contains('#')
        {
            bail!(
                "runtime.wasm.allowed_hosts entry '{host}' must be host[:port] only (no scheme/path/query)"
            );
        }
        // 不能以点号开头或结尾
        if normalized.starts_with('.') || normalized.ends_with('.') {
            bail!("runtime.wasm.allowed_hosts entry '{host}' must not start/end with '.'");
        }
        // 不能以连字符开头或结尾
        if normalized.starts_with('-') || normalized.ends_with('-') {
            bail!("runtime.wasm.allowed_hosts entry '{host}' must not start/end with '-'");
        }
        // 只允许字母、数字、点号、连字符和冒号
        if !normalized
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == ':')
        {
            bail!("runtime.wasm.allowed_hosts entry '{host}' contains invalid characters");
        }

        // 如果包含冒号，验证 host:port 格式
        if let Some((host_part, port_part)) = normalized.rsplit_once(':') {
            // 支持 host:port 格式，但拒绝格式错误的 host 部分
            if host_part.is_empty()
                || port_part.is_empty()
                || !port_part.chars().all(|c| c.is_ascii_digit())
            {
                bail!("runtime.wasm.allowed_hosts entry '{host}' has invalid port format");
            }
            // 只允许一个冒号（端口分隔符）
            if host_part.contains(':') {
                bail!("runtime.wasm.allowed_hosts entry '{host}' has too many ':' separators");
            }
        }

        Ok(normalized)
    }

    /// 根据策略规范化主机列表。
    ///
    /// 遍历所有主机条目，进行规范化处理。
    /// 如果启用了严格主机验证，无效条目将导致错误；
    /// 否则无效条目将被忽略并记录警告。
    ///
    /// # 参数
    /// - `hosts`: 主机字符串迭代器
    /// - `source`: 用于错误报告的来源标识
    ///
    /// # 返回
    /// 规范化后的主机集合（去重）
    fn normalize_hosts_with_policy<'a, I>(&self, hosts: I, source: &str) -> Result<BTreeSet<String>>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut normalized = BTreeSet::new();
        for host in hosts {
            match Self::normalize_host(host) {
                Ok(value) => {
                    normalized.insert(value);
                }
                // 如果启用了严格验证，无效主机导致错误
                Err(err) if self.config.security.strict_host_validation => return Err(err),
                // 否则记录警告并忽略无效主机
                Err(err) => {
                    tracing::warn!(
                        host,
                        source,
                        error = %err,
                        "Ignoring invalid WASM host entry because runtime.wasm.security.strict_host_validation=false"
                    );
                }
            }
        }
        Ok(normalized)
    }

    /// 规范化 SHA-256 引脚字符串。
    ///
    /// 验证引脚字符串是否为有效的 64 字符十六进制 SHA-256 摘要。
    ///
    /// # 参数
    /// - `module_name`: 模块名称（用于错误报告）
    /// - `raw`: 原始的 SHA-256 字符串
    ///
    /// # 返回
    /// 规范化后的 SHA-256 字符串（小写、去除空白）
    fn normalize_sha256_pin(module_name: &str, raw: &str) -> Result<String> {
        // 去除空白并转换为小写
        let normalized = raw.trim().to_ascii_lowercase();
        // SHA-256 必须是 64 个十六进制字符
        if normalized.len() != 64 || !normalized.chars().all(|ch| ch.is_ascii_hexdigit()) {
            bail!(
                "runtime.wasm.security.module_sha256.{module_name} must be a 64-character hex SHA-256 digest"
            );
        }
        Ok(normalized)
    }

    /// 规范化所有模块的 SHA-256 引脚。
    ///
    /// 遍历配置中的所有模块引脚，验证模块名称和哈希值的格式。
    ///
    /// # 返回
    /// 模块名称到规范化 SHA-256 哈希的映射
    fn normalize_module_sha256_pins(&self) -> Result<BTreeMap<String, String>> {
        let mut normalized = BTreeMap::new();
        for (module_name, digest) in &self.config.security.module_sha256 {
            // 验证模块名称格式
            Self::validate_module_name(module_name)?;
            // 规范化并存储引脚
            normalized
                .insert(module_name.clone(), Self::normalize_sha256_pin(module_name, digest)?);
        }
        Ok(normalized)
    }

    /// 检查模块完整性。
    ///
    /// 根据配置的哈希策略验证模块的 SHA-256 哈希：
    /// - `Disabled`: 不检查
    /// - `Warn`: 不匹配时记录警告
    /// - `Enforce`: 不匹配时返回错误
    ///
    /// # 参数
    /// - `module_name`: 模块名称
    /// - `wasm_bytes`: WASM 模块的字节内容
    ///
    /// # 返回
    /// 模块的实际 SHA-256 哈希值（十六进制）
    fn check_module_integrity(&self, module_name: &str, wasm_bytes: &[u8]) -> Result<String> {
        // 计算模块的实际 SHA-256 哈希
        let digest = hex::encode(Sha256::digest(wasm_bytes));
        let normalized_pins = self.normalize_module_sha256_pins()?;
        // 根据哈希策略执行不同的验证逻辑
        match self.config.security.module_hash_policy {
            // 禁用模式：不进行任何验证
            WasmModuleHashPolicy::Disabled => {}
            // 警告模式：不匹配时记录警告
            WasmModuleHashPolicy::Warn => match normalized_pins.get(module_name) {
                Some(expected) if expected == &digest => {}
                Some(expected) => {
                    tracing::warn!(
                        module = module_name,
                        expected_sha256 = expected,
                        actual_sha256 = digest,
                        "WASM module SHA-256 mismatch (warn mode)"
                    );
                }
                None => {
                    tracing::warn!(
                        module = module_name,
                        actual_sha256 = digest,
                        "WASM module has no SHA-256 pin configured (warn mode)"
                    );
                }
            },
            // 强制模式：不匹配时返回错误
            WasmModuleHashPolicy::Enforce => match normalized_pins.get(module_name) {
                Some(expected) if expected == &digest => {}
                Some(expected) => {
                    bail!(
                        "WASM module integrity mismatch for '{module_name}': expected sha256={expected}, got sha256={digest}"
                    );
                }
                None => {
                    bail!(
                        "WASM module '{module_name}' is missing required SHA-256 pin (runtime.wasm.security.module_hash_policy='enforce')"
                    );
                }
            },
        }
        Ok(digest)
    }

    /// 验证能力请求并返回有效的能力对象。
    ///
    /// 根据配置的能力升级模式处理能力请求：
    /// - `Deny`: 如果请求的能力超过配置限制，返回错误
    /// - `Clamp`: 将请求的能力限制在配置范围内，并记录警告
    ///
    /// # 参数
    /// - `caps`: 请求的能力对象
    ///
    /// # 返回
    /// 验证后的有效能力对象
    fn validate_capabilities(&self, caps: &WasmCapabilities) -> Result<WasmCapabilities> {
        // 规范化配置中的默认主机
        let default_hosts = self.normalize_hosts_with_policy(
            self.config.allowed_hosts.iter().map(String::as_str),
            "runtime.wasm.allowed_hosts",
        )?;
        // 规范化请求的主机
        let requested_hosts = self.normalize_hosts_with_policy(
            caps.allowed_hosts.iter().map(String::as_str),
            "wasm invocation allowed_hosts",
        )?;

        // 根据能力升级模式处理
        match self.config.security.capability_escalation_mode {
            // 拒绝模式：任何超出限制的请求都导致错误
            WasmCapabilityEscalationMode::Deny => {
                // 检查读取工作空间权限
                if caps.read_workspace && !self.config.allow_workspace_read {
                    bail!(
                        "WASM capability escalation blocked: read_workspace requested but runtime.wasm.allow_workspace_read is false"
                    );
                }
                // 检查写入工作空间权限
                if caps.write_workspace && !self.config.allow_workspace_write {
                    bail!(
                        "WASM capability escalation blocked: write_workspace requested but runtime.wasm.allow_workspace_write is false"
                    );
                }
                // 检查燃料覆盖是否超出限制
                if caps.fuel_override > self.config.fuel_limit {
                    bail!(
                        "WASM capability escalation blocked: fuel_override={} exceeds runtime.wasm.fuel_limit={}",
                        caps.fuel_override,
                        self.config.fuel_limit
                    );
                }
                // 检查内存覆盖是否超出限制
                if caps.memory_override_mb > self.config.memory_limit_mb {
                    bail!(
                        "WASM capability escalation blocked: memory_override_mb={} exceeds runtime.wasm.memory_limit_mb={}",
                        caps.memory_override_mb,
                        self.config.memory_limit_mb
                    );
                }
                // 检查请求的主机是否都在允许列表中
                for host in &requested_hosts {
                    if !default_hosts.contains(host) {
                        bail!(
                            "WASM capability escalation blocked: host '{host}' is not in runtime.wasm.allowed_hosts"
                        );
                    }
                }
                // 返回验证通过的能力
                Ok(WasmCapabilities {
                    read_workspace: caps.read_workspace,
                    write_workspace: caps.write_workspace,
                    allowed_hosts: requested_hosts.into_iter().collect(),
                    fuel_override: caps.fuel_override,
                    memory_override_mb: caps.memory_override_mb,
                })
            }
            // 钳制模式：将超出限制的请求限制在配置范围内
            WasmCapabilityEscalationMode::Clamp => {
                // 构建有效的能力对象，将所有超出限制的值钳制到配置限制
                let mut effective = WasmCapabilities {
                    read_workspace: caps.read_workspace && self.config.allow_workspace_read,
                    write_workspace: caps.write_workspace && self.config.allow_workspace_write,
                    // 只保留同时在请求列表和默认列表中的主机
                    allowed_hosts: requested_hosts
                        .intersection(&default_hosts)
                        .cloned()
                        .collect::<Vec<_>>(),
                    fuel_override: if caps.fuel_override > self.config.fuel_limit {
                        self.config.fuel_limit
                    } else {
                        caps.fuel_override
                    },
                    memory_override_mb: if caps.memory_override_mb > self.config.memory_limit_mb {
                        self.config.memory_limit_mb
                    } else {
                        caps.memory_override_mb
                    },
                };

                // 记录被钳制的权限变更警告
                if caps.read_workspace && !effective.read_workspace {
                    tracing::warn!(
                        "Clamped WASM read_workspace request because runtime.wasm.allow_workspace_read=false"
                    );
                }
                if caps.write_workspace && !effective.write_workspace {
                    tracing::warn!(
                        "Clamped WASM write_workspace request because runtime.wasm.allow_workspace_write=false"
                    );
                }
                if caps.fuel_override > self.config.fuel_limit {
                    tracing::warn!(
                        requested = caps.fuel_override,
                        allowed = self.config.fuel_limit,
                        "Clamped WASM fuel_override to runtime.wasm.fuel_limit"
                    );
                }
                if caps.memory_override_mb > self.config.memory_limit_mb {
                    tracing::warn!(
                        requested = caps.memory_override_mb,
                        allowed = self.config.memory_limit_mb,
                        "Clamped WASM memory_override_mb to runtime.wasm.memory_limit_mb"
                    );
                }
                if effective.allowed_hosts.len() != requested_hosts.len() {
                    tracing::warn!(
                        requested = requested_hosts.len(),
                        allowed = effective.allowed_hosts.len(),
                        "Clamped WASM allowed_hosts to runtime.wasm.allowed_hosts"
                    );
                }

                // 对允许的主机进行排序以确保一致性
                effective.allowed_hosts.sort();
                Ok(effective)
            }
        }
    }

    /// 从工具目录执行 WASM 模块。
    ///
    /// 这是运行沙箱工具代码的主要入口点。
    /// 模块必须导出 `_start` 函数（WASI 约定）或
    /// 自定义的 `run` 函数（无参数，返回 i32）。
    ///
    /// # 参数
    /// - `module_name`: 要执行的模块名称（不带 .wasm 扩展名）
    /// - `workspace_dir`: 工作空间目录路径
    /// - `caps`: 授予模块的能力对象
    ///
    /// # 返回
    /// 包含执行结果的 WasmExecutionResult 对象
    ///
    /// # 错误
    /// 在以下情况下返回错误：
    /// - 配置验证失败
    /// - 模块名称无效
    /// - 模块文件不存在或无法读取
    /// - 模块完整性检查失败
    /// - 模块解析或实例化失败
    /// - 模块缺少必需的入口点函数
    #[cfg(feature = "runtime-wasm")]
    pub fn execute_module(
        &self,
        module_name: &str,
        workspace_dir: &Path,
        caps: &WasmCapabilities,
    ) -> Result<WasmExecutionResult> {
        use wasmi::{Engine, Linker, Module, Store};

        // 验证运行时配置
        self.validate_config()?;
        // 验证模块名称格式
        Self::validate_module_name(module_name)?;
        // 验证并获取有效能力
        let effective_caps = self.validate_capabilities(caps)?;

        // 解析并规范化模块路径
        let tools_path = self.tools_dir(workspace_dir);
        // 检查工具目录是否存在
        if !tools_path.exists() {
            bail!("WASM tools directory does not exist: {}", tools_path.display());
        }
        // 如果配置要求，检查工具目录是否为符号链接
        if self.config.security.reject_symlink_tools_dir {
            let tools_meta = std::fs::symlink_metadata(&tools_path).with_context(|| {
                format!("Failed to inspect WASM tools directory metadata: {}", tools_path.display())
            })?;
            if tools_meta.file_type().is_symlink() {
                bail!("WASM tools directory must not be a symlink: {}", tools_path.display());
            }
        }
        // 获取工具目录的规范路径
        let canonical_tools_path = std::fs::canonicalize(&tools_path).with_context(|| {
            format!("Failed to canonicalize WASM tools directory: {}", tools_path.display())
        })?;
        // 确保是目录
        if !canonical_tools_path.is_dir() {
            bail!("WASM tools path is not a directory: {}", canonical_tools_path.display());
        }
        // 构建模块完整路径
        let module_path = canonical_tools_path.join(format!("{module_name}.wasm"));

        // 检查模块文件是否存在
        if !module_path.exists() {
            bail!(
                "WASM module not found: {} (looked in {})",
                module_name,
                canonical_tools_path.display()
            );
        }
        // 如果配置要求，检查模块文件是否为符号链接
        if self.config.security.reject_symlink_modules {
            let module_symlink_meta =
                std::fs::symlink_metadata(&module_path).with_context(|| {
                    format!("Failed to inspect WASM module metadata: {}", module_path.display())
                })?;
            if module_symlink_meta.file_type().is_symlink() {
                bail!("WASM module path must not be a symlink: {}", module_path.display());
            }
        }
        // 获取模块文件的规范路径
        let canonical_module_path = std::fs::canonicalize(&module_path).with_context(|| {
            format!("Failed to canonicalize WASM module path: {}", module_path.display())
        })?;
        // 确保模块路径在工具目录内（防止路径遍历攻击）
        if !canonical_module_path.starts_with(&canonical_tools_path) {
            bail!("WASM module path escapes tools directory: {}", canonical_module_path.display());
        }
        // 确保文件扩展名是 .wasm
        if canonical_module_path.extension().and_then(|ext| ext.to_str()) != Some("wasm") {
            bail!("WASM module path must end with .wasm: {}", canonical_module_path.display());
        }
        // 确保是文件而不是目录
        if !canonical_module_path.is_file() {
            bail!("WASM module path is not a file: {}", canonical_module_path.display());
        }

        // 检查模块文件大小是否超过限制
        let module_size_bytes = std::fs::metadata(&canonical_module_path)
            .with_context(|| {
                format!("Failed to read WASM module metadata: {}", canonical_module_path.display())
            })?
            .len();
        let max_size_bytes = self.config.max_module_size_mb * 1024 * 1024;
        if module_size_bytes > max_size_bytes {
            bail!(
                "WASM module {} is {} MB — exceeds configured {} MB safety limit",
                module_name,
                module_size_bytes / (1024 * 1024),
                self.config.max_module_size_mb
            );
        }

        // 读取模块字节内容
        let wasm_bytes = std::fs::read(&canonical_module_path).with_context(|| {
            format!("Failed to read WASM module: {}", canonical_module_path.display())
        })?;
        // 检查模块完整性
        let module_sha256 = self.check_module_integrity(module_name, &wasm_bytes)?;

        // 配置引擎并启用燃料计量
        let mut engine_config = wasmi::Config::default();
        engine_config.consume_fuel(true);
        let engine = Engine::new(&engine_config);

        // 解析并验证模块
        let module = Module::new(&engine, &wasm_bytes[..])
            .with_context(|| format!("Failed to parse WASM module: {module_name}"))?;

        // 创建带有燃料预算的存储
        let mut store = Store::new(&engine, ());
        let fuel = self.effective_fuel(&effective_caps);
        if fuel > 0 {
            store.set_fuel(fuel).with_context(|| {
                format!("Failed to set fuel budget ({fuel}) for module: {module_name}")
            })?;
        }

        // 链接宿主函数（最小化 — 纯沙箱）
        let linker = Linker::new(&engine);

        // 实例化模块
        let instance = linker
            .instantiate_and_start(&mut store, &module)
            .with_context(|| format!("Failed to instantiate WASM module: {module_name}"))?;

        // 查找导出的入口点函数（优先 run，其次 _start）
        let run_fn = instance
            .get_typed_func::<(), i32>(&store, "run")
            .or_else(|_| instance.get_typed_func::<(), i32>(&store, "_start"))
            .with_context(|| {
                format!(
                    "WASM module '{module_name}' must export a 'run() -> i32' or '_start() -> i32' function"
                )
            })?;

        // 执行模块并记录燃料消耗
        let fuel_before = store.get_fuel().unwrap_or(0);
        let exit_code = match run_fn.call(&mut store, ()) {
            Ok(code) => code,
            Err(e) => {
                // 检查是否因燃料耗尽而失败（无限循环保护）
                let fuel_after = store.get_fuel().unwrap_or(0);
                if fuel_after == 0 && fuel > 0 {
                    return Ok(WasmExecutionResult {
                        stdout: String::new(),
                        stderr: format!(
                            "WASM module '{module_name}' exceeded fuel limit ({fuel} ticks) — likely an infinite loop"
                        ),
                        exit_code: -1,
                        fuel_consumed: fuel,
                        module_sha256: module_sha256.clone(),
                    });
                }
                bail!("WASM execution error in '{module_name}': {e}");
            }
        };
        let fuel_after = store.get_fuel().unwrap_or(0);
        // 计算实际消耗的燃料
        let fuel_consumed = fuel_before.saturating_sub(fuel_after);

        Ok(WasmExecutionResult {
            stdout: String::new(), // 尚未实现 WASI stdout — 纯计算
            stderr: String::new(),
            exit_code,
            fuel_consumed,
            module_sha256,
        })
    }

    /// 当 `runtime-wasm` 功能未启用时的存根实现。
    ///
    /// # 参数
    /// - `module_name`: 请求的模块名称
    /// - `_workspace_dir`: 未使用的工作空间目录
    /// - `_caps`: 未使用的能力对象
    ///
    /// # 返回
    /// 始终返回错误，说明 WASM 运行时不可用
    #[cfg(not(feature = "runtime-wasm"))]
    pub fn execute_module(
        &self,
        module_name: &str,
        _workspace_dir: &Path,
        _caps: &WasmCapabilities,
    ) -> Result<WasmExecutionResult> {
        bail!(
            "WASM runtime is not available in this build. \
             Rebuild with `cargo build --features runtime-wasm` to enable WASM sandbox support. \
             Module requested: {module_name}"
        )
    }

    /// 列出工具目录中可用的 WASM 工具模块。
    ///
    /// 扫描工具目录，查找所有 .wasm 文件，
    /// 并返回符合命名规范的模块名称列表。
    ///
    /// # 参数
    /// - `workspace_dir`: 工作空间目录路径
    ///
    /// # 返回
    /// 有效模块名称的排序列表
    pub fn list_modules(&self, workspace_dir: &Path) -> Result<Vec<String>> {
        let tools_path = self.tools_dir(workspace_dir);
        // 如果工具目录不存在，返回空列表
        if !tools_path.exists() {
            return Ok(Vec::new());
        }

        let mut modules = Vec::new();
        // 遍历工具目录中的所有文件
        for entry in std::fs::read_dir(&tools_path)
            .with_context(|| format!("Failed to read tools dir: {}", tools_path.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            // 只处理 .wasm 文件
            if path.extension().is_some_and(|ext| ext == "wasm") {
                if let Some(stem) = path.file_stem() {
                    let module_name = stem.to_string_lossy().to_string();
                    // 只添加名称有效的模块
                    if Self::validate_module_name(&module_name).is_ok() {
                        modules.push(module_name);
                    }
                }
            }
        }
        // 对结果排序以确保输出一致性
        modules.sort();
        Ok(modules)
    }
}

impl RuntimeAdapter for WasmRuntime {
    /// 返回自身的 Any 引用，用于运行时类型检查。
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    /// 返回运行时名称。
    fn name(&self) -> &str {
        "wasm"
    }

    /// 检查运行时是否提供 shell 访问。
    ///
    /// WASM 沙箱不提供 shell 访问 — 这是它的核心特性
    fn has_shell_access(&self) -> bool {
        false
    }

    /// 检查运行时是否提供文件系统访问。
    ///
    /// 取决于配置中的工作空间读写权限
    fn has_filesystem_access(&self) -> bool {
        self.config.allow_workspace_read || self.config.allow_workspace_write
    }

    /// 返回存储路径。
    ///
    /// 如果配置了工作空间目录，则在工作空间下创建模式配置目录；
    /// 否则在当前目录下创建模式配置目录。
    fn storage_path(&self) -> PathBuf {
        self.workspace_dir.as_ref().map_or_else(
            || PathBuf::from(vw_config_types::paths::HOME_CONFIG_DIR_NAME),
            |w| w.join(vw_config_types::paths::HOME_CONFIG_DIR_NAME),
        )
    }

    /// 检查运行时是否支持长时间运行的进程。
    ///
    /// WASM 模块是短生命周期的调用，不是守护进程
    fn supports_long_running(&self) -> bool {
        false
    }

    /// 返回内存预算（字节）。
    fn memory_budget(&self) -> u64 {
        self.config.memory_limit_mb.saturating_mul(1024 * 1024)
    }

    /// 构建 shell 命令（WASM 运行时不支持）。
    ///
    /// # 参数
    /// - `_command`: 未使用的命令字符串
    /// - `_workspace_dir`: 未使用的工作空间目录
    ///
    /// # 返回
    /// 始终返回错误，说明 WASM 运行时不支持 shell 命令
    #[cfg(not(target_arch = "wasm32"))]
    fn build_shell_command(
        &self,
        _command: &str,
        _workspace_dir: &Path,
    ) -> anyhow::Result<tokio::process::Command> {
        bail!(
            "WASM runtime does not support shell commands. \
             Use `execute_module()` to run WASM tools, or switch to runtime.kind = \"native\" for shell access."
        )
    }
}

// ── 测试 ───────────────────────────────────────────────────────

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
