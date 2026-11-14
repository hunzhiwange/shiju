# 网关客户端全面拆分计划

目标：为全部客户端职责建立单向领域边界；本计划生成不代表代码拆分完成。

范围：vw-gateway-client 的全部源码、测试、清单、平台条件和四个直接消费者。来源：系统底层全面拆分任务26。

范围外：功能修复、依赖升级、Cargo.lock 更新、任何动态验证。

假设：维持 0.6.0 / edition 2024 / MIT；前序 api-types/shared/provider-resolver 子计划只是计划依赖，实施时以真实落地导出为准。

## 设计决策

整体搬包会保留耦合；逐函数建库增加无效边界。采用领域库，应用供应商适配器逐卡迁入同一所有者。库内大文件按协议、传输、DTO拆模块，不造 common/core 大库。

## 架构说明

依赖方向为消费者 → 领域客户端 → GatewayTransport → GatewayEndpoint → 现有 API 类型。详情见 audit.md；逐文件、符号及调用证据见 source-map.md，外部引用见 consumer-refs.md。新类型 GatewayTransport 和各领域客户端均为拟新增接口，不能当作当前可用符号。

## 开发策略

只做保持行为的结构迁移，保留既有测试与断言；不做测试驱动运行。共享 reqwest 连接池通过 Clone 保留，领域客户端显式持有 GatewayTransport；不能为每个方法重新构造客户端。GatewayClient 保留显式转发门面，不用 Deref、注册器或扩展 trait 隐藏行为。

## 公共执行约定

- 先读仓库 AGENTS.md、.plans/AGENTS.md、拆包计划去重审查.md、本文件及任务卡。使用 /Users/xiangmin/.codex/skills/do-task-plans/SKILL.md。plan.md 生成后只读，prompt.md 为固定历史入口。
- 常规可逆决策自主处理；源码与真实消费者优先，必要相邻接线可修复，不扩大功能范围，不覆盖他人变更。关键歧义合理调查后仍无解才记录阻塞及所需输入。
- 所有批次严格串行，每批一张卡，共写根 Cargo.toml、原包清单、门面与消费者，禁止并行争用。每卡开始核实前一张已提交及具体符号已落地；不能只凭卡消失判断。
- 新库统一 crates/libs/<目标>/Cargo.toml 与 src/lib.rs；新增 members、直接消费者路径依赖与真实导入同步接线，保持版本与 features。不得反向依赖 vw-gateway-client。跨域调用改为显式组合对应领域客户端，共享错误保留唯一所有者。
- 源码归属与后续符号映射记录在编号卡；公共文件不记进度。仅公开真实跨库使用的符号；测试保持独立文件，测试固件可局部复制小段，不为此造生产测试框架库。

## 公共验证与资源规则

用户明确覆盖：只写代码，禁止任何 Cargo 命令及间接调用，不运行单元/脚本测试、GUI、实机或截图，不领取构建槽位，不更新 Cargo.lock。只允许静态阅读、rg、git diff --check 与差异审查。各卡后续行为验收均未验证，未来另行授权统一进行；不得将静态审查写成编译、性能或测试通过。

原生/wasm reqwest 的条件依赖保持原清单差异；原生 blocking/multipart/json/query/stream/rustls，wasm json/query/stream；tokio rt/time 和 tokio-tungstenite rustls-tls-webpki-roots 只给实际原生使用者。serde derive、futures-util、tracing、vw-shared 仅按源码实际使用分配，无根 features/build脚本或包内资源需要新增。

## 公共收尾规则

仅使用未开始、已完成、已阻塞、待回归。已完成表示实现与静态审查完成，不代表动态验证。按用户要求不因延后验证保留卡；通过 /Users/xiangmin/.codex/skills/git-commit/SKILL.md 仅提交本任务内容，中文说明明确“未运行 Cargo、测试及实机验证”。提交成功后删除对应编号卡；plan.md/prompt.md 保留。提交失败或真实实现阻塞保留卡并记录证据，已有变更仍按契约提交；无实际变更不建空提交。
