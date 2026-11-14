//! 凭据泄漏检测模块 —— 用于扫描出站内容中的敏感信息。
//!
//! 本模块在消息发送前扫描出站内容，检测潜在的凭据泄漏，防止 API 密钥、令牌、
//! 密码等敏感值被意外泄露。
//!
//! # 核心功能
//!
//! - 检测各类 API 密钥（Stripe、OpenAI、Anthropic、Google、GitHub 等）
//! - 检测 AWS 凭据（Access Key ID 和 Secret Access Key）
//! - 检测通用密钥模式（密码、令牌、秘密值）
//! - 检测 PEM 格式的私钥（RSA、EC、OpenSSH）
//! - 检测 JWT 令牌
//! - 检测数据库连接 URL（PostgreSQL、MySQL、MongoDB、Redis）
//!
//! # 使用示例
//!
//! ```ignore
//! use vibe_agent::security::LeakDetector;
//!
//! let detector = LeakDetector::new();
//! let result = detector.scan("My API key is sk-1234567890abcdef");
//!
//! match result {
//!     LeakResult::Clean => println!("内容安全"),
//!     LeakResult::Detected { patterns, redacted } => {
//!         println!("检测到泄漏: {:?}", patterns);
//!         println!("脱敏后内容: {}", redacted);
//!     }
//! }
//! ```
//!
//! # 来源声明
//!
//! 贡献自 RustyClaw 项目（MIT 许可证）。

use regex::Regex;
use std::sync::OnceLock;

/// 泄漏检测结果枚举。
///
/// 表示对内容进行泄漏扫描后的结果，可能为"无泄漏"或"检测到泄漏"两种状态。
///
/// # 变体
///
/// - `Clean`: 未检测到任何敏感信息泄漏
/// - `Detected`: 检测到潜在的敏感信息泄漏，包含泄漏模式描述和脱敏后的内容
#[derive(Debug, Clone)]
pub enum LeakResult {
    /// 无泄漏 —— 内容安全，可以发送。
    Clean,

    /// 检测到泄漏 —— 包含检测到的泄漏模式及脱敏后的内容。
    Detected {
        /// 检测到的泄漏模式描述列表。
        ///
        /// 每个字符串描述一种检测到的敏感信息类型，例如 "Stripe secret key"、
        /// "AWS Access Key ID" 等。
        patterns: Vec<String>,

        /// 脱敏后的内容。
        ///
        /// 原始内容中的敏感值已被替换为 `[REDACTED_*]` 占位符，
        /// 可以安全地记录或传输。
        redacted: String,
    },
}

/// 凭据泄漏检测器。
///
/// 用于扫描出站内容中的潜在凭据泄漏。支持多种常见的敏感信息模式，
/// 并提供可配置的检测灵敏度。
///
/// # 检测范围
///
/// - API 密钥（Stripe、OpenAI、Anthropic、Google、GitHub 等）
/// - AWS 凭据（Access Key ID 和 Secret Access Key）
/// - 通用秘密值（密码、令牌、密钥）
/// - PEM 格式私钥（RSA、EC、OpenSSH）
/// - JWT 令牌
/// - 数据库连接 URL
///
/// # 灵敏度
///
/// 灵敏度值范围为 `[0.0, 1.0]`：
/// - 较低的值（如 0.3）减少误报，但可能漏检
/// - 较高的值（如 0.9）更激进，但可能产生更多误报
/// - 默认值为 0.7，在误报和漏检之间取得平衡
#[derive(Debug, Clone)]
pub struct LeakDetector {
    /// 检测灵敏度阈值（范围 0.0-1.0）。
    ///
    /// - 值越高，检测越激进，可能产生更多误报
    /// - 值越低，检测越保守，可能漏检部分泄漏
    /// - 默认值为 0.7
    sensitivity: f64,
}

impl Default for LeakDetector {
    /// 返回使用默认配置的泄漏检测器。
    ///
    /// 默认灵敏度值为 0.7。
    fn default() -> Self {
        Self::new()
    }
}

