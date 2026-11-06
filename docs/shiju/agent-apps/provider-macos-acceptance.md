# 供应商 macOS 验收证据

## 任务 1：锁定供应商领域实施契约

2026-11-06；状态：已完成。产物：[provider-contracts.md](provider-contracts.md)。本项为文档契约审查，不是供应商行为、API 请求或 Iced 运行验收。

### 开工裁定

原供应商计划等待母任务 2 完整验收，而底座正式交付要求供应商等领域交接落地后才评估母任务 2 完成，形成循环。继续执行指令已裁定以底座子计划 1–93 的实际验收作为开工门槛，逐项消费真实接口；母任务 2 完整领域迁移仍未收口，不修改其状态或只读 plan.md。

母任务 1 来源快照已验收；母任务 3 当前已完成，正式 AppId/catalog、授权主机和隔离证据存在。底座提交 `1d7f1b192` 汇总双库无窗口 6 项通过与真实图标 Iced 窗口证据。本轮仅审查历史证据，没有重跑或将图标通过推及供应商切换。

原调度会话 `01a1ab42-fb8e-7af1-86f1-41f7a5eda2a2` 和任务 93 会话 `01a1ace2-4bbf-7d80-9d7c-04db07691247` 均已完成、归档，没有仍执行的前置收口会话。本轮查询非归档历史8页及归档历史15页，并读取两会话最新完成轮；原底座计划目录清理不作为能力验收证据。

### 来源、实现与人工清单

来源 `/Users/xiangmin/worker/code/cc-switch`，HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，工作区干净。主仓审查基线 `4706d3601`。

| 审查面 | 本轮读取及结论 |
| --- | --- |
| 来源函数与调用者 | provider.rs 的 Provider/ProviderMeta；commands/provider.rs 的 get_providers/get_current_provider/add_provider/update_provider/delete_provider；mode/current.rs 的 Purpose/provider_for/is_referenced。核对 lib.rs 注册、前端 providers.ts 调用、ProviderService 与 mode/controller 消费；相邻 provider_service/provider_commands 和 current 测试只读，未运行 |
| 业务与存储 | providers/mod.rs 的 validate_identity/validate_profile、apps/catalog.rs、AppProviderStore、GlobalDatabase/settings 与图标事务；确认14身份、租户键、整条 upsert、不激活、不复制数据库 |
| API / SDK | handlers/app_providers.rs 的 router/read/save，router.rs/bootstrap.rs 两处注册；DTO app_providers.rs；SDK app_providers_api.rs 的 save/get。相邻 API、SDK、store 测试断言涵盖认证、脱敏、租户/应用隔离和路径；本轮只读，不声称重跑通过 |
| Iced | components/mod.rs、state.rs、state/app_state.rs、message/mod.rs、message/settings/mod.rs、ui_types.rs 和 system_settings.rs 的真实注册位置；本计划供应商编辑 UI 尚未交付，本轮未启动或截图 |
| 逐项覆盖 | 应用能力、读写权限、秘密写入/脱敏、错误/HTTP、状态/路由、14应用格式/路径/不支持操作均有来源；任务1–77拟文件与父模块已登记；单文件与同名目录接线冲突已说明 |

完整 CRUD/当前状态、秘密 Keep/Replace/Clear、主机配置写权限、物理文件锁/pending/恢复、原生应用适配、模式/代理/聚合交叉、环境恢复、进度取消与供应商 Iced/macOS 交叉验收仍待后续任务实现，责任见契约；不能因本项完成而标为通过。母任务 2 完整迁移及母 6/8/10 完整领域验收仍未收口。

### 本轮验证

| 确切命令或检查 | 退出码 | 结果 |
| --- | --- | --- |
| `git -C /Users/xiangmin/worker/code/cc-switch rev-parse HEAD` | 0 | 来源提交一致 |
| `git -C /Users/xiangmin/worker/code/cc-switch status --short` | 0 | 无输出，干净 |
| `/Users/xiangmin/.rustcodegraph/bin/rustcodegraph status` | 0 | 索引可用，6353文件 |
| `/Users/xiangmin/.rustcodegraph/bin/rustcodegraph explore 'agent_apps providers ProviderStore ExecutionHostPolicy'` | 0 | 定位调用者，截断部分精确 node 跟进 |
| `/Users/xiangmin/.rustcodegraph/bin/rustcodegraph node --file crates/vw-agent-apps/src/providers/mod.rs --offset 1 --limit 120` | 0 | 现有校验与启动消费者 |
| `sw_vers`、`uname -m` | 0 / 0 | macOS 27.0（26A428）、arm64 |

任务要求的关键项检查实际命令：

```bash
python3 - <<'PY'
from pathlib import Path
p=Path('docs/shiju/agent-apps/provider-contracts.md')
s=p.read_text()
assert s.strip()
for key in ['macOS', 'API', 'SDK', 'Iced']:
    assert key in s, key
print('文档关键项检查通过；仍需人工逐项审查')
PY
```

链接与登记矩阵检查实际命令：

```bash
python3 - <<'PY'
from pathlib import Path
import re
root=Path('docs/shiju/agent-apps')
for name in ['provider-contracts.md', 'provider-macos-acceptance.md']:
    p=root/name
    for target in re.findall(r'\]\(([^)]+)\)', p.read_text()):
        assert (p.parent/target).exists(), (name, target)
s=(root/'provider-contracts.md').read_text()
ids=['claude','claude-desktop','codex','gemini','grokbuild','opencode','openclaw','hermes','pi','mcode','kimi','dsh','agy','omp']
for app in ids:
    assert re.search(r'^\| '+re.escape(app)+r' /', s, re.M), app
for n in range(1,78):
    assert re.search(r'^\| '+str(n)+r' \|', s, re.M), n
print('链接可达；14应用、77任务落点登记完整')
PY
```

以上两块 Python 实际执行退出均为 0，人工对照来源和注册入口完成语义审查；字符串或行数检查不等于行为验收。定向 `git diff --check`、`git diff --cached --check` 退出 0，仅暂存两份正式文档。初次通过 shell 写验收文档时嵌套 heredoc 分隔符冲突，shell 语法解析退出1、未写入文件；改用 apply_patch 后重新验证，不将该失败记为检查通过。

本轮仅文档，不改 Rust/配置/测试；TDD、有效行为测试计数、Cargo槽位不适用，不调用 Cargo/Clippy/全量测试。未验收 Windows/Linux/其他架构。任务52–77仍须供应商真实窗口和应用证据。
