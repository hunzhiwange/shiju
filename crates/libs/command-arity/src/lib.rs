//! 命令元数（参数数量）定义模块
//!
//! 本模块维护了一个静态的命令元数映射表，用于确定各类工具命令需要多少个
//! 前缀参数来唯一标识该命令。这在权限检查和命令匹配时非常重要。
//!
//! # 元数含义
//!
//! - 元数为 1：基本 Shell 命令（如 `cat`、`ls`、`rm`）
//! - 元数为 2：包管理器/工具主命令（如 `npm`、`docker`、`git`）
//! - 元数为 3：子命令级别的操作（如 `npm run`、`docker compose`、`git config`）
//!
//! # 示例
//!
//! ```ignore
//! use command_arity::prefix;
//!
//! // 对于 "npm run build -- --prod"，返回 ["npm", "run"]
//! let tokens = vec!["npm", "run", "build", "--", "--prod"];
//! let prefix_tokens = prefix(&tokens);
//! assert_eq!(prefix_tokens, vec!["npm", "run"]);
//!
//! // 对于 "ls -la"，返回 ["ls"]
//! let tokens = vec!["ls", "-la"];
//! let prefix_tokens = prefix(&tokens);
//! assert_eq!(prefix_tokens, vec!["ls"]);
//! ```

use std::collections::HashMap;
use std::sync::LazyLock;

