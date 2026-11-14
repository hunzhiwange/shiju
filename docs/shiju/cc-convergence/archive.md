# 来源归档与恢复

2026-11-14。依赖任务18–20已删除，其静态结论见 tauri-dependencies.md、frontend.md、scripts.md。本任务不改变运行行为、不删除upstream，也不运行构建、应用或单元/实机测试。

## 归档内容与差异

持久归档：`/Users/xiangmin/.local/share/shiju/source-archives/cc-switch-ca58c031-20261114/cc-switch-source.tar.gz`。同目录另存本次归档清单副本；仓内权威记录见 `归档清单.json`。不依赖源仓、临时目录或target。

归档包含1489个来源文件（完整TS、React、Tauri后端、assets、包外测试夹具和构建上下文）以及6个来源说明/审计/母清单文件，共1495个文件。逐文件路径、字节数、SHA256、执行模式、归档哈希和恢复基线HEAD见清单。

当前快照集合与原始来源清单一致，模式一致；1488项哈希一致。唯一历史差异为 `src-tauri/src/services/subscription.rs`：引入提交已清空硬编码OAuth客户端值，原清单未同步。归档保留当前已提交脱敏内容，不恢复这些值；差异两端哈希已明确登记。因此不能声称归档等于未经脱敏的原始源提交。本次恢复严格以本次归档清单为准，原清单作为历史证据留存。

完整 `upstream/LICENSE` 随归档保存，MIT，Copyright (c) 2025 Jason Young；许可哈希见清单。恢复或再分发衍生源码时必须保留完整版权及许可文本。

## 能力映射

来源母任务与14域所有者、正式API/SDK/Iced索引见 `准入与所有者.md`。各域已绑定生产符号、现有测试位置及保留边界：供应商providers.md、模型models.md、账号accounts.md、配置config.md、MCP mcp.md、技能插件skills.md、提示词prompts.md、会话sessions.md、用量usage.md、数据库database.md、网络network.md、调度scheduling.md、事件events.md、安装installation.md；代理relay.md、IPC ipc.md及本批次三个边界文档补充交叉映射。它们均为静态依据，不能替代尚未执行的统一行为验证。自动上传计划合同仍未支持，见scheduling.md，不因归档而升级能力声明。

## 完整恢复命令

在仓库根执行以下命令；先解压至一个新目录，不直接覆盖现有生产树。完成内容核对后才按需要复制指定路径；禁止执行归档中的构建脚本或包生命周期。

```bash
python3 - <<'PY'
import hashlib, json, pathlib, tarfile, tempfile
m = json.loads(pathlib.Path('docs/shiju/cc-convergence/归档清单.json').read_text())
a = pathlib.Path(m['归档绝对路径'])
assert hashlib.sha256(a.read_bytes()).hexdigest() == m['归档SHA256']
expected = {x['路径']: x for x in m['文件']}
d = pathlib.Path(tempfile.mkdtemp(prefix='shiju-source-recovery-'))
with tarfile.open(a) as t:
    assert set(t.getnames()) == set(expected)
    for e in t.getmembers():
        p = pathlib.PurePosixPath(e.name)
        assert e.isfile() and not p.is_absolute() and '..' not in p.parts
    t.extractall(d)  # 所有成员已限定为安全相对路径普通文件
assert {str(p.relative_to(d)) for p in d.rglob('*') if p.is_file()} == set(expected)
for name, x in expected.items():
    p = d / name
    assert not p.is_symlink()
    assert hashlib.sha256(p.read_bytes()).hexdigest() == x['SHA256']
    assert oct(p.stat().st_mode & 0o777) == x['模式']
print(d)
PY
```

本次使用同等Python静态核对实际解压至隔离临时目录：1495文件，无缺失、多余、哈希或模式差异，随后清理该临时目录；持久归档与清单保留。初次操作遇本机Python不支持extractall的filter参数，改为先逐成员拒绝绝对路径、父路径、链接及特殊文件再解压，成功完成；未运行来源代码。

恢复提交位置为清单的 `归档前HEAD`；该HEAD保存当前脱敏快照。无需重置整个工作区。若使用Git恢复指定文件，先保存本地差异并仅从该提交提取对应路径，不能覆盖其他并行任务变更。本次仅新增归档证据，未修改来源/母计划/原始哈希清单。
