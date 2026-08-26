---
name: rustcodegraph
description: 使用原生工具或后备命令行界面，通过现有 RustCodeGraph 索引理解、导航或分析代码库。当用户要求使用 RustCodeGraph、需要高性能符号/源码/调用流检索、调用方/被调用方/影响分析或受影响测试选择时使用。
---

# RustCodeGraph

将 RustCodeGraph 作为当前工作区的预计算代码地图使用：符号搜索、源码片段、
调用边和影响半径都来自本地 `.rustcodegraph/` 索引。

## 选择入口

先检查当前会话的可用工具目录，再执行任何 shell 命令：

1. 如果工具目录存在原生 `rustcodegraph`，直接调用它；不要定位或执行 CLI。
2. 如果原生工具缺席，才进入「CLI 后备」并探测绝对可执行文件路径。
3. 没有 `.rustcodegraph/` 索引时，使用普通代码搜索与文件读取工具；不要自动创建索引。

## 原生工具

原生工具已注册意味着当前精确工作区存在有效索引。大多数调研先用 `explore`，
精确名称查找用 `search`，单个符号用 `node`：

```json
{"operation":"explore","query":"..."}
{"operation":"search","query":"..."}
{"operation":"node","symbol":"..."}
{"operation":"callers","symbol":"..."}
{"operation":"callees","symbol":"..."}
{"operation":"impact","symbol":"..."}
{"operation":"files"}
{"operation":"status"}
{"operation":"sync"}
```

CLI `query` 对应原生 `search`；不要把 `query` 作为原生 operation。CLI `affected`
没有原生 operation，需要选择受影响测试时使用其它可用的项目工具。

原生工具只访问当前工作区，不接受 `projectPath`，也不会创建索引。普通保存的编辑由
监视器增量刷新；切换分支、拉取或批量生成文件后调用一次 `sync`。如果状态或查询报告
pending、stale 或 degraded，调用一次 `sync` 后重试原操作，不要在每次查询前同步。

## CLI 后备

仅当原生 `rustcodegraph` 工具不在当前工具目录时使用本节。先从项目根目录检查
`.rustcodegraph/`；不存在就立即回退普通工具，不要为了使用 CLI 自动运行 `init`。

不要依赖 PATH、`$HOME` 或 `RUSTCODEGRAPH_*` 环境变量。受限 agent shell 常清空环境，
导致全局安装的裸命令失败。会话内只解析一次绝对路径，保存到 `RCG`，后续一律使用
`"$RCG"`。

### macOS / Linux

使用未加引号的 `~`，让 shell 从 passwd 展开：

```bash
RCG=""
for c in \
  ~/.rustcodegraph/bin/rustcodegraph \
  ~/.rustcodegraph/current/bin/rustcodegraph \
  ~/.local/bin/rustcodegraph \
  ~/.cargo/bin/rustcodegraph \
  /opt/homebrew/bin/rustcodegraph \
  /usr/local/bin/rustcodegraph
do
  if [ -x "$c" ]; then RCG=$c; break; fi
done
if [ -z "$RCG" ] && [ -d ~/.rustcodegraph/versions ]; then
  RCG=$(ls -1d ~/.rustcodegraph/versions/*/bin/rustcodegraph 2>/dev/null | sort -V | tail -1)
  [ -x "$RCG" ] || RCG=""
fi
[ -n "$RCG" ] || RCG=$(command -v rustcodegraph 2>/dev/null || true)
```

### Windows (PowerShell)

```powershell
$candidates = @(
  "$env:LOCALAPPDATA\rustcodegraph\current\bin\rustcodegraph.exe",
  "$env:USERPROFILE\.rustcodegraph\bin\rustcodegraph.exe",
  "$env:USERPROFILE\.rustcodegraph\current\bin\rustcodegraph.exe",
  "$env:USERPROFILE\.local\bin\rustcodegraph.exe",
  "$env:USERPROFILE\.cargo\bin\rustcodegraph.exe"
)
$RCG = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $RCG) { $RCG = (Get-Command rustcodegraph -ErrorAction SilentlyContinue)?.Source }
```

找不到 CLI 时回退普通代码搜索，并告知用户 RustCodeGraph CLI 不可用；不要猜测其它路径。

### CLI 工作流

确认绝对路径后运行 `"$RCG" status`。索引无效、为空或项目未初始化时停止使用
RustCodeGraph 并回退普通工具。索引可用时按意图选择命令：

| 意图 | CLI 命令 |
| --- | --- |
| 调研代码流 | `"$RCG" explore "<symbols or question>"` |
| 精确名称搜索 | `"$RCG" query <name> --kind <kind> --limit 10` |
| 展示符号或文件片段 | `"$RCG" node <symbol>` / `"$RCG" node --file <path> --offset <n> --limit <n>` |
| 查找调用边 | `"$RCG" callers <symbol>` / `"$RCG" callees <symbol>` |
| 估算重构影响 | `"$RCG" impact <symbol> --depth 3` |
| 列出索引文件 | `"$RCG" files --filter <dir>` |
| 选择受影响测试 | `"$RCG" affected <files>` |

切换分支或批量修改后先运行一次 `"$RCG" sync`。需要稳定机器可读输出时添加
`--json`。只有用户明确要求启用或重建索引时，才运行 `"$RCG" init -i`。

## 使用规则

- 将 `explore` 输出视为已经读过的源码上下文，不要立即用普通搜索重复读取。
- 处理未索引文件、图未覆盖的文档/配置或新鲜度警告点名的文件时，使用普通工具。
- 对流程问题，把用户给出的端点符号、文件名和关键桥接名称放进同一个精确查询。
- 对重载或常见名称，使用带类型/文件提示的 `node` 或精确搜索消除歧义。
