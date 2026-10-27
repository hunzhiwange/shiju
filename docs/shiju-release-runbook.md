# ShiJu 双源升级与 OSS 发布流程

适用日期：2026-10-27。本文对应“双源升级与 OSS 发布”已完成的实现，供发布人员执行。
详细协议见 [oss-updates.md](oss-updates.md)。本文档的生成不代表已执行生产发布。

## 1. 先判断本次能否发布

完整路径：**GitHub 正式 Release → 本地下载六个平台原包 → 离线预览 → OSS 发布 → 公网验收 → 客户端升级验收**。

当前发布器存在明确限制：

| OSS latest 状态 | 当前可执行行为 |
| --- | --- |
| 不存在，且 OSS API 明确返回 NoSuchKey | 首次发布 |
| 已是本次版本，清单字节完全一致 | 同版本恢复、重新校验 |
| 已存在较低版本 | **停止：当前实现不支持推进 latest 到更高版本** |
| 已存在较高版本 | 停止：拒绝降级 |
| 同版本但清单字节不同 | 停止：拒绝覆盖 |
| 状态不明、权限错误或读取失败 | 停止并排查 |

已有 latest 的连续升版，需要先补齐受支持的目标条件更新协议（CAS，即仅在目标仍符合预期时更新）。
不要删除 latest、手工覆盖指针或改写版本对象来绕过。匿名 URL 的 404 不能单独证明允许首次发布，最终由发布器读取 OSS 判定。

## 2. 发布前准备

- GitHub 仓库固定为 `hunzhiwange/shiju`；目标 `vX.Y.Z` 已是正式 Release，非草稿、非预发布。
- 六个平台资产上传完毕，GitHub API 为每个资产返回 SHA-256 digest。缺包或缺 digest 时先修复 Release。
- 本机具备 Python 3.10+、`uv`，可访问 GitHub、OSS 和 `https://www.shiju.cloud`。
- 产物使用无符号链接组件的绝对路径，所在文件系统支持硬链接；macOS 临时路径使用 `/private/tmp` 而非 `/tmp`。
- 同一产物目录只由一个进程操作；发布期间不改清单和安装包。
- OSS bucket 固定为 `shiju-cloud`，发布前缀固定为 `agents/shiju-agent/`。
- bucket 必须从未启用版本控制；已开启、已暂停或无法读取状态时发布器都会拒绝。
- 发布账号能够读取 bucket 版本控制状态，并读取、创建上述前缀对象；公开域名能匿名读取该前缀。
- 发布期间保持 bucket 版本控制状态不变，避免其他客户端覆盖该前缀对象。

发布器读取本机 `~/.config/shiju-oss-upload/config.json` 的 `region`、`access_key_id`、`access_key_secret`。
`region` 使用实际地域，例如 `cn-hangzhou`，不是完整 endpoint。密钥可由
`OSS_ACCESS_KEY_ID`、`OSS_ACCESS_KEY_SECRET` 环境变量覆盖，配置文件和 region 仍需存在。
通过本机安全配置渠道准备凭据，不把真实密钥写入本文、仓库或发布记录。

## 3. 设置本次版本和目录

下列命令在同一个终端执行。`0.5.10` 仅为示例，执行前替换成实际待发布版本。
本流程从已经准备好的 GitHub Release 开始，不会创建 tag 或触发打包。

```bash
RELEASE_VERSION='0.5.10'
SOURCE_REPO='/Users/Shared/work/dir/data/codes/vibe-window'
PUBLISH_SKILL='/Users/Shared/work/dir/data/codes/data-gateway-oss-upload/skills/shiju-agent-oss-update'
OUTPUT_ROOT='/private/tmp/shiju-oss-release'
RELEASE_DIR="$OUTPUT_ROOT/v$RELEASE_VERSION"
```

在其他机器上替换两个仓库路径和输出根目录。发布技能自包含，上传阶段只需要技能目录与完整产物目录，不需要本仓库源码。

## 4. 预览 GitHub 元数据并下载

先预览（访问 GitHub，但不下载六个安装包，也不访问 OSS）：

```bash
python3 "$SOURCE_REPO/scripts/prepare_oss_release.py" \
  --version "$RELEASE_VERSION" --output-dir "$OUTPUT_ROOT" --metadata-only
python3 -m json.tool "$RELEASE_DIR/manifest.preview.json"
```

确认版本、Release 地址、六个平台、文件名、大小和摘要符合本次发布目标，然后正式准备：

```bash
python3 "$SOURCE_REPO/scripts/prepare_oss_release.py" \
  --version "$RELEASE_VERSION" --output-dir "$OUTPUT_ROOT"
python3 -m json.tool "$RELEASE_DIR/manifest.json"
```

每条命令成功后才进入下一步。正式 `manifest.json` 仅在六包全部通过大小和 SHA-256 校验后生成。
`manifest.preview.json` 不能替代它。下载可使用 `GITHUB_TOKEN`，该令牌不用于 OSS。

应有以下六包及正式清单；不解压、不改名、不重新压缩：

| 平台 | 文件名（将 X.Y.Z 换为本次版本） |
| --- | --- |
| Linux ARM64 | `shiju-X.Y.Z-1.aarch64.rpm` |
| Linux x64 | `shiju-X.Y.Z-1.x86_64.rpm` |
| Windows ARM64 | `ShiJu-aarch64-pc-windows-msvc-dist.msi` |
| Windows x64 | `ShiJu-x86_64-pc-windows-msvc-dist.msi` |
| macOS ARM64 | `ShiJu-macos-aarch64-apple-darwin-dist.dmg` |
| macOS x64 | `ShiJu-macos-x86_64-apple-darwin-dist.dmg` |