impl LeakDetector {
    /// 创建使用默认灵敏度的新泄漏检测器。
    ///
    /// 默认灵敏度为 0.7，适用于大多数场景。
    ///
    /// # 返回值
    ///
    /// 返回配置了默认灵敏度的新 `LeakDetector` 实例。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let detector = LeakDetector::new();
    /// let result = detector.scan("some content");
    /// ```
    pub fn new() -> Self {
        Self { sensitivity: 0.7 }
    }

    /// 创建使用自定义灵敏度的泄漏检测器。
    ///
    /// 灵敏度值会被自动限制在 `[0.0, 1.0]` 范围内。
    ///
    /// # 参数
    ///
    /// - `sensitivity`: 检测灵敏度（0.0-1.0）
    ///   - 0.0: 最保守，仅检测明确的泄漏
    ///   - 1.0: 最激进，可能产生较多误报
    ///   - 建议值: 0.5-0.8
    ///
    /// # 返回值
    ///
    /// 返回配置了指定灵敏度的新 `LeakDetector` 实例。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // 更激进的检测
    /// let detector = LeakDetector::with_sensitivity(0.9);
    ///
    /// // 更保守的检测
    /// let detector = LeakDetector::with_sensitivity(0.3);
    /// ```
    pub fn with_sensitivity(sensitivity: f64) -> Self {
        Self { sensitivity: sensitivity.clamp(0.0, 1.0) }
    }

    /// 扫描内容，检测潜在的凭据泄漏。
    ///
    /// 对输入内容执行全面的泄漏检测，包括 API 密钥、AWS 凭据、
    /// 通用秘密、私钥、JWT 令牌和数据库 URL。
    ///
    /// # 参数
    ///
    /// - `content`: 待扫描的文本内容
    ///
    /// # 返回值
    ///
    /// 返回 `LeakResult` 枚举：
    /// - `LeakResult::Clean`: 未检测到泄漏
    /// - `LeakResult::Detected`: 检测到泄漏，包含模式列表和脱敏内容
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let detector = LeakDetector::new();
    ///
    /// // 安全内容
    /// assert!(matches!(detector.scan("Hello world"), LeakResult::Clean));
    ///
    /// // 包含泄漏的内容
    /// let result = detector.scan("key=sk-test-1234567890abcdefghijklmnop");
    /// if let LeakResult::Detected { patterns, redacted } = result {
    ///     assert!(!patterns.is_empty());
    ///     assert!(redacted.contains("[REDACTED"));
    /// }
    /// ```
    pub fn scan(&self, content: &str) -> LeakResult {
        let mut patterns = Vec::new();
        let mut redacted = content.to_string();

        // 依次检查各类敏感信息模式
        self.check_api_keys(content, &mut patterns, &mut redacted);
        self.check_aws_credentials(content, &mut patterns, &mut redacted);
        self.check_generic_secrets(content, &mut patterns, &mut redacted);
        self.check_private_keys(content, &mut patterns, &mut redacted);
        self.check_jwt_tokens(content, &mut patterns, &mut redacted);
        self.check_database_urls(content, &mut patterns, &mut redacted);

        // 根据检测结果返回相应的结果枚举
        if patterns.is_empty() {
            LeakResult::Clean
        } else {
            LeakResult::Detected { patterns, redacted }
        }
    }

