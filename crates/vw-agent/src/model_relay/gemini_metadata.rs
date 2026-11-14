//! Native read/count actions, kept outside generation conversion and retries.
use axum::http::{Method, StatusCode, Uri};

pub(crate) struct Request {
    pub(crate) model: Option<String>,
    pub(crate) count: bool,
    query: Option<String>,
}

impl Request {
    pub(crate) fn parse(uri: &Uri) -> Option<Self> {
        let path = uri.path().strip_prefix("/gemini").unwrap_or(uri.path());
        let resource = path.strip_prefix("/v1beta/models")
            .or_else(|| path.strip_prefix("/v1/models"))?;
        let (model, count) = if resource.is_empty() {
            (None, false)
        } else {
            let resource = resource.strip_prefix('/')?;
            let (model, count) = match resource.split_once(':') {
                Some((model, "countTokens")) => (model, true),
                Some(_) => return None,
                None => (resource, false),
            };
            if matches!(model, "" | "." | "..") || !model.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b)) {
                return None;
            }
            (Some(model.to_owned()), count)
        };
        if let Some(query) = uri.query() {
            // Client credentials and destination controls never pass through.
            if model.is_some() || query.is_empty() { return None; }
            let mut seen = std::collections::HashSet::new();
            for (key, value) in url::form_urlencoded::parse(query.as_bytes()) {
                if !seen.insert(key.to_string()) || value.is_empty() { return None; }
                match key.as_ref() {
                    "pageSize" if value.parse::<u32>().is_ok_and(|size| size > 0) => {},
                    "pageToken" => {},
                    _ => return None,
                }
            }
        }
        Some(Self { model, count, query: uri.query().map(str::to_owned) })
    }

    pub(crate) fn check_method(&self, method: &Method) -> Result<(), StatusCode> {
        if method == if self.count { Method::POST } else { Method::GET } {
            Ok(())
        } else {
            Err(StatusCode::METHOD_NOT_ALLOWED)
        }
    }

    pub(crate) fn url(&self, target: &reqwest::Url, model: Option<&str>) -> Result<reqwest::Url, StatusCode> {
        if model.or(self.model.as_deref()).is_some_and(|model| matches!(model.strip_prefix("models/").unwrap_or(model), "." | "..")) {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let mut url = super::transport::gemini_url(&crate::provider::provider::ApiInfo {
            protocols: None, url: target.to_string(),
            id: model.or(self.model.as_deref()).unwrap_or("metadata").into(),
            adapter: "openai-compatible".into(),
        }, false)?;
        let base = url.path().split_once("/models/").ok_or(StatusCode::UNPROCESSABLE_ENTITY)?.0.to_owned();
        let path = match model.or(self.model.as_deref()) {
            Some(model) => format!("{base}/models/{}{}", model.strip_prefix("models/").unwrap_or(model), if self.count { ":countTokens" } else { "" }),
            None => format!("{base}/models"),
        };
        url.set_path(&path);
        url.set_query(self.query.as_deref());
        Ok(url)
    }
}
