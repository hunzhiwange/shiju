# macOS CI 签名计划

目标：让 Tag 发布流水线自动生成 Developer ID 签名且已公证的 macOS ZIP 和 DMG，同时保留 PR 与本地开发构建。

范围：扩展 macOS bundle 脚本的临时钥匙串和 DMG 公证能力；在现有 cargo-dist GitHub Actions 中安全导入证书、配置 Notary 凭据、构建产物并清理凭据；补充 Secrets 文档。

范围外：不创建 Apple 证书或 App Store Connect API 密钥，不修改 GitHub 仓库 Secrets，不改变非 macOS 发布流程。

假设：发布工作流由 Tag push 触发，macOS 构建运行在 GitHub macOS runner；PR 需要继续验证打包但不能访问发布密钥。

## 设计决策

- Tag 构建使用 GitHub Secrets 中的 base64 `.p12` 和 Apple 账户 App 专用密码，将签名证书与 Notary profile 写入 runner 临时钥匙串。
- PR 沿用 ad-hoc 签名；正式密钥步骤只在非 PR 的 macOS 发布矩阵运行。
- CI 对 ZIP 内 App 和最终 DMG 分别公证并装订，兼顾离线 Gatekeeper 验证。

## 架构说明

- `scripts/bundle_macos.sh` 继续负责签名、公证和产物验证，workflow 只负责注入短期凭据。
- 本地默认钥匙串保持兼容；CI 通过可选钥匙串路径显式访问 Notary profile。
- 密钥文件仅写入 `$RUNNER_TEMP`，流水线结束始终清理。

## 开发策略

- workflow 和 shell 变更使用语法解析、缺参失败路径、模拟签名公证流程及 dry-run 验证。
- 不调用真实 Apple 服务或 GitHub Secrets；外部边界使用固定模拟结果。
- 完成后执行仓库规定的 Clippy。
