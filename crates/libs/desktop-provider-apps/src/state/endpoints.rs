use vw_api_app_provider_edit::endpoints::*;
#[derive(Clone)]
pub struct Form {
    pub id: String,
    pub url: String,
    pub full_url: bool,
    pub headers: String,
    pub overrides: String,
    pub user_agent: Option<String>,
    pub options:
        Option<vw_api_app_provider_edit::request_options::RequestOptionsView>,
    pub rows: Vec<AppProviderEndpoint>,
    pub busy: bool,
    pub notice: Option<String>,
}
impl std::fmt::Debug for Form {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EndpointForm([redacted])")
    }
}
impl Form {
    pub fn new(id: String) -> Self {
        Self {
            id,
            url: String::new(),
            full_url: false,
            rows: Vec::new(),
            headers: String::new(),
            overrides: String::new(),
            user_agent: None,
            options: None,
            busy: false,
            notice: None,
        }
    }
    pub fn finish(&mut self, result: Result<Vec<AppProviderEndpoint>, String>) {
        self.busy = false;
        match result {
            Ok(rows) => self.rows = rows,
            Err(error) => self.notice = Some(error),
        }
    }
}
