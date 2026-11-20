//! Wire types for the scoped Agent Cloud workbench response.

use reqwest::{Response, StatusCode, Url, header::HeaderValue};
use serde::Deserialize;
use std::time::Duration;

const API_BASE: &str = "https://gateway.shiju.cloud/sj/agent/api/v1/";
const CONSOLE: &str = "https://ai.shiju.cloud/console/ai-agents";

pub(super) async fn load(credential: &str) -> Result<Summary, String> {
    Client::new(credential)?.get("workbench/summary", &[]).await
}

pub(super) struct Client(reqwest::Client);

impl Client {
    pub(super) fn new(credential: &str) -> Result<Self, String> {
        let mut key = HeaderValue::from_str(credential).map_err(|_| "识矩登录凭据格式无效")?;
        key.set_sensitive(true);
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("X-Api-Key", key);
        headers.insert("Accept", HeaderValue::from_static("application/json"));
        reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .redirect(reqwest::redirect::Policy::none())
            .default_headers(headers)
            .build()
            .map(Self)
            .map_err(|_| "无法初始化数字员工连接".into())
    }

    pub(super) async fn get<T: serde::de::DeserializeOwned>(
        &self,
        resource: &str,
        query: &[(&str, &str)],
    ) -> Result<T, String> {
        Self::decode(self.get_response(path_url(resource.split('/')), query).await?).await
    }

    pub(super) async fn iam<T: serde::de::DeserializeOwned>(
        &self,
        resource: &str,
        query: &[(&str, &str)],
    ) -> Result<T, String> {
        let url = Url::parse(&format!("https://gateway.shiju.cloud/sj/iam/api/v1/{resource}"))
            .expect("fixed IAM API URL");
        let envelope: IamResponse<T> = Self::decode(self.get_response(url, query).await?).await?;
        if envelope.code == 200 {
            return Ok(envelope.data);
        }
        Err("识矩成员或权限数据暂不可用".into())
    }

    pub(super) async fn assistant_governance<T: serde::de::DeserializeOwned>(
        &self,
    ) -> Result<Option<T>, String> {
        let response =
            self.get_response(path_url(["assistants"]), &[("view", "governance")]).await?;
        if response.status() == StatusCode::FORBIDDEN {
            return Ok(None);
        }
        Self::decode(response).await.map(Some)
    }

    pub(super) async fn task<T: serde::de::DeserializeOwned>(
        &self,
        id: &str,
        collection: Option<&str>,
    ) -> Result<T, String> {
        let url = path_url(["tasks", id].into_iter().chain(collection));
        Self::decode(self.get_response(url, &[]).await?).await
    }

    pub(super) async fn get_path<T: serde::de::DeserializeOwned>(
        &self,
        segments: &[&str],
        query: &[(&str, &str)],
    ) -> Result<T, String> {
        Self::decode(self.get_response(path_url(segments.iter().copied()), query).await?).await
    }

    pub(super) async fn post<T: serde::de::DeserializeOwned>(
        &self,
        segments: &[&str],
        body: &serde_json::Value,
        key: Option<&str>,
    ) -> Result<T, String> {
        let mut request = self.0.post(path_url(segments.iter().copied())).json(body);
        if let Some(key) = key {
            request = request.header("Idempotency-Key", key);
        }
        let response = request.send().await.map_err(|_| "无法连接数字员工服务，请重试")?;
        Self::decode(response).await
    }

    pub(super) async fn task_stream(&self, id: &str) -> Result<Response, String> {
        let response = self
            .0
            .get(path_url(["tasks", id, "events", "stream"]))
            .header("Accept", "text/event-stream")
            .timeout(Duration::from_secs(24 * 60 * 60))
            .send()
            .await
            .map_err(|_| "任务事件流连接失败")?;
        if !response.status().is_success()
            || !response
                .headers()
                .get("Content-Type")
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| value.starts_with("text/event-stream"))
        {
            return Err("任务事件流不可用".into());
        }
        Ok(response)
    }

    async fn get_response(&self, url: Url, query: &[(&str, &str)]) -> Result<Response, String> {
        self.0.get(url).query(query).send().await.map_err(|_| "无法连接数字员工服务，请重试".into())
    }

    async fn decode<T: serde::de::DeserializeOwned>(response: Response) -> Result<T, String> {
        let status = response.status();
        match status {
            StatusCode::UNAUTHORIZED => return Err("识矩登录已失效，请重新登录".into()),
            StatusCode::FORBIDDEN => return Err("当前识矩账号没有查看此资源的权限".into()),
            _ if !status.is_success() => {
                return Err(format!("数字员工服务请求失败（HTTP {status}）"));
            }
            _ => {}
        }
        response.json().await.map_err(|_| "数字员工服务返回的数据无法读取".into())
    }
}

#[derive(Deserialize)]
struct IamResponse<T> {
    code: u16,
    data: T,
}

fn path_url<'a>(segments: impl IntoIterator<Item = &'a str>) -> Url {
    let mut url = Url::parse(API_BASE).expect("fixed agent cloud API URL");
    url.path_segments_mut().expect("hierarchical URL").pop_if_empty().extend(segments);
    url
}

