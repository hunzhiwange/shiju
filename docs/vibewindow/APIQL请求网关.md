# APIQL 请求网关

APIQL 是 VibeWindow 的数据查询网关层。它通过受管客户端路由读取当前租户的数据连接和实体配置。

## 概述

APIQL 网关负责：

- 接收 HTTP APIQL 请求；
- 验证认证与租户 header；
- 从请求 extension 取得租户作用域；
- 只在该租户的 Store 中加载连接和实体配置。

## 租户隔离

APIQL 请求必须发送：

```http
X-Platform-ID: 10000
X-Company-ID: 100000000
```

`10000` / `100000000` 是缺少显式 Desktop 配置时的默认平台和公司。Desktop 每个网关分别持久化 platform/company；切换网关或修改租户后，新请求使用当前网关的值。

header 是 APIQL 的唯一租户来源。URL、query、请求体和连接名称都不能覆盖租户。非 `OPTIONS` 请求出现以下任一情况时返回 `400 Bad Request`，并且不回退默认租户：

- 缺少 `X-Platform-ID` 或 `X-Company-ID`；
- 同名 header 重复；
- header 不是有效 UTF-8；
- header 为空或仅含空白。

`OPTIONS` 预检豁免租户 header。第三方 webhook 和 `/v1/health` 不属于 APIQL 受管客户端路由，也豁免。

## 核心功能

### 请求处理

```http
GET /v1/apiQL/{entity_class}
GET /v1/apiQL/{entity_class}/list-only
GET /v1/apiQL/{entity_class}/list-count
GET /v1/apiQL/{entity_class}/list-page
```

请求先经过认证和租户 middleware。middleware 构造 `TenantScope` 并注入 Axum request extension；APIQL handler 消费 extension，再用该 scope 打开租户化 Store。相同连接或实体 ID 在不同租户中互不覆盖。

## 安全特性

1. 认证与租户校验在进入 handler 前完成。
2. 非法租户明确返回 `400`，不做默认值兜底。
3. 日志只记录 scope ID 和请求元数据，不记录密钥或原始令牌。
4. `/v1/health` 与第三方 webhook 的豁免不能扩展到 APIQL 路由。
