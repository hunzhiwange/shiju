# 来源覆盖核对（任务 1）

2026-11-14，仅静态来源与契约核对，功能和 macOS 行为未验收。

固定来源 `/Users/xiangmin/worker/code/cc-switch`，提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7`。通过 Python hashlib.sha256 分别读取 `git show <提交>:<路径>` 与当前文件，对照子计划来源覆盖表及母来源清单：81/81 条哈希相符，无缺文件、无列出文件差异。该只读核对命令退出0；不是测试通过证据。

`git -C /Users/xiangmin/worker/code/cc-switch status --porcelain` 退出0，当前另有 `M package.json`、`?? demo/`，均在这81条之外；不能称整个来源工作树干净。未修改来源树。上游 `commands/usage.rs` 使用 Tauri State/run_db_query，真实包装为 `src/lib/api/usage.ts` 的 usageApi；不直接搬入网关。来源测试文件属于移植参考，本轮未运行。

## 11 条原生扫描路径

以下路径以来源 `src-tauri/src/services/` 为根，均由 `session_usage.rs::sync_all_unlocked` 显式注册；只有来源注册得到核对，ShiJu 统一扫描器尚待各编号任务实现。

| 来源身份 | 来源文件/入口 | 后续任务与边界 |
| --- | --- | --- |
| Claude | `session_usage.rs::sync_claude_session_logs` | 13/14；Desktop共享Claude身份，不另计一条 |
| Codex | `session_usage_codex.rs::sync_codex_usage` | 15—18；累计差分、续接/子代理、性能与安全重建 |
| Gemini | `session_usage_gemini.rs::sync_gemini_usage` | 19 |
| OpenCode | `session_usage_opencode.rs::sync_opencode_usage` | 20；只读数据库 |
| GrokBuild | `session_usage_grokbuild.rs::sync_grokbuild_usage` | 21 |
| Pi | `session_usage_pi.rs::sync_pi_usage` | 22/23；主会话和子代理去重 |
| OMP | `session_usage_omp.rs::sync_omp_usage` | 24；不得误扫为Pi |
| MCode | `session_usage_mcode.rs::sync_mcode_usage` | 25 |
| AGY | `session_usage_agy.rs::sync_agy_usage` 及 `session_usage_agy/native_models.rs` | 26；原生模型与时序证据 |
| Kimi | `session_usage_kimi.rs::sync_kimi_usage` | 27；sessions下agents/*/wire.jsonl |
| DSH | `session_usage_dsh.rs::sync_dsh_usage` | 28；DSH_HOME/sessions，两端共用目录只扫一次 |

Hermes/OpenClaw不在该注册表，不能因存在会话读取器而宣称支持原生用量。所有扫描根只允许从获准主机的应用路径及服务端 SourceRegistry 取得；不打开用户真实日志、不接受任意客户端路径。不同来源的统一身份由任务29负责，路由与识矩记录分别由30/31负责。

完整81条文件哈希和子任务映射沿用 `.plans/2026-11-05-统一用量价格与性能分析/来源覆盖.md` 和母 `来源清单.json`，不重复维护副本。公共入口与缺口见 [前置契约](dependencies.md)。后续验证统一安排；本轮没有 Cargo、单元测试、实机测试、最终回归或 Git 提交。
