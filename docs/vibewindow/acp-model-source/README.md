# ACP CLI 版本与模型来源证据契约

2026-11-04 公共资料及本机只读探测快照。`cli-versions.json` 覆盖注册表的 22 个稳定 key，
用于后续任务选择精确发行版、设计覆盖与隔离验证。当前契约仍有阻塞，**没有任何条目通过模型接管验收**。
此文件不锁定安装器、不改变运行时启动、不安装或更新 CLI。

## 如何读取

- `distribution` 是 ACP 启动进程所属发行包。npm/PyPI 使用精确版本 URL 和可取得的源码提交、integrity；
  `latest` 的解析结果只是当天快照。原生安装器无法固定发行号时明确为 `null`。
- `underlying` 区分同包原生引擎、适配器使用的 SDK、外部 CLI。适配器版本不能代替底层引擎版本。
- Claude 选择调查基线 0.26.0，SDK 声明为 0.2.96；Pi 选择 0.0.22。
  这是候选契约，未声称已解析整个 `^0.26.0` 范围的最高版本。
  Codex ACP 2.5.0 依赖 `@openai/codex@^0.165.0`，实际引擎仍须固定。
- `license.declared` 直接保留包元数据声明；`SEE LICENSE`、缺失值及未审计的内嵌依赖均不是再分发许可结论。
  原生安装器的许可证与发行号仍需逐平台核对。
- `native_config_candidate` 表示文档存在覆盖候选；`bridge_required` 表示协议、Gateway 或隔离需桥接/补丁；
  `upstream_blocked` 表示未证明当前发行版有可用入口。三者均不代表 supported。
- `config_root_candidate`、`config_priority`、`auxiliary_model_routes` 是后续验证要求；
  `config_isolation_verified=false` 明确没有通过配置隔离验收。`null` 根目录表示尚无已证实入口，不能用替换 HOME 猜测兼容。
- `installed` 仅记录可执行文件发现和 `--version` 结果。`not_found` 的命令**没有执行**，退出码为 `null`，
  不能伪造为 CLI 退出 127；本机发现范围为 PATH、注册表使用的 `~/.local/bin`、OpenCode 专属 bin。
  这不代表其他主机、全盘或 Windows 上没有安装。
- `runtime_verified` 全部为 `false`，原因单列；版本输出、npm 包、官方文档或 mock 均不构成实际模型请求证据。
- 三平台独立记录。当前仅 macOS 进行版本发现；Linux/Windows 未执行，不从安装命令推断支持。
  Windows 使用 npm `.cmd`/原生 `.exe` 或 PowerShell 安装器；Unix 安装器也不能证明每个 macOS/Linux 架构可用。

`acp_launch` 保留当前注册的本地启动参数，包括已有权限参数，仅作调查记录，不新增或扩大权限。
`registry_default_args` 保留 Rust 表达式，常量定义以注册表为准。本地命令优先于 npx，
所以包元数据版本不能约束已安装命令；以后必须同时记录实际命令、适配器与底层版本。
Kilo 包元数据确实同时声明 `kilo` 与 `kilocode`；Snow 身份是 `snow-ai`，不是同名数据平台工具。

## 当前证据与缺口

22 项中有 13 项原生候选、4 项桥接、5 项上游阻塞。17 个 npm/PyPI 包取得精确发行号；
Cursor、Droid、Goose、Kiro、TRAE 的原生安装发行号未固定。
本机 3 个版本探测退出码为 0：Copilot `1.0.105`、Gemini `0.66.0`；
Claude ACP 无版本输出，通过已解析可执行文件上级的 **package.json 包元数据**识别为 `0.26.0`。
其余 19 个入口在上述发现范围未找到。没有读取用户 auth/config 内容，没有模型请求或登录操作。

五项 C 已补查官方扩展/SDK 页面，详细结论和 URL 记录在 `official_extension_review`：

