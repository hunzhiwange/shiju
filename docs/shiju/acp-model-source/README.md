# ACP CLI 版本与模型来源证据契约

当前模型来源切换能力、五项未支持原因和后续对比基线见
[模型来源切换支持状态（2026-11-05）](support-status.md)。下文保留最初调查快照，
其中“未验收”等表述不代表当前运行时能力。

2026-11-04 公共资料及本机只读探测快照。`cli-versions.json` 覆盖注册表的 22 个稳定 key，
用于后续任务选择精确发行版、设计覆盖与隔离验证。版本契约与证据清单已交付；**没有任何条目通过模型接管验收**。
此文件不锁定安装器、不改变运行时启动、不安装或更新 CLI。

## 如何读取

- `distribution` 是 ACP 启动进程所属发行包。npm/PyPI 使用精确版本 URL 和可取得的源码提交、integrity；
  `latest` 的解析结果只是当天快照。原生安装器的版本声明与发行 manifest 单独记录；均不等于已安装版本。
- `underlying` 区分同包原生引擎、适配器使用的 SDK、外部 CLI。适配器版本不能代替底层引擎版本。
- Claude 选择调查基线 0.26.0，SDK 声明为 0.2.96；Pi 选择 0.0.22。
  这是候选契约，未声称已解析整个 `^0.26.0` 范围的最高版本。
  Codex ACP 2.5.0 的 `@openai/codex@^0.165.0` 已解析为候选 `0.165.0`；
  Claude SDK 的 `package.json::claudeCodeVersion` 明确为 `2.1.96`。两者仍须核对实际启动的引擎。
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

22 项中有 13 项原生候选、4 项桥接、5 项上游阻塞。22 个发行包候选均取得版本资料：
17 个 npm/PyPI 包及下列 5 个原生发行入口。它们不是 22 个实际已安装二进制。
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

## 补证：发行资料与安全替代方案

本次只读查询官方安装器和 manifest，并以数据方式检查 4 个 npm 原始发行归档。
归档不执行、不应用补丁；在 JSON 中保留 SHA256、npm integrity 与所检查文件的哈希。
Claude SDK、Codex ACP、Pi ACP、Auggie 的包身份/许可证不再只依赖可变 README。

