# 任务35：入口、许可与隔离账号准备

2026-11-05。用户最终决定跳过所有剩余关口并关闭任务：17个CLI的34条真实来源路径通过；Auggie、Cursor、Droid、Qoder、TRAE共5项、10条来源路径全部用户豁免，仍未真实验证，不计作通过，也不再阻塞当前任务。TRAE已依据88602097a关闭。regression-35.json保留原始44条观测，并单独记录最终豁免处置。以下许可、接口与账号步骤仅供以后主动恢复使用，当前不要求账号、不执行登录、不重跑已通过路径。

## Auggie：已补查新发行版与公开API

官方npm当前stable为0.37.0，preview为1.2.0-prerelease.202611041545。分别下载原始归档，验证npm SHA512 integrity，只提取package.json、实际入口及LICENSE.md到私有target目录，不运行安装器或修改全局版本。[发行物指纹](auggie/recovery-new-release-identity.json)。

使用原有真实关口probe，给PACKAGES替换为上述发行版和所记录CLI SHA256；命令入口为本地 `python3 target/task35-new-auggie.py`，退出0。新稳定版仍拒绝`--extension`，新预览版ACP/print扩展工厂已加载但`registerProvider`拒绝；Pi models.json替代仍失败。四项均退出1、模型请求0，[结果](auggie/recovery-new-release-gates.json)。

另以公开Pi `setModel(Model)` API尝试直接选中loopback OpenAI模型：扩展工厂及session_start回调均实际执行，但setModel返回false；before_agent_start未被触发。三个print进程均退出1、HTTP请求0，[结果](auggie/recovery-set-model.json)。扩展只调用公开API，没有修改bundle或模拟平台认证。测试入口为保留在target的task35-auggie-setmodel.py。

许可按组件区分：0.37.0的Augment自有CLI使用专有许可；新预览版明确保留Pi与Pi Subagents的MIT许可，包括修改、组合、分发相应第三方代码的权利。**这些MIT修改不需要再取得一份许可**，但须保留版权及许可声明。不能把整个混合bundle一概说成禁止修改，也不能把MIT扩展到未归属第三方的Augment集成代码。当前没有可审查的组件对应关系证明，修改MIT组件即可在原Auggie中避开已拒绝的Provider集成；不以独立运行Pi代替Auggie。

