use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use super::{
    api::{Client, nullable_list},
    board_api,
};

pub(crate) struct Directory {
    pub(super) employees: Vec<SharedEmployee>,
    pub(super) templates: Option<usize>,
    pub(super) counts: Option<HashMap<String, TaskCounts>>,
}

#[derive(Deserialize)]
pub(super) struct SharedEmployee {
    pub employee_id: String,
    pub name: String,
    pub agent_kind: Option<String>,
    pub visibility: Option<String>,
    pub department_path: Option<String>,
    pub status: Option<String>,
    pub publish_status: Option<String>,
    pub runtime_status: Option<String>,
    pub runtime_status_label: Option<String>,
    pub runtime_cause: Option<String>,
    pub runtime_next_action: Option<String>,
    pub updated_at: Option<String>,
    pub role_title: Option<String>,
    pub role: Option<String>,
    #[serde(alias = "templateName")]
    pub template_name: Option<String>,
    #[serde(default, deserialize_with = "nullable_list")]
    pub tags: Vec<String>,
    pub manager_name: Option<String>,
    pub worker_count: Option<usize>,
    pub today: Option<usize>,
    pub today_executed: Option<usize>,
    pub occupancy: Option<Occupancy>,
}

#[derive(Deserialize)]
pub(super) struct Occupancy {
    pub busy: bool,
}

impl SharedEmployee {
    pub fn archived(&self) -> bool {
        self.status.as_deref().is_some_and(|status| status.eq_ignore_ascii_case("ARCHIVED"))
    }

    pub fn manager(&self) -> bool {
        self.agent_kind.as_deref() != Some("worker")
    }

    pub fn department(&self) -> Option<&str> {
        self.department_path.as_deref().map(str::trim).filter(|path| !path.is_empty())
    }

    pub fn role(&self) -> Option<&str> {
        [self.role_title.as_deref(), self.role.as_deref()]
            .into_iter()
            .flatten()
            .map(str::trim)
            .find(|role| !role.is_empty())
    }

    pub fn capability_tags(&self) -> impl Iterator<Item = &str> {
        self.tags
            .iter()
            .map(|tag| tag.trim())
            .filter(|tag| !tag.is_empty() && !tag.to_ascii_lowercase().starts_with("builtin:"))
    }

    pub fn matches(&self, query: &str) -> bool {
        query.is_empty()
            || std::iter::once(self.name.as_str())
                .chain(self.role())
                .chain(self.template_name.as_deref())
                .chain(self.capability_tags())
                .any(|value| value.to_lowercase().contains(query))
    }
}

#[derive(Deserialize)]
struct Page {
    #[serde(default, deserialize_with = "nullable_list")]
    employees: Vec<SharedEmployee>,
    total: Option<usize>,
}

#[derive(Deserialize)]
struct Templates {
    #[serde(default, deserialize_with = "nullable_list")]
    templates: Vec<Template>,
}

#[derive(Deserialize)]
struct Template {
    status: String,
}

#[derive(Default)]
pub(super) struct TaskCounts {
    pub today: usize,
    pub active: usize,
}

pub(super) async fn load(credential: &str) -> Result<Directory, String> {
    let client = Client::new(credential)?;
    let (employees, templates, counts) = tokio::join!(
        employees(&client),
        client.get::<Templates>("job-templates", &[]),
        task_counts(&client),
    );
    Ok(Directory {
        employees: employees?,
        templates: templates.ok().map(|page| {
            page.templates.iter().filter(|template| template.status == "active").count()
        }),
        counts: counts.ok(),
    })
}

async fn employees(client: &Client) -> Result<Vec<SharedEmployee>, String> {
    let mut all = Vec::new();
    let mut seen = HashSet::new();
    let mut offset = 0;
    loop {
        let offset_param = offset.to_string();
        let page: Page =
            client.get("employees", &[("limit", "100"), ("offset", &offset_param)]).await?;
        let length = page.employees.len();
        let previous = seen.len();
        for employee in page.employees {
            if seen.insert(employee.employee_id.clone())
                && employee.visibility.as_deref() != Some("private")
            {
                all.push(employee);
            }
        }
        offset += length;
        if length == 0 || offset >= page.total.unwrap_or(offset) {
            return Ok(all);
        }
        if seen.len() == previous {
            return Err("共享员工分页没有返回新记录，请刷新重试".into());
        }
    }
}

async fn task_counts(client: &Client) -> Result<HashMap<String, TaskCounts>, String> {
    let today = chrono::Local::now().date_naive();
    let mut counts: HashMap<String, TaskCounts> = HashMap::new();
    for task in board_api::load_tasks(client).await? {
        let Some(owner) = task.owner_ref else { continue };
        let count = counts.entry(owner).or_default();
        if task.phase.as_deref().is_some_and(|phase| phase != "ENDED") {
            count.active += 1;
        }
        if task.status.eq_ignore_ascii_case("SUCCEEDED")
            && task
                .updated_at
                .as_deref()
                .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                .is_some_and(|value| value.with_timezone(&chrono::Local).date_naive() == today)
        {
            count.today += 1;
        }
    }
    Ok(counts)
}