/// 命令元数静态映射表
///
/// 该映射表定义了各类命令及其对应的元数（参数数量）。
/// 键为命令字符串（可能包含空格分隔的子命令），值为该命令需要的前缀参数数量。
///
/// # 元数规则
///
/// - `1`: 基本 Shell 命令（如 `cat`, `ls`, `rm`）
/// - `2`: 主工具命令（如 `npm`, `docker`, `git`）
/// - `3`: 子命令组合（如 `npm run`, `docker compose`, `git config`）
///
/// # 用途
///
/// 在权限检查时，需要确定用户调用的具体是哪个命令。通过元数，
/// 可以从完整的命令行参数中提取出命令前缀，用于与权限规则匹配。
static ARITY: LazyLock<HashMap<&'static str, usize>> = LazyLock::new(|| {
    HashMap::from([
        // === 基本 Shell 命令 (元数 = 1) ===
        // 这些是标准的 Unix/Linux 命令，通常不需要子命令即可明确其含义
        ("cat", 1),
        ("cd", 1),
        ("chmod", 1),
        ("chown", 1),
        ("cp", 1),
        ("echo", 1),
        ("env", 1),
        ("export", 1),
        ("grep", 1),
        ("kill", 1),
        ("killall", 1),
        ("ln", 1),
        ("ls", 1),
        ("mkdir", 1),
        ("mv", 1),
        ("ps", 1),
        ("pwd", 1),
        ("rm", 1),
        ("rmdir", 1),
        ("sleep", 1),
        ("source", 1),
        ("tail", 1),
        ("touch", 1),
        ("unset", 1),
        ("which", 1),
        // === 云服务 CLI (元数 = 3) ===
        // 云服务提供商的命令行工具，通常格式为 <cli> <service> <action>
        ("aws", 3), // Amazon Web Services CLI
        ("az", 3),  // Azure CLI
        // === 开发工具 (元数 = 2-3) ===
        ("bazel", 2),     // Google 构建工具
        ("brew", 2),      // macOS 包管理器
        ("bun", 2),       // Bun JavaScript 运行时
        ("bun run", 3),   // Bun 运行脚本
        ("bun x", 3),     // Bun 执行包命令
        ("cargo", 2),     // Rust 包管理器
        ("cargo add", 3), // Cargo 添加依赖
        ("cargo run", 3), // Cargo 运行项目
        // === 基础设施即代码 (元数 = 2-3) ===
        ("cdk", 2),       // AWS Cloud Development Kit
        ("cf", 2),        // Cloud Foundry CLI
        ("cmake", 2),     // CMake 构建系统
        ("composer", 2),  // PHP 依赖管理器
        ("consul", 2),    // HashiCorp Consul
        ("consul kv", 3), // Consul KV 存储
        ("crictl", 2),    // CRI 容器运行时接口工具
        // === JavaScript/TypeScript 运行时 (元数 = 2-3) ===
        ("deno", 2),      // Deno JavaScript 运行时
        ("deno task", 3), // Deno 任务运行器
        // === 云平台 CLI (元数 = 2-3) ===
        ("doctl", 3),            // DigitalOcean CLI
        ("docker", 2),           // Docker 容器引擎
        ("docker builder", 3),   // Docker 构建器
        ("docker compose", 3),   // Docker Compose
        ("docker container", 3), // Docker 容器管理
        ("docker image", 3),     // Docker 镜像管理
        ("docker network", 3),   // Docker 网络管理
        ("docker volume", 3),    // Docker 卷管理
        // === Kubernetes 工具 (元数 = 2-3) ===
        ("eksctl", 2),        // Amazon EKS CLI
        ("eksctl create", 3), // EKS 创建资源
        ("firebase", 2),      // Firebase CLI
        ("flyctl", 2),        // Fly.io CLI
        ("gcloud", 3),        // Google Cloud CLI
        ("gh", 3),            // GitHub CLI
        // === 版本控制 (元数 = 2-3) ===
        ("git", 2),        // Git 版本控制
        ("git config", 3), // Git 配置
        ("git remote", 3), // Git 远程仓库
        ("git stash", 3),  // Git 暂存
        // === 编程语言工具 (元数 = 2) ===
        ("go", 2),     // Go 语言
        ("gradle", 2), // Gradle 构建工具
        // === 容器编排 (元数 = 2) ===
        ("helm", 2),   // Kubernetes 包管理器
        ("heroku", 2), // Heroku CLI
        ("hugo", 2),   // Hugo 静态站点生成器
        // === 网络工具 (元数 = 2-3) ===
        ("ip", 2),       // IP 网络配置
        ("ip addr", 3),  // IP 地址管理
        ("ip link", 3),  // 网络接口管理
        ("ip netns", 3), // 网络命名空间
        ("ip route", 3), // 路由表管理
        // === Kubernetes 本地工具 (元数 = 2-3) ===
        ("kind", 2),              // Kubernetes in Docker
        ("kind create", 3),       // 创建 kind 集群
        ("kubectl", 2),           // Kubernetes CLI
        ("kubectl kustomize", 3), // Kustomize 集成
        ("kubectl rollout", 3),   // 部署滚动更新
        ("kustomize", 2),         // Kustomize
        // === 构建工具 (元数 = 2) ===
        ("make", 2), // Make 构建工具
        // === 对象存储 (元数 = 2-3) ===
        ("mc", 2),       // MinIO Client
        ("mc admin", 3), // MinIO 管理命令
        ("minikube", 2), // 本地 Kubernetes
        // === 数据库客户端 (元数 = 2) ===
        ("mongosh", 2), // MongoDB Shell
        ("mysql", 2),   // MySQL 客户端
        ("mvn", 2),     // Maven 构建工具
        // === 前端框架 (元数 = 2) ===
        ("ng", 2), // Angular CLI
        // === Node.js 包管理器 (元数 = 2-3) ===
        ("npm", 2),      // Node Package Manager
        ("npm exec", 3), // NPM 执行包命令
        ("npm init", 3), // NPM 初始化项目
        ("npm run", 3),  // NPM 运行脚本
        ("npm view", 3), // NPM 查看包信息
        ("nvm", 2),      // Node Version Manager
        ("nx", 2),       // Nx 构建系统
        // === 安全工具 (元数 = 2-3) ===
        ("openssl", 2),      // OpenSSL
        ("openssl req", 3),  // 证书请求
        ("openssl x509", 3), // X.509 证书操作
        // === Python 工具 (元数 = 2) ===
        ("pip", 2),    // Python 包安装器
        ("pipenv", 2), // Python 虚拟环境管理
        // === 替代包管理器 (元数 = 2-3) ===
        ("pnpm", 2),      // 快速、磁盘空间高效的包管理器
        ("pnpm dlx", 3),  // PNPM 执行包命令
        ("pnpm exec", 3), // PNPM 执行
        ("pnpm run", 3),  // PNPM 运行脚本
        ("poetry", 2),    // Python 依赖管理
        // === 容器运行时 (元数 = 2-3) ===
        ("podman", 2),           // Podman 容器引擎
        ("podman container", 3), // Podman 容器管理
        ("podman image", 3),     // Podman 镜像管理
        // === 数据库 (元数 = 2) ===
        ("psql", 2), // PostgreSQL 客户端
        // === 基础设施即代码 (元数 = 2-3) ===
        ("pulumi", 2),       // Pulumi IaC
        ("pulumi stack", 3), // Pulumi 栈管理
        // === Python 版本管理 (元数 = 2) ===
        ("pyenv", 2),  // Python 版本管理器
        ("python", 2), // Python 解释器
        // === Ruby 工具 (元数 = 2) ===
        ("rake", 2),  // Ruby 构建工具
        ("rbenv", 2), // Ruby 版本管理器
        // === 缓存/数据存储 (元数 = 2) ===
        ("redis-cli", 2), // Redis 命令行客户端
        // === Rust 工具 (元数 = 2) ===
        ("rustup", 2), // Rust 工具链管理器
        // === Serverless (元数 = 2) ===
        ("serverless", 2), // Serverless Framework
        ("sfdx", 3),       // Salesforce DX
        ("skaffold", 2),   // Kubernetes 开发工具
        ("sls", 2),        // Serverless CLI 简写
        ("sst", 2),        // SST (Serverless Stack)
        // === Apple 平台 (元数 = 2) ===
        ("swift", 2), // Swift 语言
        // === 系统管理 (元数 = 2) ===
        ("systemctl", 2), // Systemd 服务管理
        // === 基础设施即代码 (元数 = 2-3) ===
        ("terraform", 2),           // Terraform
        ("terraform workspace", 3), // Terraform 工作区
        ("tmux", 2),                // 终端复用器
        ("turbo", 2),               // Turborepo
        // === 防火墙 (元数 = 2) ===
        ("ufw", 2), // Uncomplicated Firewall
        // === 密钥管理 (元数 = 2-3) ===
        ("vault", 2),      // HashiCorp Vault
        ("vault auth", 3), // Vault 认证
        ("vault kv", 3),   // Vault KV 引擎
        // === 部署平台 (元数 = 2) ===
        ("vercel", 2), // Vercel CLI
        ("volta", 2),  // Volta JS 版本管理
        // === WordPress (元数 = 2) ===
        ("wp", 2), // WordPress CLI
        // === Yarn 包管理器 (元数 = 2-3) ===
        ("yarn", 2),     // Yarn 包管理器
        ("yarn dlx", 3), // Yarn 执行包命令
        ("yarn run", 3), // Yarn 运行脚本
    ])
});

