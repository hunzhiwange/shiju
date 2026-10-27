# VibeWindow 原包更新协议

本协议只服务 VibeWindow（GitHub 仓库 `hunzhiwange/shiju`）。OSS bucket 为
`shiju-cloud`，独立前缀为 `agents/shiju-agent/`，公开域名为
`https://www.shiju.cloud`。这些路径是新设计，生产权限尚未实测，不代表已可公开访问。
禁止覆盖 `agents/data-gateway-local-agent/` 或旧产品 `agents/` 下的固定安装包。

## 本地准备

```sh
python3 scripts/prepare_oss_release.py --version 0.5.10 --output-dir /tmp/vibewindow-release --metadata-only
python3 scripts/prepare_oss_release.py --version v0.5.10 --output-dir /tmp/vibewindow-release
```

版本必须显式指定三个非负整数，不接受前导零、预发布或路径分隔符，不自动选择 latest。
脚本只用 Python 标准库，从 GitHub Release API 读取指定 tag，拒绝草稿、预发布、
错误标签以及缺少 SHA-256 digest 的资产。可选 `GITHUB_TOKEN` 只发送给 GitHub API；
API 重定向直接拒绝。安装包下载不附带该令牌，不读取 OSS 凭据，也不写入 OSS。

预览写入 `output-dir/vX.Y.Z/manifest.preview.json`，不下载安装包。
预览不是就绪清单，发布器必须拒绝将其用作正式发布输入。
普通模式将六个原包写入同一目录，全部通过大小及 SHA-256 校验后才原子写入
`manifest.json`。再次普通执行会先移除旧就绪清单，避免失败后留下错误的就绪标记。
已有正确原包直接复用；已有不同内容拒绝覆盖，需要使用者移走冲突文件再重试。

流式读写使用 1 MiB 缓冲，单次网络操作超时 30 秒，最多尝试三次，间隔 1、2 秒。
下载先写 `.part`，校验后在同一文件系统以原子硬链接发布最终名称并删除临时名称，
避免覆盖已有文件；本地输出文件系统必须支持硬链接。失败删除临时文件；残留 `.part`
不会被当作完整文件，下次从头下载该包。不得并发操作同一输出目录。
不解压、不二次压缩、不生成 ZIP。

## 六平台映射

| target | kind | name |
| --- | --- | --- |
| aarch64-unknown-linux-gnu | rpm | shiju-X.Y.Z-1.aarch64.rpm |
| x86_64-unknown-linux-gnu | rpm | shiju-X.Y.Z-1.x86_64.rpm |
| aarch64-pc-windows-msvc | msi | ShiJu-aarch64-pc-windows-msvc-dist.msi |
| x86_64-pc-windows-msvc | msi | ShiJu-x86_64-pc-windows-msvc-dist.msi |
| aarch64-apple-darwin | dmg | ShiJu-macos-aarch64-apple-darwin-dist.dmg |
| x86_64-apple-darwin | dmg | ShiJu-macos-x86_64-apple-darwin-dist.dmg |

只按上述精确文件名选择，缺失或重复直接失败。其它 CLI/desktop/webview ZIP/TAR
资产被忽略。顺序固定为上表顺序，不受 API 数组顺序影响。

## 清单与发布边界

JSON 顶层字段：`schema_version: 1`、`version: "X.Y.Z"`、
`published_at`（GitHub 发布时间）、`release_url`（原始 Release 页面）、`assets`。
每项字段固定为：

- `name`：表中的原始文件名。
- `target`、`kind`：表中的平台和安装器类型。
- `url`：`https://www.shiju.cloud/agents/shiju-agent/releases/vX.Y.Z/<name>`。
- `github_url`：`https://github.com/hunzhiwange/shiju/releases/download/vX.Y.Z/<name>`。
- `size`：正整数，单位 bytes。
- `sha256`：GitHub `digest` 去掉 `sha256:` 后的 64 位小写十六进制。

清单不含 `binary_name`；原包是系统安装器输入，不能当作裸可执行文件覆盖当前程序。
现有二进制替换更新路径不适用于 RPM/MSI/DMG，消费方必须明确支持安装器交接。

后续发布器的不可变版本目录为 `agents/shiju-agent/releases/vX.Y.Z/`，
唯一更新指针为 `agents/shiju-agent/latest/manifest.json`。
必须先上传并回读校验全部版本对象，最后推进 latest；发布同版本不同内容必须失败。
本准备脚本不实现生产上传、安装器启动、本机更新或凭据迁移。

