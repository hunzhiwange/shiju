# 来源快照移除

2026-11-14。任务21归档提交 `64ad9bee4`，来源树已在删除前逐项核对1489文件的严格路径集合、SHA256与模式，均与本次归档清单一致。持久归档SHA256为 `1b87ba3211faf0efba8477ad9f659845d3ff11364e53ff733e89fb37ca894d0d`，文件可读且哈希一致。完整恢复步骤与历史脱敏差异见 [archive.md](archive.md)，14域及代理、IPC、前端、依赖、脚本边界静态证据均已交付。

## 实际修改

- 删除 `crates/vw-agent-apps/upstream` 的1489个来源文件，未操作独立原仓、用户数据或正式生产模块。
- 删除根 `Cargo.toml` 已失效的该来源 `workspace.exclude`，保留其他成员、排除项及锁文件。
- 唯一真实构建引用为 `src/mcp/adapters/codex_tests.rs` 的 `include_str!`，改读本包 `tests/fixtures/mcp/codex-config.toml`。夹具原字节保留，SHA256 `3803462b3f9a29b179a548cbb20e8864456a2b4dd573885743312d18e974758b`；相对路径从 adapters 上溯至包根，再进入 tests。
- 完整 MIT 版权与许可移至包根 `CC-Switch-LICENSE`，SHA256 `912b6a597d10c43b40a0909349ed95b052b17efb6502b4898e1b35dafb896755`。来源说明增加当前状态，原始来源清单与历史验收记录保留。

## 引用分类与验证边界

删除前以 `rg` 查正式 crates/scripts/根清单的 `upstream/src-tauri`、`vw-agent-apps/upstream` 与相对 upstream 路径，实际依赖只有上述夹具与 exclude；迁移后无这些运行/测试路径。`cc-switch` 供应商稳定key、User-Agent、兼容配置文件名、来源归属和历史文档不等于旧运行依赖，保留；`cc_switch_lib` 无正式模块依赖。来源清单、归档清单和历史报告内的旧路径用于恢复与审计，不批量改写。

RustCodeGraph先检查status并查询MCP原测试入口，原图包含已移除来源，只作删除前导航，删除后结论以当前文件静态搜索为准。静态差异与夹具可达性检查完成；按用户本轮要求，未运行任何Cargo命令（含metadata/tree）、构建、单元测试、实机测试或回归，不能据此声明编译、业务等价或macOS验收通过。未来统一验证需覆盖MCP夹具消费者与全量生产依赖图。

## 恢复

优先按archive.md从持久归档恢复到新目录并核对清单，不覆盖并行工作区。删除前当前Git版本亦保存完整脱敏来源。仅恢复所需来源路径和本任务清单/夹具引用差异，不能重置整个工作区；若要重建原隔离来源构建入口，再恢复相应exclude。保留本次当前许可与正式原生代码。
