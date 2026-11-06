//! Native Codex row routing parser adapted from upstream live/project/codex.rs.
use super::{Error, fields as floor};
use serde_json::Value;
use toml_edit::{DocumentMut, Item, Table, TableLike, Value as TomlValue};
const MODEL_CATALOG_JSON: &str = "model_catalog_json";
fn foreign_catalog(doc: &DocumentMut) -> Option<&TomlValue> {
    doc.get(MODEL_CATALOG_JSON).and_then(Item::as_value).filter(|v| {
        v.as_str().is_some_and(|p| {
            std::path::Path::new(p).file_name().and_then(|n| n.to_str())
                != Some("cc-switch-model-catalog.json")
        })
    })
}
/// Codex 内置的 provider id（大小写敏感，和上游一致：`OpenAI` 是合法的自定义 id）。
const BUILT_IN_IDS: &[&str] =
    &["amazon-bedrock", "amazon-bedrock-runtime", "openai", "ollama", "lmstudio"];
/// 这几个 id 的表会让 Codex 0.148 起整份拒绝加载（bedrock 两个允许覆盖）。
const RESERVED_TABLE_IDS: &[&str] = &["openai", "ollama", "lmstudio"];
const BEDROCK_IDS: &[&str] = &["amazon-bedrock", "amazon-bedrock-runtime"];
/// 顶层的关键字段里，直接取行里值的那几个（选路、凭据、模型目录指针另算）。
pub(crate) const ROW_TOP_FIELDS: &[&str] = &[
    "model",
    "review_model",
    "model_reasoning_effort",
    "plan_mode_reasoning_effort",
    "disable_response_storage",
];

fn is_built_in_id(id: &str) -> bool {
    BUILT_IN_IDS.contains(&id)
}

/// 路由表的凭据从哪来。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteAuth {
    /// 表里声明了 `env_key`。
    EnvKey,
    /// CC Switch 把 Key 写成表的 `experimental_bearer_token`。
    Bearer,
    /// 表自己带鉴权（`auth` 命令、`aws`、`Authorization` 头、查询参数），不注入 Key。
    Headers,
    /// 没有凭据（本地服务等）。
    None,
}

/// 第三方路由表的 `requires_openai_auth`。
///
/// Codex 0.149 上这个值不决定请求用什么凭据（`env_key`、`experimental_bearer_token`
/// 优先），但决定登录界面：为 `true` 而 `auth.json` 里没有登录，Codex 会卡在登录页；为
/// `false` 而旁边留着 ChatGPT 登录，Codex 当成已登出（账号信息不显示、token 不刷新）。
/// 所以只有凭据走自己的通道（Bearer、EnvKey）时才跟着「写完后盘上有没有登录」走；
/// Headers、None 恒为 `false`，否则请求会回退去读 `auth.json` 里的官方登录，把它发给
/// 第三方地址。官方镜像形态（统一会话历史、代理的官方路由）恒为 `true`，不经过这里。
pub fn requires_openai_auth(auth: RouteAuth, login_on_disk: bool) -> bool {
    matches!(auth, RouteAuth::EnvKey | RouteAuth::Bearer) && login_on_disk
}

/// 行要求 Codex 走哪条路由。
#[derive(Debug, Clone)]
pub enum Route {
    /// 官方：不写选路，走 Codex 内置的 openai（统一会话历史时另写官方镜像表）。
    Official,
    /// 第三方：写成 `[model_providers.custom]`。`table` 已带上 Key，`requires_openai_auth`
    /// 在写入时按 [`requires_openai_auth`] 现算。
    Custom { table: Table, auth: RouteAuth },
    /// 选中 Codex 内置的其他 provider（ollama、lmstudio、bedrock），bedrock 可以带覆盖表。
    BuiltIn { id: String, table: Option<Table> },
    /// 第三方行里没有任何路由（只有 model、MCP 之类）：不写选路。
    Default,
}

/// 一个供应商在 `config.toml` 里拥有的键。
#[derive(Debug, Clone)]
pub struct CodexProjection {
    pub route: Route,
    /// 顶层关键字段（模型名、推理档位、`disable_response_storage`）。
    pub top: Vec<(String, TomlValue)>,
    /// 嵌在用户表里的模型名（`agents.default_subagent_model` 等）。
    pub nested: Vec<(Vec<String>, TomlValue)>,
    /// 独有字段（窗口、推理摘要能力、verbosity、行里自己写的 `web_search`）。上游拒收
    /// web_search 时写入方再把它改成 `"disabled"`。
    pub exclusive: Vec<(String, TomlValue)>,
}

/// 投影的输入。
pub struct RowInput<'a> {
    /// 供应商行的 `settings_config`（`{auth, config}`）。
    pub settings: &'a Value,
    /// 官方卡（`category == "official"`，或按 `is_codex_official_provider` 认出来的）。
    pub official: bool,
    /// 代理注入凭据的 OAuth 卡（xai_oauth、github_copilot…）：本来就没有 Key，行里的
    /// `requires_openai_auth = true` 是旧模板留下的。
    pub proxy_injected_oauth: bool,
}