/// 从命令行参数中提取命令前缀
///
/// 该函数根据预定义的元数映射表，从完整的命令行参数中提取出
/// 用于权限匹配的命令前缀。
///
/// # 参数
///
/// * `tokens` - 命令行参数切片，例如 `["npm", "run", "build"]`
///
/// # 返回值
///
/// 返回一个字符串向量，包含提取出的命令前缀。
///
/// # 算法逻辑
///
/// 1. 从最长的可能前缀开始尝试（从 `tokens.len()` 到 1）
/// 2. 如果某个前缀在 ARITY 映射表中找到，返回对应元数的前缀
/// 3. 如果没有找到任何匹配，返回第一个参数作为前缀
/// 4. 如果输入为空，返回空向量
///
/// # 示例
///
/// ```ignore
/// // npm run 的元数是 3，所以返回前 3 个参数
/// let tokens = vec!["npm", "run", "build", "--prod"];
/// assert_eq!(prefix(&tokens), vec!["npm", "run", "build"]);
///
/// // git 的元数是 2，所以返回前 2 个参数
/// let tokens = vec!["git", "commit", "-m", "message"];
/// assert_eq!(prefix(&tokens), vec!["git", "commit"]);
///
/// // ls 的元数是 1，所以返回第 1 个参数
/// let tokens = vec!["ls", "-la"];
/// assert_eq!(prefix(&tokens), vec!["ls"]);
///
/// // 未知命令返回第一个参数
/// let tokens = vec!["unknown", "command"];
/// assert_eq!(prefix(&tokens), vec!["unknown"]);
/// ```
pub fn prefix(tokens: &[&str]) -> Vec<String> {
    let mut out = Vec::with_capacity(tokens.len());
    for len in 1..=tokens.len() {
        out.push(tokens[..len].join(" "));
        if let Some(arity) = ARITY.get(out.last().map(String::as_str).unwrap_or_default()) {
            if *arity <= len {
                break;
            }
        }
    }
    out
}

#[cfg(test)]
#[path = "arity_tests.rs"]
mod arity_tests;
