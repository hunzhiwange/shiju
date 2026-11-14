# macOS 交付包清单

2026-11-14：文档已编写，未验证。按本轮指令，构建、格式/链接核验、验收及最终回归全部延期。下表为仓库配置所述命名，不能作为安装包已生成或发布成功的证据。

## 输入与适用边界

- [运行契约](runtime-contract.json)：任务 1 的准备输入；`artifacts.binary_paths`、`artifacts.binary_sha256`、`resources.arm64_host_ref`、`resources.x64_host_ref` 均未验证，没有可消费的冻结构建复制品。
- [打包脚本](../../../scripts/bundle_macos.sh)：包格式、默认开关、嵌入文件、签名与公证流程的来源。
- [发布说明](../../shiju-release-runbook.md)：正式 Release macOS DMG 命名来源。其示例版本不是本次交付版本；本任务不下载、不发布。
- [桌面包配置](../../../crates/vw-desktop/Cargo.toml)：阅读时源码声明版本 `0.6.0`，不等于冻结包版本或安装后的应用版本。
- [本任务记录](evidence/61.json)：保存缺口和延期事项，不构成验收通过凭证。

## 架构与包名

脚本 `TARGET_SUFFIX` 来自显式 `TARGET`；未设置时文件名不含目标三元组，不能据此判断架构。`PROFILE` 默认 `release`；发布说明使用 `dist` 后缀，两者不得混作同一实际产物。以下均为规则展开结果，未读取实际包。

| 架构 | 目标三元组 | 发布说明列出的 DMG | 脚本默认 release DMG |
| --- | --- | --- | --- |
| ARM64 | `aarch64-apple-darwin` | `ShiJu-macos-aarch64-apple-darwin-dist.dmg` | `ShiJu-macos-aarch64-apple-darwin-release.dmg` |
| x64 | `x86_64-apple-darwin` | `ShiJu-macos-x86_64-apple-darwin-dist.dmg` | `ShiJu-macos-x86_64-apple-darwin-release.dmg` |

脚本还定义以下类型（来源均为上述打包脚本）：

| 类型 | 路径或命名规则 | 相关配置 |
| --- | --- | --- |
| 应用目录 | `<TARGET_ROOT>/<TARGET>/<PROFILE_DIR>/bundle/osx/ShiJu.app` | 默认 `TARGET_ROOT=target-dir`；未设置 TARGET 时省略该层；dev 映射 debug |
| 桌面 ZIP | `ShiJu-macos-<TARGET>-<PROFILE>.zip` | `PACKAGE_APP=1` 才生成；默认 0 |
| 桌面 DMG | `ShiJu-macos-<TARGET>-<PROFILE>.dmg` | `CREATE_DMG=1` 才生成；默认 0；含应用与 Applications 链接 |
| CLI ZIP | `ShiJu-CLI-macos-<TARGET>-<PROFILE>.zip` | `MODE=cli`；包含 `shiju`、`vibe-agent`；不表示桌面包通过 |

默认输出目录 `dist/macos`。应用主程序 `shi-ju`，嵌入 `vw-webview`；默认嵌入 CLI `shiju`，兼容 CLI `vibe-agent` 与 `acp` 分别由 `INCLUDE_CLI_COMPAT_IN_APP`、`INCLUDE_ACP_IN_APP` 控制，默认均关闭。脚本还暂存 computer-control 资源并写入相机、麦克风和语音用途说明；实际包内容及授权行为未验证。

## 待回填的实际交付记录

| 字段 | ARM64 | x64 |
| --- | --- | --- |
| 实际文件路径、文件名、大小 | 未验证 | 未验证 |
| 冻结产品树提交及工作区差异 | 未交付 | 未交付 |
| 构建配置与构建记录引用 | 未交付 | 未交付 |
| 包版本及安装后应用版本 | 未验证 | 未验证 |
| 最终分发包 SHA-256 | 未验证 | 未验证 |
| 包内可执行文件实际架构 | 未验证 | 未验证 |
| Developer ID 签名及校验结果 | 未验证 | 未验证 |
| 公证、票据及 Gatekeeper 结果 | 未验证 | 未验证 |
| 最低 macOS 配置及实际兼容性 | 未验证 | 未验证 |
| 安装主机 macOS 版本、架构、安装/启动证据 | 未验证 | 未验证 |

签名流程来源：脚本默认 `MACOS_SIGN_MODE=adhoc`、`MACOS_NOTARIZE=0`；ad-hoc 不能计作 Developer ID 或公证成功。Developer ID 分支要求身份存在，公证要求 `PACKAGE_APP=1` 和钥匙串配置引用。脚本定义签名校验、Apple 提交、staple/validate 与 Gatekeeper 检查；定义这些步骤不代表本轮执行过。包重打或补票后必须记录最终文件摘要，不能沿用较早 ZIP 摘要。文档不保存真实签名身份、钥匙串密码或令牌。

最低 macOS 支持版本尚无本轮可采信的交付依据；不得从 Rust 三元组、当前主机版本或签名成功推导。后续需结合冻结构建配置、最终 Info.plist/二进制部署目标与对应系统实测回填，冲突交由构建所有者处理。

## 延期工作与交接

构建所有者先交付两架构独立冻结包及其产品树、版本、最终 SHA-256、配置和签名/公证记录，再核对本清单与实际包。源码或构建输入变化后须重新确认受影响证据的适用性。缺失资源保持未验证，不计为上游不支持。

用户依据[人工验收步骤](../../../.plans/2026-11-05-CC能力完整融合/人工验收.md)回填原生安装、启动、系统授权与必要功能结果；ARM64 与 x64 分列，不能互相替代。JSON 格式、内部链接、来源与差异核对均留待后续统一执行；本轮行为用例执行数为 0，无通过结论。编号任务卡按指令删除，持久清单和记录保留。本任务未启动应用、创建运行资源或改变用户配置，无运行资源需要清理。
