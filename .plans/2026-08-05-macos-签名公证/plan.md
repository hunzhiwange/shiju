# macOS 签名公证计划

目标：将原 `make macos` 本地构建迁移为 `make macos-dev`，并让 `make macos` 产出 Developer ID 签名且已公证的 ZIP。

范围：修改 Makefile、macOS bundle 脚本和构建文档；正式流程从钥匙串读取签名身份与公证凭据，逐个签名内嵌二进制，再签名、公证、装订和验证应用。

范围外：不申请或安装 Apple 证书，不把任何密码或私钥写入仓库，不启用 App Sandbox，不改变 Rust 应用逻辑。

假设：现有 `.app` 包含主程序 `shi-ju`、helper `vw-webview`、CLI `shiju`，可选包含 `vibe-agent` 与 `acp`；应用采用 Developer ID 站外分发并需要出站联网。

## 设计决策

- `make macos-dev` 保留 ad-hoc 本地构建；`make macos` 强制要求 Developer ID 身份及钥匙串公证配置。
- 正式流程生成 ZIP，提交 ZIP 公证，装订 `.app` 后重新生成最终 ZIP。
- 不应用现有沙盒 entitlement；Hardened Runtime 不限制非沙盒应用联网。

## 架构说明

- 沿用 `scripts/bundle_macos.sh` 作为唯一组包入口，通过显式模式控制开发签名与正式签名。
- 内嵌二进制从内向外签名，最终签名 `.app`；签名阶段禁用 `--deep`，验证阶段允许递归检查。
- 公证凭据仅通过 `notarytool` 钥匙串 profile 引用。

## 开发策略

- shell/Makefile 外部签名流程不适合传统单元测试，使用语法检查、Make dry-run、缺参失败路径及现有构建检查验证。
- 先确认旧目标和脚本当前行为，再做最小改动并检查差异。
- 不执行仓库禁止的全量测试、格式化检查或 Makefile 验证目标。
