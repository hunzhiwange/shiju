# SHiJu

SHiJu 是一个 Rust-first 的自主代理运行时，目标是在桌面端、CLI、网关服务和长驻守护进程之间提供同一套高性能、可扩展、可审计的 Agent 能力。

项目当前包含桌面应用、命令行入口、HTTP/WebSocket 网关、任务与记忆管理、通信通道、模型/提供商解析、ACP 桥接和跨平台 WebView helper。

## 主要能力

- `shiju` CLI：交互式 Agent、单次消息、网关、守护进程、任务、记忆、技能、集成、计划任务和安全维护。
- `vibe-window` 桌面端：基于 Iced 的本地桌面应用。
- `vw-webview` helper：跨平台 WebView 窗口，用于桌面打包场景。
- `acp`：VibeWindow 作为 ACP Client 的现有桥接入口。
- `shiju acp`：VibeWindow 作为 ACP Agent 的 stdio 入口。
- 共享 crate：API 类型、配置类型、网关客户端、运行时核心、提供商解析和通用工具。

## 仓库结构

```text
crates/
  vw-cli/               # shiju CLI 和 vibe-agent 兼容入口
  vw-desktop/           # Iced 桌面应用
  vw-agent/             # Agent 运行时核心
  vw-acp/               # Agent Client Protocol 桥接
  vw-api-types/         # API 数据结构
  vw-config-types/      # 配置数据结构
  vw-gateway-client/    # 网关客户端
  vw-provider-resolver/ # 提供商和模型解析
  vw-shared/            # 跨 crate 共享工具
  vw-webview/           # WebView helper
docs/                   # 使用文档、OpenAPI 和参考资料
scripts/                # 构建、安装和打包脚本
release/                # 发布说明与发布辅助文件
skills/                 # 本地技能说明
```

## 环境要求

- Rust toolchain 使用仓库内 `rust-toolchain.toml` 指定的 `1.93.0`。
- 推荐通过 `rustup` 安装 Rust，并确保 `clippy` 组件可用。
- macOS 打包需要 `cargo-bundle`。
- Windows 交叉编译可选 `cargo-xwin`，Linux 交叉编译可选 `cross`。

## 快速开始

查看 CLI 帮助：

```bash
cargo run -p vw-cli --bin shiju -- --help
```

启动交互式 Agent：

```bash
cargo run -p vw-cli --bin shiju -- agent
```

发送单条消息：

```bash
cargo run -p vw-cli --bin shiju -- agent -m "总结这个仓库的主要模块"
```

启动桌面应用：

```bash
cargo build -p vw-webview --bin vw-webview
cargo run -p vw-desktop --bin shi-ju
```

启动网关：

```bash
cargo run -p vw-cli --bin shiju -- gateway --host 127.0.0.1 --port 8080
```

启动长驻运行时：

```bash
cargo run -p vw-cli --bin shiju -- daemon
```

作为 ACP Agent 供外部 Client 启动：

```bash
cargo run -p vw-cli --bin shiju -- acp
```

## 常用 CLI 命令

```bash
# 查看运行状态
cargo run -p vw-cli --bin shiju -- status

# 诊断模型提供商
cargo run -p vw-cli --bin shiju -- doctor models

# 导出配置 JSON Schema
cargo run -p vw-cli --bin shiju -- config schema

# 查看支持的提供商
cargo run -p vw-cli --bin shiju -- providers

# 创建项目任务
cargo run -p vw-cli --bin shiju -- task create --prompt "检查 OAuth 回调并修复"

# 读取项目任务
cargo run -p vw-cli --bin shiju -- task read --limit 20

# 查看计划任务
cargo run -p vw-cli --bin shiju -- cron list

# 生成 shell 补全
cargo run -p vw-cli --bin shiju -- completions zsh
```

本地安装或 release 构建后，可将上面的 `cargo run -p vw-cli --bin shiju --` 替换为 `shiju`。

## 构建

构建 CLI：

```bash
cargo build -p vw-cli --bin shiju --release
```

构建桌面应用：

```bash
cargo build -p vw-desktop --bin shi-ju --release
```

构建 WebView helper：

```bash
cargo build -p vw-webview --bin vw-webview --release
```

构建 VibeWindow 作为 ACP Client 的现有桥接程序：

```bash
cargo build -p vw-acp --bin acp --release
```

ACP Agent 入口包含在 `shiju` CLI 构建中，接入配置和支持矩阵见
[`docs/vibewindow/ACP-对外接入.md`](docs/vibewindow/ACP-对外接入.md)。

## 打包

macOS `.app` 打包：

```bash
cargo install cargo-bundle
./scripts/bundle_macos.sh
```

Windows 打包：

```bash
cargo install cargo-xwin --locked
bash scripts/bundle_windows.sh
```

Linux 打包：

```bash
bash scripts/bundle_linux.sh
```

CLI 多平台构建脚本示例：

```bash
./scripts/build-cli.sh --target x86_64-apple-darwin
./scripts/build-cli.sh --target x86_64-unknown-linux-gnu --use-cross
./scripts/build-cli.sh --target x86_64-pc-windows-msvc --use-xwin
```

更多平台构建说明见 [`scripts/BUILDING.md`](scripts/BUILDING.md)。

## 配置与数据

CLI 支持通过全局参数指定配置目录：

```bash
shiju --config-dir /path/to/config status
```

项目任务默认写入目标项目下的 `.vibewindow/tasks`。可以通过 `--project-dir` 指向其他仓库：

```bash
shiju task --project-dir /path/to/repo read
```

完整配置键请以当前代码生成的 Schema 为准：

```bash
shiju config schema > vibewindow.schema.json
```

## 开发检查

基础静态检查：

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

按需运行更聚焦的包级检查：

```bash
cargo clippy -p vw-cli --all-targets -- -D warnings
cargo clippy -p vw-desktop --all-targets -- -D warnings
```

测试命令可按变更范围选择具体 package 或测试目标运行。

## 相关文档

- [`docs/vibewindow/ACP-对外接入.md`](docs/vibewindow/ACP-对外接入.md)：ACP Agent 启动、能力矩阵与安全边界。
- [`docs/knowledge-base.md`](docs/knowledge-base.md)：知识库入库、嵌入配置和验证指南。
- [`docs/vibe-agent-usage.md`](docs/vibe-agent-usage.md)：Vibe Agent 使用指南。
- [`docs/openapi.json`](docs/openapi.json)：网关 OpenAPI 描述。
- [`scripts/BUILDING.md`](scripts/BUILDING.md)：macOS / Windows 构建与打包说明。
- [`release/README.md`](release/README.md)：发布相关说明。
- [`skills/README.md`](skills/README.md)：本地技能说明。

## 许可证

本项目使用 MIT License，见 [`LICENSE`](LICENSE)。