| key | 入口调查结果 | 解除阻塞所需证据 |
| --- | --- | --- |
| auggie | [1.x 官方入口](https://docs.augmentcode.com/cli/reference) 链接 Pi 扩展；[Pi 扩展](https://raw.githubusercontent.com/earendil-works/pi/main/packages/coding-agent/docs/extensions.md) 有 Provider 注册 API。当前 npm 快照却是 0.36.0 | 固定同一代实际 Auggie，证明 ACP 加载 Provider、无需厂商模型登录且不回退；不能将 Pi 源码能力直接归给 Auggie |
| cursor | [官方 SDK](https://cursor.com/docs/sdk/typescript) 明确 local/cloud 均使用 Cursor 托管模型 | 官方任意 Provider 契约与原 CLI ACP 的实际请求证明 |
| kiro | [harness 入口](https://kiro.dev/docs/cli/agent-harness/) 只规定 V3/V2；[设置](https://kiro.dev/docs/reference/settings/) 有全局/工作区优先级 | 任意 endpoint/key 的入口与配置隔离证明；固定版本后核对 ACP harness |
| qoder | [SDK](https://docs.qoder.com/cli/sdk/overview) 包含/启动 qodercli；[模型选择](https://docs.qoder.com/cli/sdk/model-policy) 未证明 endpoint 注册；[BYOK](https://docs.qoder.com/cli/custom-models) 限于账号向导目录 | 程序化注册任意 Provider，而非仅选择已有账号模型；证明 ACP 使用该注册 |
| trae | [插件页面](https://docs.trae.cn/cli_tools-and-extensions) 说明命令、技能、工具；[配置](https://docs.trae.cn/cli_config-file) 未证明模型 Provider hook | 原 `traecli` 的 Provider/模型 RPC 契约及线协议证明；不能使用另一个 trae-agent 替代 |

以上“未证明”限于已检查资料，不断言厂商永远无法支持。没有联系厂商或提交 issue。
后续任务 9 处理隔离边界，任务 10 建立真实 CLI 离线验收工具，任务 11–32 逐项处理入口与适配；
必须先补齐真实 CLI 与发行版证据，不能将这里的缺口改成 supported。

## 参考实现边界

只读检查参考项目 `packages/desktop/shiju/backend/manifest.json`、`cli-models/README.md`、
`cli-models/aioncore.patch` 及团队补丁清单。manifest 的 `patchSet` 为 `shiju-2026-10-28.2`，
AionCore 基线为 `47e66d0d151123e973b3fd1e77afcb5671b3f8c5`。
四项覆盖是 claude/codex/qwen/opencode；其原生参数不可直接移植到 ACP 适配器。
参考 README 明确没有真实模型请求验收；补丁应用顺序还包括模型窗口和 Claude 首轮恢复，
不能以补丁存在证明已交付能力。

## 复验命令

在仓库根目录运行以下 Python 标准库检查；这是资料集合/结构验收，不是 CLI 模型测试。

```sh
python3 - <<'PY'
import hashlib, json, re
from pathlib import Path
p = Path('docs/vibewindow/acp-model-source/cli-versions.json')
d = json.loads(p.read_text())
r = Path('crates/vw-acp/src/agent_registry.rs')
s = r.read_text().split('const BUILT_IN_AGENT_DEFINITIONS:')[1].split('const AGENT_ALIASES:')[0]
keys = re.findall(r'\bname: "([a-z]+)"', s)
entries = d['entries']
assert len(entries) == len(keys) == 22
assert len({e['key'] for e in entries}) == 22
assert {e['key'] for e in entries} == set(keys)
assert hashlib.sha256(r.read_bytes()).hexdigest() == d['registry_sha256']
i = Path('crates/vw-config-types/src/acp_install.rs')
assert hashlib.sha256(i.read_bytes()).hexdigest() == d['installer_sha256']
for e in entries:
    assert e['route'] in {'native_config_candidate', 'bridge_required', 'upstream_blocked'}
    assert e['source_urls'] and all(u.startswith('https://') for u in e['source_urls'])
    assert e['runtime_verified'] is False and e['runtime_evidence_reason']
    assert e['config_isolation_verified'] is False
    assert e['distribution'] and e['underlying'] and e['license']['review']
    assert set(e['platforms']) == {'windows', 'macos', 'linux'}
    assert e['acp_launch'] and e['config_priority'] and e['auxiliary_model_routes']
    if e['route'] == 'upstream_blocked':
        assert e['official_extension_review']['source_urls']
        assert e['official_extension_review']['runtime_proof'] is False
    if e['installed']['status'] == 'not_found':
        assert e['installed']['exit_code'] is None and 'output' not in e['installed']
raw = p.read_text()
assert '/Users/' not in raw
assert not re.search(r'sk-[A-Za-z0-9_-]{16,}|gh[pousr]_[A-Za-z0-9]{20,}|Bearer\s+\S+', raw)
assert not re.search(r'https://[^/\s]+:[^/\s]+@', raw)
assert not re.search(r'"(?:api_key|access_token|refresh_token|password)"\s*:', raw)
print('PASS: 22 keys; identities/routes/platforms/evidence; source hashes; credential-pattern scan')
PY
```

链接检查通过 Python `urllib.request` 对公共证据 URL 发起 GET（15 秒超时，只读首 1024 字节），
结果逐项保留在 `source_link_checks` 和 `distribution.link_check`，总数和失败 URL 见 `link_check_summary`。
HTTP 成功仅证明链接可达；失败链接保留原因，不替换为虚构证据，不将 TLS/网络失败视作 CLI 不支持。
可重新对这些 URL 运行同样 GET 检查。没有执行安装器脚本。

本任务为纯资料清单，按任务要求不执行 Cargo、CLI 真实模型测试或业务单元测试。