官方[CLI参考](https://docs.augmentcode.com/cli/reference)和[文档索引](https://docs.augmentcode.com/llms.txt)仍未提供此固定发行版可用的任意模型Provider契约。Pi上游[扩展实现](https://raw.githubusercontent.com/earendil-works/pi/main/packages/coding-agent/src/core/extensions/runner.ts)具有Provider注册路径，但这不能证明Auggie启用了同一路径。

最小解除条件：上游提供原Auggie ACP可用的endpoint/key/model注入与辅助/恢复请求约束；或提供必要集成代码的来源及适用许可，证明所需改动已被现有许可覆盖；若涉及Augment自有受限代码，则需适用的修改许可。仅提供Augment订阅账号不补足任意Provider接口，不要求用户为本阻塞购买订阅。

## Cursor：账号与模型入口是两个条件

固定版2026.10.22-27dcf5f已证明`--endpoint`指向账号交换/平台RPC，未证明指向标准模型API；现有负向报告见[上游关口](upstream-gates-35.json)。官方[ACP说明](https://cursor.com/docs/cli/acp)接受Cursor登录、用户API key或认证令牌；[认证说明](https://cursor.com/docs/cli/reference/authentication)中的key来自Cursor Dashboard，不能用OpenAI/Anthropic模型key代替。

原版`login --help`已核对。用户可在自己的zsh终端执行下面步骤；仅创建独立配置目录并使用用户主动提供的Cursor用户API key，不读取已有登录态、不把key放进argv、文件或聊天：

```zsh
cd /Users/xiangmin/worker/code/shiju
set +x
umask 077
mkdir -p target/acp-auth-cursor
export CURSOR_CONFIG_DIR="$PWD/target/acp-auth-cursor"
export NO_OPEN_BROWSER=1
read -rs 'CURSOR_API_KEY?Cursor Dashboard user API key: '; print
export CURSOR_API_KEY
./target/acp-model-source-tools/cursor-2026.10.22-27dcf5f/dist-package/cursor-agent status
# 后续获授权的验证需从同一终端继承该环境；结束后：
unset CURSOR_API_KEY
```

浏览器替代是同配置根运行 `./target/acp-model-source-tools/cursor-2026.10.22-27dcf5f/dist-package/cursor-agent login`，由用户完成官方登录。该命令只核对过help，实际登录/凭据落点未验证；需保留独立用户/容器环境来确认完整隔离，不能宣称CURSOR_CONFIG_DIR已经隔离所有钥匙串状态。优先采用上述短期环境key路径。

账号要求：用户本人获授权的Cursor账号及CLI访问权；团队策略须允许所需模型。没有证据说明为了注册任意loopback Provider应购买哪个套餐，因此不推荐购买。账号仅解除平台认证关口，仍需厂商提供CLI/ACP可用的任意Provider注册/配置接口（base URL、协议、认证、模型、主辅助及恢复路由、无回退）。IDE BYOK不能自动推导到CLI。[Bedrock](https://cursor.com/docs/customizing/aws-bedrock)另需AWS身份、模型调用权限及团队配置，不等于任意标准模型中继，也不适用于本计划仅loopback的离线验收。

随包LICENSE.txt中的MIT等通知适用于列出的第三方依赖，不是整个CLI的开源许可。官方[服务条款](https://cursor.com/terms-of-service)对自有服务另有限制；本任务不把所有依赖修改均称为需要新授权，当前缺口首先是原CLI可用的公开模型入口及平台身份。

## Droid：已有BYOK入口，缺真实Factory身份

真实1.3.0的ACP initialize成功，指定custom model的session/new明确要求Factory认证。官方[BYOK](https://docs.factory.com/model-independence/byok)支持自定义baseUrl；不需要厂商再开发Provider接口。原版`--help`没有login子命令，`droid login --help`实际显示的是根帮助，不把它作为已验证登录命令。

所需身份为Factory个人用户，或组织明确授权的自动化service account及其Factory API key。组织service account由Owner/Manager创建；组织若限制API key创建，普通成员需让有权管理员创建。政策须允许custom model及本地fixture base URL；不修改或绕过组织限制。依据[身份与访问](https://docs.factory.com/enterprise/identity-and-access)、[CLI认证](https://docs.factory.com/droid-cli/cli-reference)和[企业模型策略](https://docs.factory.com/enterprise/hierarchical-settings-and-org-control)。不假定一定需要企业账号，也不把厂商权限/计费状态未验证写成已满足。

推荐用户在自己的zsh终端通过环境注入专用Factory API key：

```zsh
cd /Users/xiangmin/worker/code/shiju
set +x
umask 077
mkdir -p target/acp-auth-droid
export FACTORY_HOME_OVERRIDE="$PWD/target/acp-auth-droid"
export FACTORY_DISABLE_KEYRING=1
export FACTORY_DROID_AUTO_UPDATE_ENABLED=false
read -rs 'FACTORY_API_KEY?Factory API key: '; print
export FACTORY_API_KEY
./target/acp-model-source-tools/droid/droid
# 不提交业务模型提示；后续明确授权的fixture验收从同一终端继承环境。
# 退出CLI并结束验证后：
unset FACTORY_API_KEY
```

无key时也可由用户在同隔离目录启动原droid，按其官方浏览器/device-pairing提示登录。当前尚未执行真实登录，不能保证disable-keyring模式下该版本保存/刷新登录成功。API key方式避免将现有用户钥匙串当作前提。环境注入只对当前终端及其子进程有效，不会自动传给已运行的Codex进程；需要明确授权后从持有环境的终端启动后续验收，或使用用户指定的安全秘密注入设施。

Factory自有软件的[条款](https://www.factory.ai/terms-of-service)与第三方开源组件应分别处理；此项当前无需改厂商代码，关键是合法身份及政策允许BYOK。取得身份后仍需实现driver、证明全部配置/历史隔离并运行两条真实来源验收；身份准备不等于功能完成。

本轮只新增文档及脱敏调查JSON，无Rust变更或Cargo调用，不领取槽位。未执行登录、发消息、创建账号或购买服务；未重跑已通过34条路径。