    /// 检查常见的 API 密钥模式。
    ///
    /// 检测多种主流服务的 API 密钥格式，包括：
    /// - Stripe（密钥和可发布密钥）
    /// - OpenAI（API 密钥）
    /// - Anthropic（API 密钥）
    /// - Google（API 密钥）
    /// - GitHub（令牌和个人访问令牌）
    /// - 通用 API 密钥格式
    ///
    /// # 参数
    ///
    /// - `content`: 原始待扫描内容
    /// - `patterns`: 输出参数，用于追加检测到的模式描述
    /// - `redacted`: 输出参数，敏感值会被替换为 `[REDACTED_API_KEY]`
    fn check_api_keys(&self, content: &str, patterns: &mut Vec<String>, redacted: &mut String) {
        // 使用 OnceLock 延迟初始化正则表达式集合，避免重复编译
        static API_KEY_PATTERNS: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
        let regexes = API_KEY_PATTERNS.get_or_init(|| {
            vec![
                // Stripe 密钥：以 sk_live_ 或 sk_test_ 开头
                (Regex::new(r"sk_(live|test)_[a-zA-Z0-9]{24,}").unwrap(), "Stripe secret key"),
                // Stripe 可发布密钥：以 pk_live_ 或 pk_test_ 开头
                (Regex::new(r"pk_(live|test)_[a-zA-Z0-9]{24,}").unwrap(), "Stripe publishable key"),
                // OpenAI API 密钥：包含 T3BlbkFJ 标识符的格式
                (
                    Regex::new(r"sk-[a-zA-Z0-9]{20,}T3BlbkFJ[a-zA-Z0-9]{20,}").unwrap(),
                    "OpenAI API key",
                ),
                // OpenAI 风格 API 密钥：48 字符以上的通用格式
                (Regex::new(r"sk-[a-zA-Z0-9]{32,}").unwrap(), "OpenAI-style API key"),
                // Anthropic API 密钥：以 sk-ant- 开头
                (Regex::new(r"sk-ant-[a-zA-Z0-9-_]{32,}").unwrap(), "Anthropic API key"),
                // Google API 密钥：以 AIza 开头，固定 35 字符后缀
                (Regex::new(r"AIza[a-zA-Z0-9_-]{35}").unwrap(), "Google API key"),
                // GitHub 令牌：ghp_（个人）、gho_（OAuth）、ghs_（服务）、ghu_（用户）、ghr_（刷新）
                (Regex::new(r"gh[pousr]_[a-zA-Z0-9]{36,}").unwrap(), "GitHub token"),
                // GitHub 个人访问令牌：以 github_pat_ 开头
                (Regex::new(r"github_pat_[a-zA-Z0-9_]{22,}").unwrap(), "GitHub PAT"),
                // 通用 API 密钥：api_key=、api-key: 等格式后跟长字符串
                (
                    Regex::new(r#"api[_-]?key[=:]\s*['"]*[a-zA-Z0-9_-]{20,}"#).unwrap(),
                    "Generic API key",
                ),
            ]
        });

        // 遍历所有模式，检测并脱敏匹配的内容
        for (regex, name) in regexes {
            if regex.is_match(content) {
                patterns.push(name.to_string());
                *redacted = regex.replace_all(redacted, "[REDACTED_API_KEY]").to_string();
            }
        }
    }

    /// 检查 AWS 凭据。
    ///
    /// 检测 AWS 相关的敏感凭据，包括：
    /// - Access Key ID（以 AKIA 开头，16 字符）
    /// - Secret Access Key（通常与 aws_secret_access_key 配置项一起出现）
    ///
    /// # 参数
    ///
    /// - `content`: 原始待扫描内容
    /// - `patterns`: 输出参数，用于追加检测到的模式描述
    /// - `redacted`: 输出参数，敏感值会被替换为 `[REDACTED_AWS_CREDENTIAL]`
    fn check_aws_credentials(
        &self,
        content: &str,
        patterns: &mut Vec<String>,
        redacted: &mut String,
    ) {
        // 使用 OnceLock 延迟初始化正则表达式集合
        static AWS_PATTERNS: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
        let regexes = AWS_PATTERNS.get_or_init(|| {
            vec![
                // AWS Access Key ID：AKIA 前缀 + 16 位大写字母数字
                (Regex::new(r"AKIA[A-Z0-9]{16}").unwrap(), "AWS Access Key ID"),
                // AWS Secret Access Key：配置文件中的密钥值
                (
                    Regex::new(
                        r#"aws[_-]?secret[_-]?access[_-]?key[=:]\s*['"]*[a-zA-Z0-9/+=]{40}"#,
                    )
                    .unwrap(),
                    "AWS Secret Access Key",
                ),
            ]
        });

        // 遍历所有模式，检测并脱敏匹配的内容
        for (regex, name) in regexes {
            if regex.is_match(content) {
                patterns.push(name.to_string());
                *redacted = regex.replace_all(redacted, "[REDACTED_AWS_CREDENTIAL]").to_string();
            }
        }
    }

    /// 检查通用密钥模式。
    ///
    /// 检测配置文件或代码中常见的密码、秘密值和令牌模式。
    /// 此检测受灵敏度阈值影响，仅在灵敏度 > 0.5 时触发。
    ///
    /// # 检测模式
    ///
    /// - `password=...` / `password: ...` 格式的密码配置
    /// - `secret=...` / `secret: ...` 格式的秘密值
    /// - `token=...` / `token: ...` 格式的令牌值
    ///
    /// # 参数
    ///
    /// - `content`: 原始待扫描内容
    /// - `patterns`: 输出参数，用于追加检测到的模式描述
    /// - `redacted`: 输出参数，敏感值会被替换为 `[REDACTED_SECRET]`
    fn check_generic_secrets(
        &self,
        content: &str,
        patterns: &mut Vec<String>,
        redacted: &mut String,
    ) {
        // 使用 OnceLock 延迟初始化正则表达式集合
        static SECRET_PATTERNS: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
        let regexes = SECRET_PATTERNS.get_or_init(|| {
            vec![
                // 密码配置：password= 或 password: 后跟 8 字符以上的值
                (
                    Regex::new(r#"(?i)password[=:]\s*['"]*[^\s'"]{8,}"#).unwrap(),
                    "Password in config",
                ),
                // 秘密值：secret= 或 secret: 后跟 16 字符以上的值
                (
                    Regex::new(r#"(?i)secret[=:]\s*['"]*[a-zA-Z0-9_-]{16,}"#).unwrap(),
                    "Secret value",
                ),
                // 令牌值：token= 或 token: 后跟 20 字符以上的值
                (Regex::new(r#"(?i)token[=:]\s*['"]*[a-zA-Z0-9_.-]{20,}"#).unwrap(), "Token value"),
            ]
        });

        // 仅在灵敏度 > 0.5 时检测通用模式（避免过多误报）
        for (regex, name) in regexes {
            if regex.is_match(content) && self.sensitivity > 0.5 {
                patterns.push(name.to_string());
                *redacted = regex.replace_all(redacted, "[REDACTED_SECRET]").to_string();
            }
        }
    }

    /// 检查 PEM 格式的私钥。
    ///
    /// 检测多种格式的 PEM 编码私钥，包括：
    /// - RSA 私钥
    /// - EC（椭圆曲线）私钥
    /// - 通用 PKCS#8 私钥
    /// - OpenSSH 私钥
    ///
    /// 检测到私钥时，会脱敏整个密钥块（从 BEGIN 到 END 标记之间的所有内容）。
    ///
    /// # 参数
    ///
    /// - `content`: 原始待扫描内容
    /// - `patterns`: 输出参数，用于追加检测到的模式描述
    /// - `redacted`: 输出参数，私钥块会被替换为 `[REDACTED_PRIVATE_KEY]`
    fn check_private_keys(&self, content: &str, patterns: &mut Vec<String>, redacted: &mut String) {
        // 定义 PEM 格式私钥的标记对
        let key_patterns = [
            ("-----BEGIN RSA PRIVATE KEY-----", "-----END RSA PRIVATE KEY-----", "RSA private key"),
            ("-----BEGIN EC PRIVATE KEY-----", "-----END EC PRIVATE KEY-----", "EC private key"),
            ("-----BEGIN PRIVATE KEY-----", "-----END PRIVATE KEY-----", "Private key"),
            (
                "-----BEGIN OPENSSH PRIVATE KEY-----",
                "-----END OPENSSH PRIVATE KEY-----",
                "OpenSSH private key",
            ),
        ];

        // 检查每种私钥格式
        for (begin, end, name) in key_patterns {
            if content.contains(begin) && content.contains(end) {
                patterns.push(name.to_string());

                // 脱敏整个密钥块
                if let Some(start_idx) = content.find(begin) {
                    if let Some(end_idx) = content.find(end) {
                        let key_block = &content[start_idx..end_idx + end.len()];
                        *redacted = redacted.replace(key_block, "[REDACTED_PRIVATE_KEY]");
                    }
                }
            }
        }
    }

    /// 检查 JWT 令牌。
    ///
    /// 检测 JSON Web Token (JWT) 格式的令牌。JWT 由三部分组成，每部分使用
    /// Base64URL 编码，用点号分隔。JWT 通常包含用户身份和权限信息，
    /// 泄露可能导致身份伪造。
    ///
    /// # JWT 格式
    ///
    /// JWT 格式为：`header.payload.signature`，其中：
    /// - header 和 payload 以 `eyJ` 开头（Base64 编码的 `{`）
    ///
    /// # 参数
    ///
    /// - `content`: 原始待扫描内容
    /// - `patterns`: 输出参数，用于追加检测到的模式描述
    /// - `redacted`: 输出参数，JWT 会被替换为 `[REDACTED_JWT]`
    fn check_jwt_tokens(&self, content: &str, patterns: &mut Vec<String>, redacted: &mut String) {
        // 使用 OnceLock 延迟初始化 JWT 正则表达式
        static JWT_PATTERN: OnceLock<Regex> = OnceLock::new();
        let regex = JWT_PATTERN.get_or_init(|| {
            // JWT 格式：三段 Base64URL 编码，用点号分隔
            // eyJ 是 Base64 编码的 {，JWT 的 header 和 payload 都是 JSON 对象
            Regex::new(r"eyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]*").unwrap()
        });

        // 检测并脱敏 JWT 令牌
        if regex.is_match(content) {
            patterns.push("JWT token".to_string());
            *redacted = regex.replace_all(redacted, "[REDACTED_JWT]").to_string();
        }
    }

    /// 检查数据库连接 URL。
    ///
    /// 检测包含凭据的数据库连接字符串，这些 URL 通常包含用户名和密码，
    /// 泄露可能导致数据库被未授权访问。
    ///
    /// # 检测的数据库类型
    ///
    /// - PostgreSQL（postgres:// 或 postgresql://）
    /// - MySQL（mysql://）
    /// - MongoDB（mongodb:// 或 mongodb+srv://）
    /// - Redis（redis://）
    ///
    /// # 参数
    ///
    /// - `content`: 原始待扫描内容
    /// - `patterns`: 输出参数，用于追加检测到的模式描述
    /// - `redacted`: 输出参数，连接 URL 会被替换为 `[REDACTED_DATABASE_URL]`
    fn check_database_urls(
        &self,
        content: &str,
        patterns: &mut Vec<String>,
        redacted: &mut String,
    ) {
        // 使用 OnceLock 延迟初始化正则表达式集合
        static DB_PATTERNS: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
        let regexes = DB_PATTERNS.get_or_init(|| {
            vec![
                // PostgreSQL 连接 URL
                (
                    Regex::new(r"postgres(ql)?://[^:]+:[^@]+@[^\s]+").unwrap(),
                    "PostgreSQL connection URL",
                ),
                // MySQL 连接 URL
                (Regex::new(r"mysql://[^:]+:[^@]+@[^\s]+").unwrap(), "MySQL connection URL"),
                // MongoDB 连接 URL（支持 mongodb:// 和 mongodb+srv://）
                (
                    Regex::new(r"mongodb(\+srv)?://[^:]+:[^@]+@[^\s]+").unwrap(),
                    "MongoDB connection URL",
                ),
                // Redis 连接 URL
                (Regex::new(r"redis://[^:]+:[^@]+@[^\s]+").unwrap(), "Redis connection URL"),
            ]
        });

        // 遍历所有模式，检测并脱敏匹配的内容
        for (regex, name) in regexes {
            if regex.is_match(content) {
                patterns.push(name.to_string());
                *redacted = regex.replace_all(redacted, "[REDACTED_DATABASE_URL]").to_string();
            }
        }
    }
}

#[cfg(test)]
#[path = "leak_detector_tests.rs"]
mod leak_detector_tests;