| key | 官方版本证据 | 可用于后续独立安装的路径 |
| --- | --- | --- |
| cursor | [安装器](https://cursor.com/install) 的下载和安装目录均嵌入 `2026.10.22-27dcf5f` | 直接取得版本化 darwin/arm64 归档；[配置参考](https://cursor.com/docs/cli/reference/configuration) 另声明 `CURSOR_CONFIG_DIR`，隔离仍待验证 |
| droid | [安装器](https://app.factory.ai/cli) 声明 `1.3.0`；[官方校验和](https://downloads.factory.ai/factory-cli/releases/1.3.0/darwin/arm64/droid.sha256) 已取得 | 直接下载二进制并核对 SHA256；不执行会杀死现有 droid 并写用户 bin 的安装器 |
| goose | [官方 release](https://github.com/aaif-goose/goose/releases/tag/v1.57.0) 为 `v1.57.0`，资产提供 SHA256 digest | 安装器支持 `GOOSE_VERSION`、`GOOSE_BIN_DIR`、`CONFIGURE=false`；本任务只记录，不运行安装 |
| kiro | [manifest](https://prod.download.cli.kiro.dev/stable/latest/manifest.json) 与[版本索引](https://prod.download.cli.kiro.dev/stable/index.json) 均有 `3.0.0`，独立 ZIP 有校验和 | 直接取得版本化 standalone ZIP 并校验，不运行 bundled install.sh 或安装 DMG |
| trae | [安装器版本指针](https://lf-cdn.trae.com.cn/obj/trae-com-cn/trae-cli/trae-cli_latest_version.txt) 为 `v0.121.3`；安装器将 `traecli` 链接到 `trae-cli` | 直接取得归档可避免卸载旧版本、改用户 XDG/bin、写 shell profile；**0.121.3 与 2.0 ACP 文档代际是否匹配尚未证明** |

Goose 固定 release LICENSE、Qwen 固定源码提交 LICENSE、Kimi 固定 release LICENSE 均已检查为 Apache-2.0；
[Kiro 许可](https://kiro.dev/license/) 和 [Cursor 条款](https://cursor.com/terms-of-service) 也已检查。
这不代替每个实际二进制所带第三方声明的审计。iFlow 固定提交的 `/LICENSE` 请求为 404，
缺失声明仍明确保留，不从其他项目猜测许可。

本任务步骤 4 要求将安装需求交给对应 CLI 任务，因此没有批量安装 19 个缺失入口。
独立目录获取发行版是可行替代方案，**全局安装限制并非这些下载的技术阻塞**。
本任务资料结构验证与后续真实 driver 模型验收分开：前者已通过，后者未执行。
按用户最新明确的任务 1 边界，未安装及入口未证实的事实保留在清单，
不阻塞版本契约交付；真实 CLI 验收和上游接口问题由对应适配任务继续处理。

后续适配任务的最小解除条件：五项 C 分别提供与选定发行版一致的官方 Provider/扩展入口，
包括任意 endpoint、认证、model、ACP 读取及禁止厂商模型回退的可验证契约；
然后在独立目录用真实 CLI 补齐所需证据。Cursor 的托管 SDK、Kiro 的 harness、Qoder 的账号模型选择、
TRAE 的工具插件不能替代这些条件；Auggie 必须先对齐 0.36.0 发行包与 1.x 扩展文档。
无需对 22 项先实现协议网关或执行付费模型调用来验证资料 JSON，也不会将这些后续实现误称为本任务交付。

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
    assert e['distribution']['release'] and e['underlying'] and e['license']['review']
    if 'native_release_evidence' in e:
        assert e['native_release_evidence']['metadata_http_status'] == 200
        assert len(e['native_release_evidence']['installer_sha256']) == 64
        assert e['native_release_evidence']['binary_executed'] is False
    assert set(e['platforms']) == {'windows', 'macos', 'linux'}
    assert e['acp_launch'] and e['config_priority'] and e['auxiliary_model_routes']
    if e['route'] == 'upstream_blocked':
        assert e['official_extension_review']['source_urls']
        assert e['official_extension_review']['runtime_proof'] is False
    if e['installed']['status'] == 'not_found':
        assert e['installed']['exit_code'] is None and 'output' not in e['installed']
raw = p.read_text()
assert '/Users/' not in raw
assert not re.search(r'[?&](?:sig|jwt|token|access_token)=', raw)
assert not re.search(r'sk-[A-Za-z0-9_-]{16,}|gh[pousr]_[A-Za-z0-9]{20,}|Bearer\s+\S+', raw)
assert not re.search(r'https://[^/\s]+:[^/\s]+@', raw)
assert not re.search(r'"(?:api_key|access_token|refresh_token|password)"\s*:', raw)
print('PASS: 22 keys; identities/routes/platforms/evidence; source hashes; credential-pattern scan')
PY
```

链接检查通过 Python `urllib.request` 对公共证据 URL 发起 GET（15 秒超时，只读首 1024 字节），
结果逐项保留在 `source_link_checks` 和 `distribution.link_check`，首轮总数和失败 URL 见 `link_check_summary`；本轮新增 28 个公共链接均为 HTTP 2xx，
完整结果保留于 `supplemental_link_checks`。
HTTP 成功仅证明链接可达；失败链接保留原因，不替换为虚构证据，不将 TLS/网络失败视作 CLI 不支持。
可重新对这些 URL 运行同样 GET 检查。没有执行安装器脚本。

本任务为纯资料清单，按任务要求不执行 Cargo、CLI 真实模型测试或业务单元测试。