impl CodexProjection {
    pub fn of(input: &RowInput<'_>) -> Result<Self, Error> {
        let settings = input.settings.as_object().ok_or(Error::Shape)?;
        if !settings.contains_key("auth") {
            return Err(Error::Shape);
        }
        let config_text = settings.get("config").and_then(Value::as_str).unwrap_or("");
        let doc = config_text.parse::<DocumentMut>().map_err(|_| Error::Shape)?;

        let mut top: Vec<(String, TomlValue)> = ROW_TOP_FIELDS
            .iter()
            .filter_map(|key| {
                let value = doc.get(key)?.as_value()?.clone();
                Some(((*key).to_string(), undecorated(value)))
            })
            .collect();
        // 行里自己指定的模型目录（用户管理的文件）照写；指向 CC Switch 自己目录的不算，
        // 那个指针由写入方按有没有生成目录决定。
        if let Some(pointer) = foreign_catalog(&doc) {
            top.push((MODEL_CATALOG_JSON.to_string(), undecorated(pointer.clone())));
        }
        let nested = floor::CODEX_FLOOR_NESTED
            .iter()
            .filter_map(|segments| {
                let value = value_at(doc.as_table(), segments)?.clone();
                Some((segments.iter().map(|s| (*s).to_string()).collect(), undecorated(value)))
            })
            .collect();
        let exclusive = floor::CODEX_EXCLUSIVE_TOP
            .iter()
            .filter_map(|key| {
                let value = doc.get(key)?.as_value()?.clone();
                Some(((*key).to_string(), undecorated(value)))
            })
            .collect();

        let route = if input.official { Route::Official } else { third_party_route(&doc, input)? };
        Ok(Self { route, top, nested, exclusive })
    }
}

fn undecorated(mut value: TomlValue) -> TomlValue {
    value.decor_mut().clear();
    value
}

fn value_at<'a>(root: &'a Table, segments: &[&str]) -> Option<&'a TomlValue> {
    let (last, parents) = segments.split_last()?;
    let mut current: &dyn TableLike = root;
    for segment in parents {
        current = current.get(segment)?.as_table_like()?;
    }
    current.get(last)?.as_value()
}

fn non_empty_str(item: Option<&Item>) -> Option<String> {
    item.and_then(Item::as_str).map(str::trim).filter(|text| !text.is_empty()).map(str::to_string)
}

/// 行里的表（标准表或内联表）转成标准表，去掉原来的空白和注释。
fn to_table(item: &Item) -> Option<Table> {
    let mut table = match item {
        Item::Table(table) => table.clone(),
        Item::Value(TomlValue::InlineTable(inline)) => inline.clone().into_table(),
        _ => return None,
    };
    table.decor_mut().clear();
    table.set_implicit(false);
    table.set_dotted(false);
    for (_, value) in table.iter_mut() {
        if let Some(value) = value.as_value_mut() {
            value.decor_mut().clear();
        }
    }
    table.sort_values_by(|_, _, _, _| std::cmp::Ordering::Equal);
    Some(table)
}

fn table_bool(table: &dyn TableLike, key: &str) -> bool {
    table.get(key).and_then(Item::as_bool).unwrap_or(false)
}

fn declares_authorization_header(item: Option<&Item>) -> bool {
    item.and_then(Item::as_table_like)
        .is_some_and(|table| table.iter().any(|(key, _)| key.eq_ignore_ascii_case("authorization")))
}

/// 表自己带的凭据来源。`requires_openai_auth = true` 时 `Authorization` 头不算：官方
/// 登录会在头之后覆盖它（Codex 0.149），只有注入的 Key 能挡住回退。
fn declared_auth(table: &dyn TableLike) -> Option<RouteAuth> {
    if table.get("env_key").is_some() {
        return Some(RouteAuth::EnvKey);
    }
    if table.get("auth").is_some() || table.get("aws").is_some() {
        return Some(RouteAuth::Headers);
    }
    let requires = table_bool(table, "requires_openai_auth");
    if !requires
        && (declares_authorization_header(table.get("http_headers"))
            || declares_authorization_header(table.get("env_http_headers")))
    {
        return Some(RouteAuth::Headers);
    }
    None
}