pub(super) fn employee_config_url(id: &str) -> String {
    let mut url = Url::parse(CONSOLE).expect("fixed console URL");
    url.path_segments_mut().expect("hierarchical URL").extend(["employees", id, "config", "team"]);
    url.into()
}

/// Resolve only known product routes. Server data is never passed to the OS as an arbitrary URL.
pub(super) fn console_url(route: &str, query: Option<(&str, &str)>) -> String {
    let (path, params) = route.split_once('?').unwrap_or((route, ""));
    let path = match path {
        "/employees" | "/tasks" | "/monitoring" | "/usage" => path,
        _ => "/tasks",
    };
    let mut url = Url::parse(&format!("{CONSOLE}{path}")).expect("fixed console URL");
    url.set_query(Some(params));
    let incoming: Vec<_> = url.query_pairs().into_owned().collect();
    let mut pairs = url.query_pairs_mut();
    pairs.clear();
    if let Some((key, value)) = query {
        pairs.append_pair(key, value);
    } else {
        for (key, value) in incoming {
            match key.as_ref() {
                "task_id" | "task" => {
                    pairs.append_pair("task", &value);
                }
                "phase" | "status" | "alert_id" => {
                    pairs.append_pair(&key, &value);
                }
                _ => {}
            }
        }
    }
    drop(pairs);
    if url.query() == Some("") {
        url.set_query(None);
    }
    url.to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Summary {
    pub(super) kpi: Kpi,
    #[serde(default, deserialize_with = "nullable_list")]
    pub(super) todos: Vec<Todo>,
    #[serde(default, deserialize_with = "nullable_list")]
    pub(super) decision_inbox: Vec<Decision>,
    pub(super) roster: Option<Roster>,
    pub(super) trend: Option<Trend>,
    pub(super) scope: Option<Scope>,
    #[serde(default, deserialize_with = "nullable_list")]
    pub(super) sources: Vec<Source>,
    #[serde(default, deserialize_with = "nullable_list")]
    pub(super) recent_activity: Vec<Activity>,
    #[serde(default)]
    pub(super) partial: bool,
    pub(super) data_as_of: Option<String>,
}

impl Summary {
    pub(super) fn other_todos(&self) -> impl Iterator<Item = &Todo> {
        self.todos.iter().filter(|todo| {
            todo.task_id
                .as_ref()
                .is_none_or(|id| !self.decision_inbox.iter().any(|item| &item.task_id == id))
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct Kpi {
    pub(super) employees_total: Option<u64>,
    pub(super) employees_active: Option<u64>,
    pub(super) tasks_total: Option<u64>,
    pub(super) tasks_running: Option<u64>,
    pub(super) pending_human_total: Option<u64>,
    pub(super) approvals_pending: Option<u64>,
    pub(super) tasks_success_rate: Option<f64>,
    pub(super) tasks_completed_today: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct Scope {
    pub(super) visible_hint: Option<String>,
    pub(super) role_hint: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct Source {
    pub(super) name: String,
    pub(super) status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct Todo {
    #[serde(rename = "type")]
    pub(super) kind: String,
    pub(super) title: String,
    pub(super) href: String,
    pub(super) task_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct Decision {
    pub(super) task_id: String,
    pub(super) task_name: String,
    pub(super) waiting_type: String,
    pub(super) blocked_step: String,
    pub(super) decision_summary: String,
    pub(super) impact_hint: String,
    pub(super) waiting_duration_ms: u64,
    pub(super) sla_breached: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct Roster {
    pub(super) working_total: u64,
    pub(super) available_total: u64,
    #[serde(default, deserialize_with = "nullable_list")]
    pub(super) rows: Vec<Employee>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct Employee {
    pub(super) employee_id: String,
    pub(super) name: String,
    pub(super) agent_kind: String,
    pub(super) state: String,
    pub(super) department_path: Option<String>,
    pub(super) runtime: Runtime,
    pub(super) availability: Option<String>,
    pub(super) can_dispatch: bool,
    pub(super) current_work: Option<Work>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct Runtime {
    pub(super) status: String,
    pub(super) label: String,
    pub(super) cause: Option<String>,
    pub(super) next_action: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct Work {
    pub(super) task_id: String,
    pub(super) task_name: String,
    pub(super) progress_pct: f32,
    pub(super) current_step: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct Trend {
    #[serde(default, deserialize_with = "nullable_list")]
    pub(super) days: Vec<String>,
    #[serde(default, deserialize_with = "nullable_list")]
    pub(super) tasks: Vec<Option<u64>>,
    #[serde(default, deserialize_with = "nullable_list")]
    pub(super) success_rate: Vec<Option<f64>>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct Activity {
    pub(super) id: String,
    pub(super) summary: Option<String>,
    pub(super) current_step: Option<String>,
    pub(super) kind: Option<String>,
    pub(super) at: Option<String>,
}

pub(super) fn nullable_list<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<Vec<T>>::deserialize(deserializer).map(Option::unwrap_or_default)
}