## 聚焦验证

```sh
python3 -m unittest discover -s scripts/tests -p test_prepare_oss_release.py
python3 scripts/prepare_oss_release.py --help
```

测试使用固定中性 fixture 和本地 HTTP 服务器，覆盖资产选择、元数据拒绝、预览、
重试、截断、哈希不符、临时文件恢复与现有完整文件复用，不下载生产安装包。

## OSS 发布入口与安全限制

```sh
uv run scripts/publish_oss_release.py --dir /absolute/output/v0.5.10
# 仅在已明确授权生产发布时：
uv run scripts/publish_oss_release.py --dir /absolute/output/v0.5.10 --publish
```

Python >=3.10，PEP 723 固定 `oss2==2.19.1`。默认离线预览，不读取凭据。
只接受正式 manifest.json 与精确六包，再验 schema、字段、映射、URL、大小和 SHA-256。
路径必须绝对且没有符号链接组件；macOS `/tmp` 可改用 `/private/tmp`。
单包限制为 OSS 单 PUT 的 5 GiB。不支持 bucket/key 覆盖参数或 `--workspace`。

发布模式仅从本机 `~/.config/shiju-oss-upload/config.json` 读取 region、access_key_id、
access_key_secret；密钥可通过 OSS_ACCESS_KEY_ID、OSS_ACCESS_KEY_SECRET 覆盖。
names 不参与发布，不修改配置，不迁移凭据，不输出 SDK 原始异常。

2026-10-27 核实的官方能力：

- [PutObject](https://www.alibabacloud.com/help/en/oss/developer-reference/putobject)
  支持 `x-oss-forbid-overwrite: true`；但 bucket 版本控制开启或暂停时无效。
  因此发布前必须成功读取 bucket versioning，且状态为空（从未启用）；未知或无权限直接失败。
- [错误 0017-00000245](https://www.alibabacloud.com/help/en/oss/user-guide/0017-00000245)
  明确 PutObject 不支持 If-Match、If-None-Match 等条件头。
- [CopyObject](https://www.alibabacloud.com/help/en/oss/developer-reference/copyobject)
  的条件针对源对象，不能证明目标 latest 的比较并交换。

因此当前入口支持首次发布和相同字节版本恢复，**拒绝推进已有 latest 至更高版本**。
这是安全失败，不是完整的连续升级发布能力；不能删指针或使用写前再读绕开限制。
后续需要有官方保证的目标 CAS 或经证明的等效协议，才可开放版本推进。
运维必须保证发布期间不启用/暂停 bucket 版本控制，也不能由其他客户端无条件覆盖对象。

状态机先读取 latest（仅 NoSuchKey 的 404 算首次）并比较稳定版本；降级、同版本
不同字节、未知状态、鉴权错误均失败。七个不可变对象一律禁止覆盖，冲突或传输结果
不确定时从 OSS 完整回读，匹配才复用；不匹配停止。上传后匿名流式回读七对象，
全部通过才禁止覆盖创建 latest，再匿名完整回读。版本缓存为一年 immutable，latest 为
`no-store, no-cache, max-age=0, must-revalidate`。不自动重试写入或修改 ACL/CDN。
公网目录权限仍未实测，离线 HTTP 测试不能替代生产匿名验证。

本地回执 `vX.Y.Z/oss-publish-receipt.json` 使用临时文件、fsync 和原子替换，记录版本、
目标、摘要及阶段。`publishing_latest` 写在 latest 操作前，表示可能已发布；失败后先读
远端，再使用未修改的同一产物恢复。不得并发操作同一本地目录或修改发布中的输入文件。
回执不是远端事实来源，每次执行都重新读取 latest 并校验对象，不依赖回执跳过校验。

独立技能位于配套 data-gateway-oss-upload 仓库的
`skills/vibewindow-oss-update/SKILL.md`，调用本仓库两个脚本，不复制业务实现。

```sh
python3 -m unittest discover -s scripts/tests -p test_publish_oss_release.py
python3 scripts/publish_oss_release.py --help
```

内存 OSS 状态机与本地 HTTP 测试涵盖首次发布、同版本恢复、内容冲突、降级、
不支持 CAS 的安全失败、部分上传、并发创建、未知写入结果、匿名失败与回执恢复、
凭据隔离和本地输入重验。本任务不运行 Cargo，不执行生产发布或本机升级。