fn third_party_route(doc: &DocumentMut, input: &RowInput<'_>) -> Result<Route, Error> {
    // 配置整个是空的却带着 Key：没有地方放 Key，也不知道该发到哪。
    if doc.as_table().is_empty() && row_key(doc, None, input).is_some() {
        return Err(Error::Shape);
    }
    let providers = doc.get("model_providers").and_then(Item::as_table_like);
    let selector = non_empty_str(doc.get("model_provider"));
    let row_table = |id: &str| providers.and_then(|p| p.get(id)).and_then(to_table);

    let (table, fallback_name) = match selector.as_deref() {
        Some(id) if !is_built_in_id(id) => match row_table(id) {
            Some(table) => (table, id.to_string()),
            None => return Err(Error::Shape),
        },
        // 旧版留下的保留 id 表：Codex 整份拒绝加载，按内容改写成 custom。
        Some(id) if RESERVED_TABLE_IDS.contains(&id) && row_table(id).is_some() => {
            let mut table = row_table(id).expect("checked above");
            table.insert("wire_api", toml_edit::value("responses"));
            (table, "Custom".to_string())
        }
        // 旧形态：内置 openai 加顶层 openai_base_url 改道。
        None | Some("openai") => match non_empty_str(doc.get("openai_base_url")) {
            Some(base_url) => {
                let mut table = Table::new();
                table.insert("name", toml_edit::value("Custom"));
                table.insert("base_url", toml_edit::value(base_url));
                table.insert("wire_api", toml_edit::value("responses"));
                (table, "Custom".to_string())
            }
            // 旧形态：没有选路，地址直接写在顶层 `base_url`（Codex 不认，代理认）。不收进
            // 路由表的话，存行时它作为关键字段被剥掉、再也写不回来（#8039）。
            None if selector.is_none() => match non_empty_str(doc.get("base_url")) {
                Some(base_url) => {
                    let mut table = Table::new();
                    table.insert("name", toml_edit::value("Custom"));
                    table.insert("base_url", toml_edit::value(base_url));
                    table.insert("wire_api", toml_edit::value("responses"));
                    (table, "Custom".to_string())
                }
                None => return default_route(doc, input),
            },
            None => return built_in_route("openai", providers, doc, input),
        },
        Some(id) => return built_in_route(id, providers, doc, input),
    };
    custom_route(table, &fallback_name, doc, input)
}

/// 行的 Key：`auth.OPENAI_API_KEY`，或者直接写在配置里的 `experimental_bearer_token`
/// （路由表里的优先，其次顶层）。
fn row_key(doc: &DocumentMut, table: Option<&Table>, input: &RowInput<'_>) -> Option<String> {
    input
        .settings
        .get("auth")
        .and_then(|auth| auth.get("OPENAI_API_KEY"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(str::to_string)
        .or_else(|| table.and_then(|table| non_empty_str(table.get("experimental_bearer_token"))))
        .or_else(|| non_empty_str(doc.get("experimental_bearer_token")))
}

fn keyless_fallback_error() -> Error {
    Error::Keyless
}

fn custom_route(
    mut table: Table,
    fallback_name: &str,
    doc: &DocumentMut,
    input: &RowInput<'_>,
) -> Result<Route, Error> {
    if non_empty_str(table.get("name")).is_none() {
        table.insert("name", toml_edit::value(fallback_name));
    }
    let key = row_key(doc, Some(&table), input);
    let auth = match declared_auth(&table) {
        Some(declared) => declared,
        None => match key {
            Some(key) => {
                table.insert("experimental_bearer_token", toml_edit::value(key));
                RouteAuth::Bearer
            }
            None => {
                let requires = table_bool(&table, "requires_openai_auth");
                let rerouted =
                    doc.get("openai_base_url").is_some() && doc.get("model_providers").is_none();
                if (requires || rerouted) && !input.proxy_injected_oauth {
                    return Err(keyless_fallback_error());
                }
                if table.get("query_params").is_some()
                    || declares_authorization_header(table.get("http_headers"))
                    || declares_authorization_header(table.get("env_http_headers"))
                {
                    RouteAuth::Headers
                } else {
                    RouteAuth::None
                }
            }
        },
    };
    if matches!(auth, RouteAuth::Headers | RouteAuth::None) {
        table.remove("requires_openai_auth");
    }
    Ok(Route::Custom { table, auth })
}

fn built_in_route(
    id: &str,
    providers: Option<&dyn TableLike>,
    doc: &DocumentMut,
    input: &RowInput<'_>,
) -> Result<Route, Error> {
    // 内置 openai 没有改道，就是官方地址：带 Key 没地方放，不带 Key 会用官方登录。
    if id == "openai" {
        return match row_key(doc, None, input) {
            Some(_) => Err(no_token_slot_error()),
            None => Err(keyless_fallback_error()),
        };
    }
    let table = BEDROCK_IDS
        .contains(&id)
        .then(|| providers.and_then(|p| p.get(id)).and_then(to_table))
        .flatten();
    Ok(Route::BuiltIn { id: id.to_string(), table })
}

fn no_token_slot_error() -> Error {
    Error::Shape
}

/// 没有任何路由的第三方行（只改模型、MCP 的卡）：有意放行，Codex 继续用内置 openai
/// 和它自己的登录。行里即使带着 Key 也用不上（没有第三方地址可发），不写进 live。
fn default_route(_doc: &DocumentMut, _input: &RowInput<'_>) -> Result<Route, Error> {
    Ok(Route::Default)
}