## 5. 执行发布器离线预览

```bash
cd "$PUBLISH_SKILL"
uv run scripts/publish.py --dir "$RELEASE_DIR"
```

预览会重新校验清单、六包、URL、大小和摘要，不读取 OSS 凭据、不上传对象。
首次 `uv run` 可能联网安装脚本依赖；“离线”指发布逻辑不访问 OSS。
检查输出的六条地址均位于 `https://www.shiju.cloud/agents/shiju-agent/releases/vX.Y.Z/`。
单包不得超过 5 GiB。预览成功不代表生产权限或已有 latest 状态已通过验证。

## 6. 执行生产发布

在确认本次版本和生产发布目标后执行；此命令会写入 OSS：

```bash
cd "$PUBLISH_SKILL"
uv run scripts/publish.py --dir "$RELEASE_DIR" --publish
```

发布器依次执行：

1. 重验本地输入，检查 bucket 版本控制并读取 latest。
2. 创建六包和版本清单，共七个不可变对象；逐个从 OSS 完整回读校验。
3. 从公开域名匿名完整读取七个对象，核对大小与 SHA-256。
4. 最后以禁止覆盖方式创建 `agents/shiju-agent/latest/manifest.json`。
5. 匿名回读 latest，写入成功回执。

成功应满足：命令退出码为 0，`$RELEASE_DIR/oss-publish-receipt.json` 中 `stage` 为 `complete`。

```bash
python3 -m json.tool "$RELEASE_DIR/oss-publish-receipt.json"
```

保存本次版本、GitHub Release 地址、正式清单、六包和回执。回执用于记录，不能替代远端实际状态。

## 7. 公网及客户端验收

发布器已经完整校验公网对象。发布后再检查 latest 内容与响应头，便于人工留档：

```bash
curl --fail --silent --show-error \
  --dump-header "$RELEASE_DIR/latest.headers.txt" \
  --output "$RELEASE_DIR/latest.remote.json" \
  'https://www.shiju.cloud/agents/shiju-agent/latest/manifest.json'
cmp "$RELEASE_DIR/manifest.json" "$RELEASE_DIR/latest.remote.json"
```

两条命令应成功；latest 响应的缓存策略应为 `no-store, no-cache, max-age=0, must-revalidate`，
版本对象应为一年 immutable 缓存。若 CDN 返回旧内容或修改缓存策略，先排查域名/CDN 配置再宣布发布完成。

在包含本次双源功能、且版本低于待发布版本的客户端中验证：

- 选择并保存 `shiju` 来源，检查更新，确认目标版本并下载安装包。
- 下载校验完成后打开系统安装器，按系统流程安装，重新启动并核对实际版本。
- 切换为 `GitHub` 来源并保存，验证该来源可检查和下载；重启后确认来源偏好保留。
- 来源失败时应展示错误，不自动切换另一来源。
- 按实际覆盖平台分别记录 DMG、MSI、RPM 安装结果；未覆盖的平台标注未验收。

首次上线应先确认 OSS 公开可用，再面向用户推广默认使用 shiju 源的客户端。
更早且不包含双源功能的客户端能否迁移，需要另行实测；不能仅凭发布成功认定可自动升级。

## 8. 失败处理与恢复

| 现象 | 处理 |
| --- | --- |
| GitHub 资产缺失、名称不符、缺 digest | 修复目标 Release 后重新准备，不手工伪造清单 |
| 下载中断或残留 `.part` | 重跑准备命令；正确完整包会复用，临时包重新下载 |
| 本地同名文件内容冲突 | 查明原因，将冲突文件移出产物目录后重新准备 |
| OSS 403、版本控制状态不符 | 修复访问配置；已启用/暂停版本控制的 bucket 不满足当前协议，不强行发布 |
| 匿名回读失败 | 检查公开域名、目录权限、CDN；修复后使用原产物恢复 |
| 七对象部分上传成功 | 保持原产物不变，排除故障后重跑同一条 `--publish` 命令 |
| 回执停在 `publishing_latest` 或写入结果不明 | latest 可能已创建；先核对远端状态，再以原产物重跑，发布器会重新验证 |
| 已有较低版本 latest，提示 CAS 限制 | 停止；先实现连续升版协议，当前命令无法完成本次升版 |
| 同版本内容不同或请求降级 | 停止并核对版本与产物，不覆盖已发布对象 |

恢复时不重排、格式化或重新生成已进入发布阶段的清单；同版本要求清单字节一致。
不通过删除 OSS 对象来“重试”。当前发布器也没有自动回滚或降级入口；有问题的版本应暂停推广，
由维护者制定处置方案，不能把旧清单覆盖回 latest。

## 9. 发布记录模板

```text
发布时间 / 操作人：
版本 / GitHub Release：
本地产物目录：
发布类型：首次 / 同版本恢复
离线预览：通过 / 失败
发布退出码 / 回执 stage：
公网 latest 字节比较 / 缓存：
shiju 来源检查与下载：
GitHub 来源检查与下载：
来源偏好重启保留：
实际安装与重启版本（逐平台）：
未验证项 / 异常 / 后续动作：
```
